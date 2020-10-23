use std::error::Error;
use wapc::{ModuleState, WapcFunctions, WasiParams, WebAssemblyEngineProvider, HOST_NAMESPACE};
use wasmtime::{Engine, Extern, ExternType, Func, Instance, Module, Store};

// namespace needed for some language support
const WASI_UNSTABLE_NAMESPACE: &str = "wasi_unstable";
const WASI_SNAPSHOT_PREVIEW1_NAMESPACE: &str = "wasi_snapshot_preview1";

use crate::modreg::ModuleRegistry;
use std::sync::{Arc, RwLock};

#[macro_use]
extern crate log;

mod callbacks;
mod modreg;

macro_rules! call {
    ($func:expr, $($p:expr),*) => {
      match $func.call(&[$($p.into()),*]) {
        Ok(result) => {
          let result: i32 = result[0].i32().unwrap();
          result
        }
        Err(e) => {
            error!("Failure invoking guest module handler: {:?}", e);
            0
        }
      }
    }
}

struct EngineInner {
    instance: Arc<RwLock<Instance>>,
    guest_call_fn: Func,
    host: Arc<ModuleState>,
}

/// A waPC engine provider that encapsulates the Wasmtime WebAssembly runtime
pub struct WasmtimeEngineProvider {
    inner: Option<EngineInner>,
    wasidata: Option<WasiParams>,
    modbytes: Vec<u8>,
}

impl WasmtimeEngineProvider {
    /// Creates a new instance of the wasmtime provider
    pub fn new(buf: &[u8], wasi: Option<WasiParams>) -> WasmtimeEngineProvider {
        WasmtimeEngineProvider {
            inner: None,
            modbytes: buf.to_vec(),
            wasidata: wasi,
        }
    }
}

impl WebAssemblyEngineProvider for WasmtimeEngineProvider {
    fn init(&mut self, host: Arc<ModuleState>) -> Result<(), Box<dyn Error>> {
        let instance = instance_from_buffer(&self.modbytes, &self.wasidata, host.clone())?;
        let instance_ref = Arc::new(RwLock::new(instance));
        let gc = guest_call_fn(instance_ref.clone())?;
        self.inner = Some(EngineInner {
            instance: instance_ref,
            guest_call_fn: gc,
            host,
        });
        self.initialize()?;
        Ok(())
    }

    fn call(&mut self, op_length: i32, msg_length: i32) -> Result<i32, Box<dyn Error>> {
        // Note that during this call, the guest should, through the functions
        // it imports from the host, set the guest error and response

        let callresult: i32 = call!(
            self.inner.as_ref().unwrap().guest_call_fn,
            op_length,
            msg_length
        );

        Ok(callresult)
    }

    fn replace(&mut self, module: &[u8]) -> Result<(), Box<dyn Error>> {
        info!(
            "HOT SWAP - Replacing existing WebAssembly module with new buffer, {} bytes",
            module.len()
        );

        let new_instance = instance_from_buffer(
            module,
            &self.wasidata,
            self.inner.as_ref().unwrap().host.clone(),
        )?;
        *self.inner.as_ref().unwrap().instance.write().unwrap() = new_instance;

        self.initialize()
    }
}

impl WasmtimeEngineProvider {
    fn initialize(&self) -> Result<(), Box<dyn Error>> {
        for starter in wapc::WapcFunctions::REQUIRED_STARTS.iter() {
            if let Some(ext) = self
                .inner
                .as_ref()
                .unwrap()
                .instance
                .read()
                .unwrap()
                .get_export(starter)
            {
                ext.into_func().unwrap().call(&[])?;
            }
        }
        Ok(())
    }
}

fn instance_from_buffer(
    buf: &[u8],
    wasi: &Option<WasiParams>,
    state: Arc<ModuleState>,
) -> Result<Instance, Box<dyn Error>> {
    let engine = Engine::default();
    let store = Store::new(&engine);
    let module = Module::new(&engine, buf).unwrap();

    let d = WasiParams::default();
    let wasi = match wasi {
        Some(w) => w,
        None => &d,
    };

    // Make wasi available by default.
    let preopen_dirs = modreg::compute_preopen_dirs(&wasi.preopened_dirs, &wasi.map_dirs).unwrap();
    let argv = vec![]; // TODO: add support for argv (if applicable)

    let module_registry =
        ModuleRegistry::new(&store, &preopen_dirs, &argv, &wasi.env_vars).unwrap();

    let imports = arrange_imports(&module, state, store.clone(), &module_registry);

    Ok(wasmtime::Instance::new(&store, &module, imports?.as_slice()).unwrap())
}

/// wasmtime requires that the list of callbacks be "zippable" with the list
/// of module imports. In order to ensure that both lists are in the same
/// order, we have to loop through the module imports and instantiate the
/// corresponding callback. We **cannot** rely on a predictable import order
/// in the wasm module
fn arrange_imports(
    module: &Module,
    host: Arc<ModuleState>,
    store: Store,
    mod_registry: &ModuleRegistry,
) -> Result<Vec<Extern>, Box<dyn Error>> {
    Ok(module
        .imports()
        .filter_map(|imp| {
            if let ExternType::Func(_) = imp.ty() {
                match imp.module() {
                    HOST_NAMESPACE => {
                        Some(callback_for_import(imp.name(), host.clone(), store.clone()))
                    }
                    WASI_UNSTABLE_NAMESPACE => {
                        let f = Extern::from(
                            mod_registry
                                .wasi_unstable
                                .get_export(imp.name())
                                .unwrap()
                                .clone(),
                        );
                        Some(f)
                    }
                    WASI_SNAPSHOT_PREVIEW1_NAMESPACE => {
                        let f: Extern = Extern::from(
                            mod_registry
                                .wasi_snapshot_preview1
                                .get_export(imp.name())
                                .unwrap()
                                .clone(),
                        );
                        Some(f)
                    }
                    other => panic!("import module `{}` was not found", other), //TODO: get rid of panic
                }
            } else {
                None
            }
        })
        .collect())
}

fn callback_for_import(import: &str, host: Arc<ModuleState>, store: Store) -> Extern {
    match import {
        WapcFunctions::HOST_CONSOLE_LOG => callbacks::console_log_func(&store, host.clone()).into(),
        WapcFunctions::HOST_CALL => callbacks::host_call_func(&store, host.clone()).into(),
        WapcFunctions::GUEST_REQUEST_FN => {
            callbacks::guest_request_func(&store, host.clone()).into()
        }
        WapcFunctions::HOST_RESPONSE_FN => {
            callbacks::host_response_func(&store, host.clone()).into()
        }
        WapcFunctions::HOST_RESPONSE_LEN_FN => {
            callbacks::host_response_len_func(&store, host.clone()).into()
        }
        WapcFunctions::GUEST_RESPONSE_FN => {
            callbacks::guest_response_func(&store, host.clone()).into()
        }
        WapcFunctions::GUEST_ERROR_FN => callbacks::guest_error_func(&store, host.clone()).into(),
        WapcFunctions::HOST_ERROR_FN => callbacks::host_error_func(&store, host.clone()).into(),
        WapcFunctions::HOST_ERROR_LEN_FN => {
            callbacks::host_error_len_func(&store, host.clone()).into()
        }
        _ => unreachable!(),
    }
}

// Called once, then the result is cached. This returns a `Func` that corresponds
// to the `__guest_call` export
fn guest_call_fn(instance: Arc<RwLock<Instance>>) -> Result<Func, Box<dyn Error>> {
    if let Some(func) = instance.read().unwrap().get_func(WapcFunctions::GUEST_CALL) {
        Ok(func)
    } else {
        Err("Guest module did not export __guest_call function!".into())
    }
}
