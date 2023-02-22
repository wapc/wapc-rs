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
use wapc::{wapc_functions, ModuleState, WasiParams, WebAssemblyEngineProvider};
// export wasmtime and wasmtime_wasi, so that consumers of this crate can use
// the very same version
pub use wasmtime;
use wasmtime::{AsContextMut, Engine, Instance, InstancePre, Linker, Module, Store, TypedFunc};

cfg_if::cfg_if! {
    if #[cfg(feature = "wasi")] {
        pub use wasmtime_wasi;
        use wasmtime_wasi::WasiCtx;
    }
}

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
  host: Option<Arc<ModuleState>>,
}

impl WapcStore {
  fn new(wasi_params: &WasiParams, host: Option<Arc<ModuleState>>) -> Result<WapcStore> {
    cfg_if::cfg_if! {
      if #[cfg(feature = "wasi")] {

        let preopened_dirs = wasi::compute_preopen_dirs(
            &wasi_params.preopened_dirs,
            &wasi_params.map_dirs
        ).map_err(|e|
            errors::Error::WasiInitCtxError(format!("Cannot compute preopened dirs: {:?}", e)))?;
        let wasi_ctx = wasi::init_ctx(
            &preopened_dirs,
            &wasi_params.argv,
            &wasi_params.env_vars,
        ).map_err(|e| errors::Error::WasiInitCtxError(e.to_string()))?;

        Ok(WapcStore{
            wasi_ctx,
            host,
        })
      } else {
        if wasi.is_some() {
            // this check is required because otherwise the `wasi` parameter
            // would not be used when the feature `wasi` is not enabled.
            // That would cause a compilation error because we do not allow unused
            // code.
            Err(errors::Error::WasiDisabled);
        } else {
          Ok(WapcStore{
              wasi_ctx,
              host,
          })
        }
      }
    }
  }
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

/// A pre initialized WasmtimeEngineProvider
///
/// Can be used to quickly create a new instance of WasmtimeEngineProvider
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub(crate) struct WasmtimeEngineProviderPre {
  module: Module,
  wasi_params: WasiParams,
  engine: Engine,
  linker: Linker<WapcStore>,
  instance_pre: InstancePre<WapcStore>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl WasmtimeEngineProviderPre {
  fn new(engine: Engine, module: Module, wasi: Option<WasiParams>) -> Result<Self> {
    let mut linker: Linker<WapcStore> = Linker::new(&engine);

    let wasi_params = wasi.unwrap_or_default();

    cfg_if::cfg_if! {
      if #[cfg(feature = "wasi")] {
        wasmtime_wasi::add_to_linker(&mut linker, |s: &mut WapcStore| &mut s.wasi_ctx).unwrap();
      }
    };

    // register all the waPC host functions
    callbacks::add_to_linker(&mut linker)?;

    let instance_pre = linker.instantiate_pre(&module)?;

    Ok(Self {
      module,
      wasi_params,
      engine,
      linker,
      instance_pre,
      epoch_deadlines: None,
    })
  }

  /// Create an instance of [`WasmtimeEngineProvider`] ready to be consumed
  ///
  /// Note: from micro-benchmarking, this method is 10 microseconds faster than
  /// `WasmtimeEngineProvider::clone`. This isn't a significant gain to justify
  /// the exposure of this method to all the consumers of `wasmtime_provider`.
  pub(crate) fn rehydrate(&self) -> Result<WasmtimeEngineProvider> {
    let engine = self.engine.clone();

    let wapc_store = WapcStore::new(&self.wasi_params, None)?;
    let store = Store::new(&engine, wapc_store);

    Ok(WasmtimeEngineProvider {
      module: self.module.clone(),
      inner: None,
      engine,
      epoch_deadlines: self.epoch_deadlines,
      linker: self.linker.clone(),
      instance_pre: self.instance_pre.clone(),
      store,
      wasi_params: self.wasi_params.clone(),
    })
  }
}

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime
#[allow(missing_debug_implementations)]
pub struct WasmtimeEngineProvider {
  module: Module,
  wasi_params: WasiParams,
  inner: Option<EngineInner>,
  engine: Engine,
  linker: Linker<WapcStore>,
  store: Store<WapcStore>,
  instance_pre: InstancePre<WapcStore>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl Clone for WasmtimeEngineProvider {
  fn clone(&self) -> Self {
    let engine = self.engine.clone();

    let wapc_store = WapcStore::new(&self.wasi_params, None).unwrap();
    let store = Store::new(&engine, wapc_store);

    match &self.inner {
      Some(state) => {
        let mut new = Self {
          module: self.module.clone(),
          inner: None,
          engine,
          epoch_deadlines: self.epoch_deadlines,
          linker: self.linker.clone(),
          instance_pre: self.instance_pre.clone(),
          store,
          wasi_params: self.wasi_params.clone(),
        };
        new.init(state.host.clone()).unwrap();
        new
      }
      None => Self {
        module: self.module.clone(),
        inner: None,
        engine,
        epoch_deadlines: self.epoch_deadlines,
        linker: self.linker.clone(),
        instance_pre: self.instance_pre.clone(),
        store,
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
    let pre = WasmtimeEngineProviderPre::new(engine, module, wasi)?;
    pre.rehydrate()
  }
}

impl WebAssemblyEngineProvider for WasmtimeEngineProvider {
  fn init(
    &mut self,
    host: Arc<ModuleState>,
  ) -> std::result::Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>> {
    // create the proper store, now we have a value for `host`
    let wapc_store = WapcStore::new(&self.wasi_params, Some(host.clone()))?;
    self.store = Store::new(&self.engine, wapc_store);

    let instance = self.instance_pre.instantiate(&mut self.store)?;

    let instance_ref = Arc::new(RwLock::new(instance));
    let gc = guest_call_fn(&mut self.store, &instance_ref)?;
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
      Err(err) => {
        error!("Failure invoking guest module handler: {:?}", err);
        let mut guest_error = err.to_string();
        if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
          if matches!(trap, wasmtime::Trap::Interrupt) {
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

    let module = Module::new(&self.engine, module)?;
    self.instance_pre = self.linker.instantiate_pre(&module)?;
    let new_instance = self.instance_pre.instantiate(&mut self.store)?;
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
        starter_func.call(&mut self.store, ()).map_err(|err| {
          if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
            if matches!(trap, wasmtime::Trap::Interrupt) {
              Error::InitializationFailedTimeout((*starter).to_owned())
            } else {
              Error::InitializationFailed(err.into())
            }
          } else {
            Error::InitializationFailed(err.into())
          }
        })?;
      }
    }
    Ok(())
  }
}

// Called once, then the result is cached. This returns a `Func` that corresponds
// to the `__guest_call` export
fn guest_call_fn(store: impl AsContextMut, instance: &Arc<RwLock<Instance>>) -> Result<TypedFunc<(i32, i32), i32>> {
  instance
    .read()
    .get_typed_func::<(i32, i32), i32>(store, wapc_functions::GUEST_CALL)
    .map_err(|_| errors::Error::GuestCallNotFound)
}
