use wapc::{WebAssemblyEngineProvider, ModuleState, WapcFunctions, HOST_NAMESPACE};
use std::sync::Arc;
use std::error::Error;

use wasm3::{Environment, Runtime, CallContext};
use wasm3::Module;

#[macro_use]
extern crate log;

mod callbacks;

pub struct Wasm3EngineProvider {
    inner: Option<InnerProvider>,
    modbytes: Vec<u8>
}

impl Wasm3EngineProvider {
    pub fn new(buf: &[u8]) -> Wasm3EngineProvider {
        Wasm3EngineProvider {
            inner: None,
            modbytes: buf.to_vec()
        }
    }
}

struct InnerProvider {
    rt: Runtime,
    mod_name: String,
}

impl WebAssemblyEngineProvider for Wasm3EngineProvider {
    fn init(&mut self, host: Arc<ModuleState>) -> Result<(), Box<dyn Error>> {
        info!("Initializing Wasm3 Engine");
        let env = Environment::new().expect("Unable to create environment");
        let rt = env
            .create_runtime(1024 * 120)
            ?;
        let module = Module::parse(&env, &self.modbytes)
            .map_err(|e| Box::new(e))?;

        let mut module = rt.load_module(module)
            .map_err(|e| Box::new(e))?;

        let mod_name = module.name().to_string();

        let h = host.clone();
        module
            .link_closure(HOST_NAMESPACE,
                          WapcFunctions::HOST_CALL,
            move |ctx: &CallContext, (bd_ptr, bd_len, ns_ptr, ns_len, op_ptr,
            op_len, ptr, len): (i32, i32, i32, i32, i32, i32, i32, i32)| -> i32 {
                callbacks::host_call(ctx, bd_ptr, bd_len, ns_ptr, ns_len, op_ptr, op_len, ptr, len, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::GUEST_REQUEST_FN,
        move |ctx: &CallContext, (op_ptr, ptr): (i32, i32)| {
            callbacks::guest_request(ctx, op_ptr, ptr, h.clone());
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::HOST_CONSOLE_LOG,
        move | ctx: &CallContext, (ptr, len): (i32, i32)| {
            callbacks::console_log(ctx, ptr, len, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::HOST_RESPONSE_FN,
        move | ctx: &CallContext, ptr: i32| {
            callbacks::host_response(ctx, ptr, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::HOST_RESPONSE_LEN_FN,
        move | ctx: &CallContext, ()| -> i32 {
            callbacks::host_response_length(ctx, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::GUEST_RESPONSE_FN,
        move | ctx: &CallContext, (ptr, len): (i32, i32)| {
            callbacks::guest_response(ctx, ptr, len, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::GUEST_ERROR_FN,
        move | ctx: &CallContext, (ptr, len): (i32, i32)| {
            callbacks::guest_error(ctx, ptr, len, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::HOST_ERROR_FN,
        move | ctx: &CallContext, ptr: i32| {
            callbacks::host_error(ctx, ptr, h.clone())
        })?;

        let h = host.clone();
        module.link_closure(HOST_NAMESPACE,
        WapcFunctions::HOST_ERROR_LEN_FN,
        move | _ctx: &CallContext, ()| -> i32 {
            callbacks::host_error_length(h.clone())
        })?;

        // Fail the initialization if we can't find the guest call function
        let _func = module
            .find_function::<(i32, i32), i32>(WapcFunctions::GUEST_CALL)?;

        self.inner = Some(InnerProvider {
            rt,
            mod_name,
        });

        Ok(())
    }

    fn call(&mut self, op_length: i32, msg_length: i32) -> Result<i32, Box<dyn Error>> {
        if let Some(ref i) = self.inner {
            let module = i.rt.find_module(&i.mod_name)?;
            let func = module.find_function::<(i32, i32), i32>(WapcFunctions::GUEST_CALL)?;
            let res = func.call(op_length, msg_length)?;
            Ok(res)
        } else {
            Err("Module call failure - no module was initialized".into())
        }
    }

    fn replace(&mut self, _bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        unimplemented!()
    }
}
