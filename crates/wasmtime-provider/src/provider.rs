use std::sync::Arc;

use log::{error, info};
use parking_lot::RwLock;
use tracing::trace;
#[cfg(feature = "wasi")]
use wapc::WasiParams;
use wapc::{wapc_functions, ModuleState, WebAssemblyEngineProvider};
use wasmtime::{AsContextMut, Engine, Instance, InstancePre, Linker, Module, Store, TypedFunc};

use crate::errors::{Error, Result};
use crate::store::WapcStore;
use crate::{callbacks, EpochDeadlines};

struct EngineInner {
  instance: Arc<RwLock<Instance>>,
  guest_call_fn: TypedFunc<(i32, i32), i32>,
  host: Arc<ModuleState>,
}

/// A pre initialized WasmtimeEngineProvider
///
/// Can be used to quickly create a new instance of WasmtimeEngineProvider
///
/// Refer to [`WasmtimeEngineProviderBuilder::build_pre`](crate::WasmtimeEngineProviderBuilder::build_pre) to create an instance of this struct.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct WasmtimeEngineProviderPre {
  module: Module,
  #[cfg(feature = "wasi")]
  wasi_params: WasiParams,
  engine: Engine,
  linker: Linker<WapcStore>,
  instance_pre: InstancePre<WapcStore>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl WasmtimeEngineProviderPre {
  #[cfg(feature = "wasi")]
  pub(crate) fn new(
    engine: Engine,
    module: Module,
    wasi: Option<WasiParams>,
    epoch_deadlines: Option<EpochDeadlines>,
  ) -> Result<Self> {
    let mut linker: Linker<WapcStore> = Linker::new(&engine);

    let wasi_params = wasi.unwrap_or_default();
    wasi_common::sync::add_to_linker(&mut linker, |s: &mut WapcStore| &mut s.wasi_ctx).unwrap();

    // register all the waPC host functions
    callbacks::add_to_linker(&mut linker)?;

    let instance_pre = linker.instantiate_pre(&module)?;

    Ok(Self {
      module,
      wasi_params,
      engine,
      linker,
      instance_pre,
      epoch_deadlines,
    })
  }

  #[cfg(not(feature = "wasi"))]
  pub(crate) fn new(engine: Engine, module: Module, epoch_deadlines: Option<EpochDeadlines>) -> Result<Self> {
    let mut linker: Linker<WapcStore> = Linker::new(&engine);

    // register all the waPC host functions
    callbacks::add_to_linker(&mut linker)?;

    let instance_pre = linker.instantiate_pre(&module)?;

    Ok(Self {
      module,
      engine,
      linker,
      instance_pre,
      epoch_deadlines,
    })
  }

  /// Create an instance of [`WasmtimeEngineProvider`] ready to be consumed
  ///
  /// Note: from micro-benchmarking, this method is 10 microseconds faster than
  /// `WasmtimeEngineProvider::clone`.
  pub fn rehydrate(&self) -> Result<WasmtimeEngineProvider> {
    let engine = self.engine.clone();

    #[cfg(feature = "wasi")]
    let wapc_store = WapcStore::new(&self.wasi_params, None)?;
    #[cfg(not(feature = "wasi"))]
    let wapc_store = WapcStore::new(None);

    let store = Store::new(&engine, wapc_store);

    Ok(WasmtimeEngineProvider {
      module: self.module.clone(),
      inner: None,
      engine,
      epoch_deadlines: self.epoch_deadlines,
      linker: self.linker.clone(),
      instance_pre: self.instance_pre.clone(),
      store,
      #[cfg(feature = "wasi")]
      wasi_params: self.wasi_params.clone(),
    })
  }
}

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime
#[allow(missing_debug_implementations)]
pub struct WasmtimeEngineProvider {
  module: Module,
  #[cfg(feature = "wasi")]
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

    #[cfg(feature = "wasi")]
    let wapc_store = WapcStore::new(&self.wasi_params, None).unwrap();
    #[cfg(not(feature = "wasi"))]
    let wapc_store = WapcStore::new(None);

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
          #[cfg(feature = "wasi")]
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
        #[cfg(feature = "wasi")]
        wasi_params: self.wasi_params.clone(),
      },
    }
  }
}

impl WebAssemblyEngineProvider for WasmtimeEngineProvider {
  fn init(
    &mut self,
    host: Arc<ModuleState>,
  ) -> std::result::Result<(), Box<(dyn std::error::Error + Send + Sync + 'static)>> {
    // create the proper store, now we have a value for `host`
    #[cfg(feature = "wasi")]
    let wapc_store = WapcStore::new(&self.wasi_params, Some(host.clone()))?;
    #[cfg(not(feature = "wasi"))]
    let wapc_store = WapcStore::new(Some(host.clone()));

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
        error!("Failure invoking guest module handler: {err:?}");
        let mut guest_error = err.to_string();
        if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
          if matches!(trap, wasmtime::Trap::Interrupt) {
            "guest code interrupted, execution deadline exceeded".clone_into(&mut guest_error);
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
    self.module = module;
    self.instance_pre = self.linker.instantiate_pre(&self.module)?;
    let new_instance = self.instance_pre.instantiate(&mut self.store)?;
    if let Some(inner) = self.inner.as_mut() {
      *inner.instance.write() = new_instance;
      let gc = guest_call_fn(&mut self.store, &inner.instance)?;
      inner.guest_call_fn = gc;
    }

    Ok(self.initialize()?)
  }
}

impl WasmtimeEngineProvider {
  fn initialize(&mut self) -> Result<()> {
    for starter in wapc_functions::REQUIRED_STARTS.iter() {
      trace!(function = starter, "calling init function");
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

        if let Err(err) = starter_func.call(&mut self.store, ()) {
          trace!(function = starter, ?err, "handling error returned by init function");
          if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
            if matches!(trap, wasmtime::Trap::Interrupt) {
              return Err(Error::InitializationFailedTimeout((*starter).to_owned()));
            }
            return Err(Error::InitializationFailed(err.to_string()));
          }

          // WASI programs built by tinygo have to be written with a `main` function, even if it's empty.
          // Starting from tinygo >= 0.35.0, the `main` function calls the WASI process exit function,
          // which is handled by wasmtime as an Error.
          //
          // We must check if this error can be converted into a WASI exit
          // error and, if the exit code is 0, we can ignore it. Otherwise the waPC initialization
          // will fail.
          #[cfg(feature = "wasi")]
          if let Some(exit_err) = err.downcast_ref::<wasi_common::I32Exit>() {
            if exit_err.0 != 0 {
              return Err(Error::InitializationFailed(err.to_string()));
            }
            trace!("ignoring successful exit trap generated by WASI");
            continue;
          }

          return Err(Error::InitializationFailed(err.to_string()));
        };
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
    .map_err(|_| Error::GuestCallNotFound)
}
