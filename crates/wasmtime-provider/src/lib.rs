#![deny(
  clippy::expect_used,
  clippy::explicit_deref_methods,
  clippy::option_if_let_else,
  clippy::await_holding_lock,
  clippy::cloned_instead_of_copied,
  clippy::explicit_into_iter_loop,
  clippy::flat_map_option,
  clippy::fn_params_excessive_bools,
  clippy::implicit_clone,
  clippy::inefficient_to_string,
  clippy::large_types_passed_by_value,
  clippy::manual_ok_or,
  clippy::map_flatten,
  clippy::map_unwrap_or,
  clippy::must_use_candidate,
  clippy::needless_for_each,
  clippy::needless_pass_by_value,
  clippy::option_option,
  clippy::redundant_else,
  clippy::semicolon_if_nothing_returned,
  clippy::too_many_lines,
  clippy::trivially_copy_pass_by_ref,
  clippy::unnested_or_patterns,
  clippy::future_not_send,
  clippy::useless_let_if_seq,
  clippy::str_to_string,
  clippy::inherent_to_string,
  clippy::let_and_return,
  clippy::string_to_string,
  clippy::try_err,
  clippy::unused_async,
  clippy::missing_enforced_import_renames,
  clippy::nonstandard_macro_braces,
  clippy::rc_mutex,
  clippy::unwrap_or_else_default,
  clippy::manual_split_once,
  clippy::derivable_impls,
  clippy::needless_option_as_deref,
  clippy::iter_not_returning_iterator,
  clippy::same_name_method,
  clippy::manual_assert,
  clippy::non_send_fields_in_send_ty,
  clippy::equatable_if_let,
  bad_style,
  clashing_extern_declarations,
  const_err,
  dead_code,
  deprecated,
  explicit_outlives_requirements,
  improper_ctypes,
  invalid_value,
  missing_copy_implementations,
  missing_debug_implementations,
  mutable_transmutes,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  private_in_public,
  trivial_bounds,
  trivial_casts,
  trivial_numeric_casts,
  type_alias_bounds,
  unconditional_recursion,
  unreachable_pub,
  unsafe_code,
  unstable_features,
  unused,
  unused_allocation,
  unused_comparisons,
  unused_import_braces,
  unused_parens,
  unused_qualifications,
  while_true,
  missing_docs
)]
#![doc = include_str!("../README.md")]

mod callbacks;
#[cfg(feature = "wasi")]
mod wasi;

/// The crate's error module
pub mod errors;
use errors::{Error, Result};

mod builder;
pub use builder::WasmtimeEngineProviderBuilder;
use parking_lot::RwLock;
use wapc::{wapc_functions, ModuleState, WasiParams, WebAssemblyEngineProvider, HOST_NAMESPACE};
// export wasmtime and wasmtime_wasi, so that consumers of this crate can use
// the very same version
pub use wasmtime;
use wasmtime::{AsContextMut, Engine, Extern, ExternType, Instance, Linker, Module, Store, TypedFunc};

cfg_if::cfg_if! {
    if #[cfg(feature = "wasi")] {
        pub use wasmtime_wasi;
        use wasmtime_wasi::WasiCtx;
    }
}

// namespace needed for some language support
const WASI_UNSTABLE_NAMESPACE: &str = "wasi_unstable";
const WASI_SNAPSHOT_PREVIEW1_NAMESPACE: &str = "wasi_snapshot_preview1";

use std::sync::Arc;

#[macro_use]
extern crate log;

struct EngineInner {
  instance: Arc<RwLock<Instance>>,
  guest_call_fn: TypedFunc<(i32, i32), i32>,
  host: Arc<ModuleState>,
}

struct WapcStore {
  #[cfg(feature = "wasi")]
  wasi_ctx: WasiCtx,
}

/// Configure behavior of wasmtime [epoch-based interruptions](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption)
///
/// There are two kind of deadlines that apply to waPC modules:
///
/// * waPC initialization code: this is the code defined by the module inside
///   of the `wapc_init` or the `_start` functions
/// * user function: the actual waPC guest function written by an user
#[derive(Clone, Copy, Debug)]
struct EpochDeadlines {
  /// Deadline for waPC initialization code. Expressed in number of epoch ticks
  wapc_init: u64,

  /// Deadline for user-defined waPC function computation. Expressed in number of epoch ticks
  wapc_func: u64,
}

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime
#[allow(missing_debug_implementations)]
pub struct WasmtimeEngineProvider {
  module: Module,
  #[cfg(feature = "wasi")]
  wasi_params: WasiParams,
  inner: Option<EngineInner>,
  store: Store<WapcStore>,
  engine: Engine,
  linker: Linker<WapcStore>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl Clone for WasmtimeEngineProvider {
  fn clone(&self) -> Self {
    let engine = self.engine.clone();
    cfg_if::cfg_if! {
      if #[cfg(feature = "wasi")] {
        let wasi_ctx = init_wasi(&self.wasi_params).unwrap();
        let store = Store::new(&engine, WapcStore { wasi_ctx });
      } else {
        let store = Store::new(&engine, WapcStore {});
      }
    };

    match &self.inner {
      Some(state) => {
        let mut new = Self {
          module: self.module.clone(),
          inner: None,
          store,
          engine,
          epoch_deadlines: self.epoch_deadlines,
          linker: self.linker.clone(),
          #[cfg(feature = "wasi")]
          wasi_params: self.wasi_params.clone(),
        };
        new.init(state.host.clone()).unwrap();
        new
      }
      None => Self {
        module: self.module.clone(),
        inner: None,
        store,
        engine,
        epoch_deadlines: self.epoch_deadlines,
        linker: self.linker.clone(),
        #[cfg(feature = "wasi")]
        wasi_params: self.wasi_params.clone(),
      },
    }
  }
}

impl WasmtimeEngineProvider {
  /// Creates a new instance of a [WasmtimeEngineProvider].
  #[deprecated(
    since = "1.2.0",
    note = "please use `WasmtimeEngineProviderBuilder` instead to create a `WasmtimeEngineProvider`"
  )]
  #[allow(deprecated)]
  pub fn new(buf: &[u8], wasi: Option<WasiParams>) -> Result<WasmtimeEngineProvider> {
    let engine = Engine::default();
    Self::new_with_engine(buf, engine, wasi)
  }

  #[cfg(feature = "cache")]
  #[allow(deprecated)]
  /// Creates a new instance of a [WasmtimeEngineProvider] with caching enabled.
  #[deprecated(
    since = "1.2.0",
    note = "please use `WasmtimeEngineProviderBuilder` instead to create a `WasmtimeEngineProvider`"
  )]
  pub fn new_with_cache(
    buf: &[u8],
    wasi: Option<WasiParams>,
    cache_path: Option<&std::path::Path>,
  ) -> Result<WasmtimeEngineProvider> {
    let mut config = wasmtime::Config::new();
    config.strategy(wasmtime::Strategy::Cranelift);
    if let Some(cache) = cache_path {
      config.cache_config_load(cache)
    } else {
      config.cache_config_load_default()
    }?;
    let engine = Engine::new(&config)?;
    Self::new_with_engine(buf, engine, wasi)
  }

  /// Creates a new instance of a [WasmtimeEngineProvider] from a separately created [wasmtime::Engine].
  #[deprecated(
    since = "1.2.0",
    note = "please use `WasmtimeEngineProviderBuilder` instead to create a `WasmtimeEngineProvider`"
  )]
  pub fn new_with_engine(buf: &[u8], engine: Engine, wasi: Option<WasiParams>) -> Result<Self> {
    let module = Module::new(&engine, buf)?;

    cfg_if::cfg_if! {
      if #[cfg(feature = "wasi")] {
        let mut linker: Linker<WapcStore> = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| &mut s.wasi_ctx).unwrap();
        let wasi_params = wasi.unwrap_or_default();
        let wasi_ctx = wasi::init_ctx(
            &wasi::compute_preopen_dirs(&wasi_params.preopened_dirs, &wasi_params.map_dirs)
                .unwrap(),
            &wasi_params.argv,
            &wasi_params.env_vars,
        )
        .unwrap();
        let store = Store::new(&engine, WapcStore { wasi_ctx });
      } else {
        if wasi.is_some() {
            // this check is required because otherwise the `wasi` parameter
            // would not be used when the feature `wasi` is not enabled.
            // That would cause a compilation error because we do not allow unused
            // code.
            return Err(errors::Error::WasiDisabled);
        }
        let linker: Linker<WapcStore> = Linker::new(&engine);
        let store = Store::new(&engine, WapcStore {});
      }
    };

    Ok(WasmtimeEngineProvider {
      module,
      #[cfg(feature = "wasi")]
      wasi_params,
      inner: None,
      store,
      engine,
      linker,
      epoch_deadlines: None,
    })
  }
}

impl WebAssemblyEngineProvider for WasmtimeEngineProvider {
  fn init(
    &mut self,
    host: Arc<ModuleState>,
  ) -> std::result::Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>> {
    let instance = instance_from_module(&mut self.store, &self.module, &host, &self.linker)?;
    let instance_ref = Arc::new(RwLock::new(instance));
    let gc = guest_call_fn(self.store.as_context_mut(), &instance_ref)?;
    self.inner = Some(EngineInner {
      instance: instance_ref,
      guest_call_fn: gc,
      host,
    });
    self.initialize()?;
    Ok(())
  }

  fn call(
    &mut self,
    op_length: i32,
    msg_length: i32,
  ) -> std::result::Result<i32, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
    if let Some(deadlines) = &self.epoch_deadlines {
      // the deadline counter must be set before invoking the wasm function
      self.store.set_epoch_deadline(deadlines.wapc_func);
    }

    let engine_inner = self.inner.as_ref().unwrap();
    let call = engine_inner
      .guest_call_fn
      .call(&mut self.store, (op_length, msg_length));

    match call {
      Ok(result) => Ok(result),
      Err(trap) => {
        error!("Failure invoking guest module handler: {:?}", trap);
        let mut guest_error = trap.to_string();
        if let Some(trap_code) = trap.trap_code() {
          if matches!(trap_code, wasmtime::TrapCode::Interrupt) {
            guest_error = "guest code interrupted, execution deadline exceeded".to_owned();
          }
        }
        engine_inner.host.set_guest_error(guest_error);
        Ok(0)
      }
    }
  }

  fn replace(
    &mut self,
    module: &[u8],
  ) -> std::result::Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>> {
    info!(
      "HOT SWAP - Replacing existing WebAssembly module with new buffer, {} bytes",
      module.len()
    );

    let new_instance = instance_from_buffer(
      &mut self.store,
      &self.engine,
      module,
      &self.inner.as_ref().unwrap().host,
      &self.linker,
    )?;
    *self.inner.as_ref().unwrap().instance.write() = new_instance;

    Ok(self.initialize()?)
  }
}

impl WasmtimeEngineProvider {
  fn initialize(&mut self) -> Result<()> {
    for starter in wapc::wapc_functions::REQUIRED_STARTS.iter() {
      if let Some(deadlines) = &self.epoch_deadlines {
        // the deadline counter must be set before invoking the wasm function
        self.store.set_epoch_deadline(deadlines.wapc_init);
      }

      let engine_inner = self.inner.as_ref().unwrap();
      if engine_inner
        .instance
        .read()
        .get_export(&mut self.store, starter)
        .is_some()
      {
        // Need to get a `wasmtime::TypedFunc` because its `call` method
        // can return a Trap error. Non-typed functions instead return a
        // generic `anyhow::Error` that doesn't allow nice handling of
        // errors
        let starter_func: TypedFunc<(), ()> = engine_inner.instance.read().get_typed_func(&mut self.store, starter)?;
        starter_func.call(&mut self.store, ()).map_err(|trap| {
          if let Some(trap_code) = trap.trap_code() {
            if matches!(trap_code, wasmtime::TrapCode::Interrupt) {
              Error::InitializationFailedTimeout((*starter).to_owned())
            } else {
              Error::InitializationFailed(trap.into())
            }
          } else {
            Error::InitializationFailed(trap.into())
          }
        })?;
      }
    }
    Ok(())
  }
}

fn instance_from_buffer(
  store: &mut Store<WapcStore>,
  engine: &Engine,
  buf: &[u8],
  state: &Arc<ModuleState>,
  linker: &Linker<WapcStore>,
) -> Result<Instance> {
  let module = Module::new(engine, buf).unwrap();
  let imports = arrange_imports(&module, state, store, linker);
  Ok(wasmtime::Instance::new(store.as_context_mut(), &module, imports?.as_slice()).unwrap())
}

fn instance_from_module(
  store: &mut Store<WapcStore>,
  module: &Module,
  state: &Arc<ModuleState>,
  linker: &Linker<WapcStore>,
) -> Result<Instance> {
  let imports = arrange_imports(module, state, store, linker);
  Ok(wasmtime::Instance::new(store.as_context_mut(), module, imports?.as_slice()).unwrap())
}

#[cfg(feature = "wasi")]
fn init_wasi(params: &WasiParams) -> Result<WasiCtx> {
  wasi::init_ctx(
    &wasi::compute_preopen_dirs(&params.preopened_dirs, &params.map_dirs).unwrap(),
    &params.argv,
    &params.env_vars,
  )
  .map_err(|e| errors::Error::InitializationFailed(e))
}

/// wasmtime requires that the list of callbacks be "zippable" with the list
/// of module imports. In order to ensure that both lists are in the same
/// order, we have to loop through the module imports and instantiate the
/// corresponding callback. We **cannot** rely on a predictable import order
/// in the wasm module
#[allow(clippy::unnecessary_wraps)]
fn arrange_imports(
  module: &Module,
  host: &Arc<ModuleState>,
  store: &mut impl AsContextMut<Data = WapcStore>,
  linker: &Linker<WapcStore>,
) -> Result<Vec<Extern>> {
  Ok(
    module
      .imports()
      .filter_map(|imp| {
        if let ExternType::Func(_) = imp.ty() {
          match imp.module() {
            HOST_NAMESPACE => Some(callback_for_import(store.as_context_mut(), imp.name(), host.clone())),
            WASI_SNAPSHOT_PREVIEW1_NAMESPACE | WASI_UNSTABLE_NAMESPACE => {
              linker.get_by_import(store.as_context_mut(), &imp)
            }
            other => panic!("import module `{}` was not found", other), //TODO: get rid of panic
          }
        } else {
          None
        }
      })
      .collect(),
  )
}

fn callback_for_import(store: impl AsContextMut, import: &str, host: Arc<ModuleState>) -> Extern {
  match import {
    wapc_functions::HOST_CONSOLE_LOG => callbacks::console_log_func(store, host).into(),
    wapc_functions::HOST_CALL => callbacks::host_call_func(store, host).into(),
    wapc_functions::GUEST_REQUEST_FN => callbacks::guest_request_func(store, host).into(),
    wapc_functions::HOST_RESPONSE_FN => callbacks::host_response_func(store, host).into(),
    wapc_functions::HOST_RESPONSE_LEN_FN => callbacks::host_response_len_func(store, host).into(),
    wapc_functions::GUEST_RESPONSE_FN => callbacks::guest_response_func(store, host).into(),
    wapc_functions::GUEST_ERROR_FN => callbacks::guest_error_func(store, host).into(),
    wapc_functions::HOST_ERROR_FN => callbacks::host_error_func(store, host).into(),
    wapc_functions::HOST_ERROR_LEN_FN => callbacks::host_error_len_func(store, host).into(),
    _ => unreachable!(),
  }
}

// Called once, then the result is cached. This returns a `Func` that corresponds
// to the `__guest_call` export
fn guest_call_fn(store: impl AsContextMut, instance: &Arc<RwLock<Instance>>) -> Result<TypedFunc<(i32, i32), i32>> {
  instance
    .read()
    .get_typed_func::<(i32, i32), i32, _>(store, wapc_functions::GUEST_CALL)
    .map_err(|_| errors::Error::GuestCallNotFound)
}
