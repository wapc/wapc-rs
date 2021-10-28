#[cfg(feature = "wasi")]
use std::{error::Error, path::Path};
use wapc::{ModuleState, WapcFunctions, WasiParams, WebAssemblyEngineProvider, HOST_NAMESPACE};
use wasmtime::{
    AsContextMut, Config, Engine, Extern, ExternType, Func, Instance, Linker, Module, Store,
};
use wasmtime_wasi::WasiCtx;

// namespace needed for some language support
const WASI_UNSTABLE_NAMESPACE: &str = "wasi_unstable";
const WASI_SNAPSHOT_PREVIEW1_NAMESPACE: &str = "wasi_snapshot_preview1";

use std::sync::{Arc, RwLock};

#[macro_use]
extern crate log;

mod callbacks;
#[cfg(feature = "wasi")]
mod wasi;

struct EngineInner {
    instance: Arc<RwLock<Instance>>,
    guest_call_fn: Func,
    host: Arc<ModuleState>,
}

struct WapcStore {
    #[cfg(feature = "wasi")]
    wasi_ctx: WasiCtx,
}

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime
pub struct WasmtimeEngineProvider {
    module: Module,
    #[cfg(feature = "wasi")]
    wasi_params: WasiParams,
    inner: Option<EngineInner>,
    store: Store<WapcStore>,
    engine: Engine,
    linker: Linker<WapcStore>,
}

impl Clone for WasmtimeEngineProvider {
    fn clone(&self) -> Self {
        let wasi_ctx = init_wasi(&self.wasi_params).unwrap();
        let store = Store::new(&self.engine, WapcStore { wasi_ctx });
        match &self.inner {
            Some(state) => {
                let mut new = Self {
                    module: self.module.clone(),
                    inner: None,
                    store,
                    engine: self.engine.clone(),
                    linker: self.linker.clone(),
                    wasi_params: self.wasi_params.clone(),
                };
                new.init(state.host.clone()).unwrap();
                new
            }
            None => Self {
                module: self.module.clone(),
                inner: None,
                store,
                engine: self.engine.clone(),
                linker: self.linker.clone(),
                wasi_params: self.wasi_params.clone(),
            },
        }
    }
}

impl WasmtimeEngineProvider {
    /// Creates a new instance of a [WasmtimeEngineProvider].
    pub fn new(buf: &[u8], wasi: Option<WasiParams>) -> anyhow::Result<WasmtimeEngineProvider> {
        let engine = Engine::default();
        Self::new_with_engine(buf, engine, wasi)
    }

    #[cfg(feature = "cache")]
    /// Creates a new instance of a [WasmtimeEngineProvider] with caching enabled.
    pub fn new_with_cache(
        buf: &[u8],
        wasi: Option<WasiParams>,
        cache_path: Option<&std::path::Path>,
    ) -> anyhow::Result<WasmtimeEngineProvider> {
        let mut config = wasmtime::Config::new();
        config.strategy(wasmtime::Strategy::Cranelift)?;
        if let Some(cache) = cache_path {
            config.cache_config_load(cache)?;
        } else if let Err(e) = config.cache_config_load_default() {
            warn!("Wasmtime cache configuration not found ({}). Repeated loads will speed up significantly with a cache configuration. See https://docs.wasmtime.dev/cli-cache.html for more information.",e);
        }
        let engine = Engine::new(&config)?;
        Self::new_with_engine(buf, engine, wasi)
    }

    /// Creates a new instance of a [WasmtimeEngineProvider] from a separately created [wasmtime::Engine].
    pub fn new_with_engine(
        buf: &[u8],
        engine: Engine,
        wasi: Option<WasiParams>,
    ) -> anyhow::Result<Self> {
        let mut linker: Linker<WapcStore> = Linker::new(&engine);
        let module = Module::new(&engine, buf)?;

        cfg_if::cfg_if! {
          if #[cfg(feature = "wasi")] {
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
            let store = Store::new(&engine, WapcStore {});
          }
        };

        let mut linker: Linker<WapcStore> = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| &mut s.wasi_ctx).unwrap();

        Ok(WasmtimeEngineProvider {
            module,
            #[cfg(feature = "wasi")]
            wasi_params,
            inner: None,
            store,
            engine,
            linker,
        })
    }
}

impl WebAssemblyEngineProvider for WasmtimeEngineProvider {
    fn init(&mut self, host: Arc<ModuleState>) -> Result<(), Box<dyn Error>> {
        let instance =
            instance_from_module(&mut self.store, &self.module, host.clone(), &self.linker)?;
        let instance_ref = Arc::new(RwLock::new(instance));
        let gc = guest_call_fn(self.store.as_context_mut(), instance_ref.clone())?;
        self.inner = Some(EngineInner {
            instance: instance_ref,
            guest_call_fn: gc,
            host,
        });
        self.initialize()?;
        Ok(())
    }

    fn call(&mut self, op_length: i32, msg_length: i32) -> Result<i32, Box<dyn Error>> {
        let engine_inner = self.inner.as_ref().unwrap();
        let call = engine_inner
            .guest_call_fn
            .call(&mut self.store, &[op_length.into(), msg_length.into()]);

        match call {
            Ok(result) => {
                let result: i32 = result[0].i32().unwrap();
                Ok(result)
            }
            Err(e) => {
                error!("Failure invoking guest module handler: {:?}", e);
                engine_inner.host.set_guest_error(e.to_string());
                Ok(0)
            }
        }
    }

    fn replace(&mut self, module: &[u8]) -> Result<(), Box<dyn Error>> {
        info!(
            "HOT SWAP - Replacing existing WebAssembly module with new buffer, {} bytes",
            module.len()
        );

        let new_instance = instance_from_buffer(
            &mut self.store,
            &self.engine,
            module,
            self.inner.as_ref().unwrap().host.clone(),
            &self.linker,
        )?;
        *self.inner.as_ref().unwrap().instance.write().unwrap() = new_instance;

        self.initialize()
    }
}

impl WasmtimeEngineProvider {
    fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        for starter in wapc::WapcFunctions::REQUIRED_STARTS.iter() {
            if let Some(ext) = self
                .inner
                .as_ref()
                .unwrap()
                .instance
                .read()
                .unwrap()
                .get_export(&mut self.store, starter)
            {
                ext.into_func().unwrap().call(&mut self.store, &[])?;
            }
        }
        Ok(())
    }
}

fn instance_from_buffer(
    store: &mut Store<WapcStore>,
    engine: &Engine,
    buf: &[u8],
    state: Arc<ModuleState>,
    linker: &Linker<WapcStore>,
) -> Result<Instance, Box<dyn Error>> {
    let module = Module::new(engine, buf).unwrap();
    let imports = arrange_imports(&module, state, store, linker);
    Ok(wasmtime::Instance::new(store.as_context_mut(), &module, imports?.as_slice()).unwrap())
}

fn instance_from_module(
    store: &mut Store<WapcStore>,
    module: &Module,
    state: Arc<ModuleState>,
    linker: &Linker<WapcStore>,
) -> Result<Instance, Box<dyn Error>> {
    let imports = arrange_imports(module, state, store, linker);
    Ok(wasmtime::Instance::new(store.as_context_mut(), module, imports?.as_slice()).unwrap())
}

fn init_wasi(params: &WasiParams) -> anyhow::Result<WasiCtx> {
    wasi::init_ctx(
        &wasi::compute_preopen_dirs(&params.preopened_dirs, &params.map_dirs).unwrap(),
        &params.argv,
        &params.env_vars,
    )
    .map_err(|e| anyhow::anyhow!("WASI initialization failed: {}", e))
}

/// wasmtime requires that the list of callbacks be "zippable" with the list
/// of module imports. In order to ensure that both lists are in the same
/// order, we have to loop through the module imports and instantiate the
/// corresponding callback. We **cannot** rely on a predictable import order
/// in the wasm module
#[allow(clippy::unnecessary_wraps)]
fn arrange_imports(
    module: &Module,
    host: Arc<ModuleState>,
    store: &mut impl AsContextMut<Data = WapcStore>,
    linker: &Linker<WapcStore>,
) -> Result<Vec<Extern>, Box<dyn Error>> {
    Ok(module
        .imports()
        .filter_map(|imp| {
            if let ExternType::Func(_) = imp.ty() {
                match imp.module() {
                    HOST_NAMESPACE => Some(callback_for_import(
                        store.as_context_mut(),
                        imp.name()?,
                        host.clone(),
                    )),
                    WASI_SNAPSHOT_PREVIEW1_NAMESPACE | WASI_UNSTABLE_NAMESPACE => {
                        linker.get_by_import(store.as_context_mut(), &imp)
                    }
                    other => panic!("import module `{}` was not found", other), //TODO: get rid of panic
                }
            } else {
                None
            }
        })
        .collect())
}

fn callback_for_import(store: impl AsContextMut, import: &str, host: Arc<ModuleState>) -> Extern {
    match import {
        WapcFunctions::HOST_CONSOLE_LOG => callbacks::console_log_func(store, host).into(),
        WapcFunctions::HOST_CALL => callbacks::host_call_func(store, host).into(),
        WapcFunctions::GUEST_REQUEST_FN => callbacks::guest_request_func(store, host).into(),
        WapcFunctions::HOST_RESPONSE_FN => callbacks::host_response_func(store, host).into(),
        WapcFunctions::HOST_RESPONSE_LEN_FN => {
            callbacks::host_response_len_func(store, host).into()
        }
        WapcFunctions::GUEST_RESPONSE_FN => callbacks::guest_response_func(store, host).into(),
        WapcFunctions::GUEST_ERROR_FN => callbacks::guest_error_func(store, host).into(),
        WapcFunctions::HOST_ERROR_FN => callbacks::host_error_func(store, host).into(),
        WapcFunctions::HOST_ERROR_LEN_FN => callbacks::host_error_len_func(store, host).into(),
        _ => unreachable!(),
    }
}

// Called once, then the result is cached. This returns a `Func` that corresponds
// to the `__guest_call` export
fn guest_call_fn(
    store: impl AsContextMut,
    instance: Arc<RwLock<Instance>>,
) -> Result<Func, Box<dyn Error>> {
    if let Some(func) = instance
        .read()
        .unwrap()
        .get_func(store, WapcFunctions::GUEST_CALL)
    {
        Ok(func)
    } else {
        Err("Guest module did not export __guest_call function!".into())
    }
}
