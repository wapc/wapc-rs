use std::sync::Arc;

use async_trait::async_trait;
use log::{error, info};
use parking_lot::RwLock;
use tracing::trace;
#[cfg(feature = "wasi")]
use wapc::WasiParams;
use wapc::{wapc_functions, ModuleStateAsync, WebAssemblyEngineProviderAsync};
use wasmtime::{AsContextMut, Engine, Instance, InstancePre, Linker, Module, Store, TypedFunc};

use crate::errors::{Error, Result};
use crate::store_async::WapcStoreAsync;
use crate::{callbacks_async, EpochDeadlines};

struct EngineInner {
  instance: Arc<RwLock<Instance>>,
  guest_call_fn: TypedFunc<(i32, i32), i32>,
  host: Arc<ModuleStateAsync>,
}

/// A pre initialized [`WasmtimeEngineProviderAsync`]
///
/// Can be used to quickly create a new instance of [`WasmtimeEngineProviderAsync`]
///
/// Refer to [`WasmtimeEngineProviderBuilder::build_async_pre`](crate::WasmtimeEngineProviderBuilder::build_async_pre) to create an instance of this struct.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct WasmtimeEngineProviderAsyncPre {
  module: Module,
  #[cfg(feature = "wasi")]
  wasi_params: WasiParams,
  engine: Engine,
  linker: Linker<WapcStoreAsync>,
  instance_pre: InstancePre<WapcStoreAsync>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl WasmtimeEngineProviderAsyncPre {
  #[cfg(feature = "wasi")]
  pub(crate) fn new(
    engine: Engine,
    module: Module,
    wasi: Option<WasiParams>,
    epoch_deadlines: Option<EpochDeadlines>,
  ) -> Result<Self> {
    let mut linker: Linker<WapcStoreAsync> = Linker::new(&engine);

    let wasi_params = wasi.unwrap_or_default();
    wasi_common::tokio::add_to_linker(&mut linker, |s: &mut WapcStoreAsync| &mut s.wasi_ctx).unwrap();

    // register all the waPC host functions
    callbacks_async::add_to_linker(&mut linker)?;

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
    let mut linker: Linker<WapcStoreAsync> = Linker::new(&engine);

    // register all the waPC host functions
    callbacks_async::add_to_linker(&mut linker)?;

    let instance_pre = linker.instantiate_pre(&module)?;

    Ok(Self {
      module,
      engine,
      linker,
      instance_pre,
      epoch_deadlines,
    })
  }

  /// Create an instance of [`WasmtimeEngineProviderAsync`] ready to be consumed
  ///
  /// Note: from micro-benchmarking, this method is 10 microseconds faster than
  /// `WasmtimeEngineProviderAsync::clone`.
  pub fn rehydrate(&self) -> Result<WasmtimeEngineProviderAsync> {
    let engine = self.engine.clone();

    #[cfg(feature = "wasi")]
    let wapc_store = WapcStoreAsync::new(&self.wasi_params, None)?;
    #[cfg(not(feature = "wasi"))]
    let wapc_store = WapcStoreAsync::new(None);

    let store = Store::new(&engine, wapc_store);

    Ok(WasmtimeEngineProviderAsync {
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

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime.
/// This can be used inside of async contexts.
///
/// Refer to
/// [`WasmtimeEngineProviderBuilder::build_async`](crate::WasmtimeEngineProviderBuilder::build_async) to create an instance of this struct.
///
/// ## Example
///
/// ```rust
/// use wasmtime_provider::WasmtimeEngineProviderBuilder;
/// use wapc::WapcHostAsync;
/// use std::error::Error;
///
/// // Sample host callback that prints the operation a WASM module requested.
/// async fn host_callback(
///   id: u64,
///   bd: String,
///   ns: String,
///   op: String,
///   payload: Vec<u8>,
/// ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
///   println!(
///     "Guest {} invoked '{}->{}:{}' on the host with a payload of '{}'",
///     id,
///     bd,
///     ns,
///     op,
///     ::std::str::from_utf8(&payload).unwrap()
///   );
///   Ok(vec![])
/// }
///
/// #[tokio::main]
/// pub async fn main() -> Result<(), Box<dyn Error>> {
///   let callback: Box<wapc::HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
///     let fut = host_callback(id, bd, ns, op, payload);
///     Box::pin(fut)
///   });
///
///   let file = "../../wasm/crates/wasm-basic/build/wasm_basic.wasm";
///   let module_bytes = std::fs::read(file)?;
///
///   let engine = WasmtimeEngineProviderBuilder::new()
///     .module_bytes(&module_bytes)
///     .build_async()?;
///   let host = WapcHostAsync::new(Box::new(engine), Some(callback)).await?;
///
///   let res = host.call("ping", b"payload bytes").await?;
///   assert_eq!(res, b"payload bytes");
///
///   Ok(())
/// }
/// ```
#[allow(missing_debug_implementations)]
pub struct WasmtimeEngineProviderAsync {
  module: Module,
  #[cfg(feature = "wasi")]
  wasi_params: WasiParams,
  inner: Option<EngineInner>,
  engine: Engine,
  linker: Linker<WapcStoreAsync>,
  store: Store<WapcStoreAsync>,
  instance_pre: InstancePre<WapcStoreAsync>,
  epoch_deadlines: Option<EpochDeadlines>,
}

impl Clone for WasmtimeEngineProviderAsync {
  fn clone(&self) -> Self {
    let engine = self.engine.clone();

    #[cfg(feature = "wasi")]
    let wapc_store = WapcStoreAsync::new(&self.wasi_params, None).unwrap();
    #[cfg(not(feature = "wasi"))]
    let wapc_store = WapcStoreAsync::new(None);

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

        tokio::runtime::Handle::current().block_on(async {
          new.init(state.host.clone()).await.unwrap();
        });

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

#[async_trait]
impl WebAssemblyEngineProviderAsync for WasmtimeEngineProviderAsync {
  async fn init(
    &mut self,
    host: Arc<ModuleStateAsync>,
  ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // create the proper store, now we have a value for `host`
    #[cfg(feature = "wasi")]
    let wapc_store = WapcStoreAsync::new(&self.wasi_params, Some(host.clone()))?;
    #[cfg(not(feature = "wasi"))]
    let wapc_store = WapcStoreAsync::new(Some(host.clone()));

    self.store = Store::new(&self.engine, wapc_store);

    let instance = self.instance_pre.instantiate_async(&mut self.store).await?;

    let instance_ref = Arc::new(RwLock::new(instance));
    let gc = guest_call_fn(&mut self.store, &instance_ref)?;
    self.inner = Some(EngineInner {
      instance: instance_ref,
      guest_call_fn: gc,
      host,
    });
    self.initialize().await?;
    Ok(())
  }

  async fn call(
    &mut self,
    op_length: i32,
    msg_length: i32,
  ) -> std::result::Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(deadlines) = &self.epoch_deadlines {
      // the deadline counter must be set before invoking the wasm function
      self.store.set_epoch_deadline(deadlines.wapc_func);
    }

    let engine_inner = self.inner.as_ref().unwrap();
    let call = engine_inner
      .guest_call_fn
      .call_async(&mut self.store, (op_length, msg_length))
      .await;

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
        engine_inner.host.set_guest_error(guest_error).await;
        Ok(0)
      }
    }
  }

  async fn replace(&mut self, module: &[u8]) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
      "HOT SWAP - Replacing existing WebAssembly module with new buffer, {} bytes",
      module.len()
    );

    let module = Module::new(&self.engine, module)?;
    self.module = module;
    self.instance_pre = self.linker.instantiate_pre(&self.module)?;
    let new_instance = self.instance_pre.instantiate_async(&mut self.store).await?;
    if let Some(inner) = self.inner.as_mut() {
      *inner.instance.write() = new_instance;
      let gc = guest_call_fn(&mut self.store, &inner.instance)?;
      inner.guest_call_fn = gc;
    }

    Ok(self.initialize().await?)
  }
}

impl WasmtimeEngineProviderAsync {
  async fn initialize(&mut self) -> Result<()> {
    for starter in wapc_functions::REQUIRED_STARTS.iter() {
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

        if let Err(err) = starter_func.call_async(&mut self.store, ()).await {
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
