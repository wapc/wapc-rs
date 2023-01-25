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

/// This crate's error module
pub mod errors;
use std::error::Error;
use std::sync::Arc;

use parking_lot::Mutex;
use wapc::{wapc_functions, ModuleState, WebAssemblyEngineProvider, HOST_NAMESPACE};
use wasm3::error::Trap;
use wasm3::{CallContext, Environment, Module, Runtime};

use crate::errors::SendSyncResult;

#[macro_use]
extern crate log;

mod callbacks;

const WASI_UNSTABLE: &str = "wasi_unstable";

/// [Wasm3EngineProvider] implements the [WebAssemblyEngineProvider] trait and normalizes the interface to the wasm3 engine.
#[must_use]
#[allow(missing_debug_implementations)]
pub struct Wasm3EngineProvider {
  inner: Option<InnerProvider>,
  modbytes: Mutex<Vec<u8>>,
}

impl Wasm3EngineProvider {
  /// Instantiate a new wasm3 provider with the supplied wasm module.
  pub fn new(buf: &[u8]) -> Wasm3EngineProvider {
    Wasm3EngineProvider {
      inner: None,
      modbytes: Mutex::new(buf.to_vec()),
    }
  }
}

struct InnerProvider {
  rt: Runtime,
}

impl WebAssemblyEngineProvider for Wasm3EngineProvider {
  // TODO: refactor to avoid skipping this lint
  #[allow(clippy::too_many_lines)]
  fn init(&mut self, host: Arc<ModuleState>) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    info!("Initializing Wasm3 Engine");

    let env = match Environment::new() {
      Ok(env) => env,
      Err(e) => {
        panic!("Could not create a wasm3 environment: {}.", e)
      }
    };
    let rt = env.create_runtime(1024 * 120).to_wapc()?;
    let bytes = self.modbytes.lock();
    let module = Module::parse(&env, bytes.as_ref()).to_wapc()?;

    let mut module = rt.load_module(module).to_wapc()?;
    module.link_wasi().to_wapc()?;
    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::HOST_CALL,
      move |ctx: CallContext,
            (bd_ptr, bd_len, ns_ptr, ns_len, op_ptr, op_len, ptr, len): (i32, i32, i32, i32, i32, i32, i32, i32)|
            -> Result<i32, Trap> {
        Ok(callbacks::host_call(
          &ctx, bd_ptr, bd_len, ns_ptr, ns_len, op_ptr, op_len, ptr, len, &h,
        ))
      },
    ) {
      warn!("Guest module did not import __host_call - functionality may be limited");
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::GUEST_REQUEST_FN,
      move |ctx: CallContext, (op_ptr, ptr): (i32, i32)| {
        callbacks::guest_request(&ctx, op_ptr, ptr, &h);
        Ok(())
      },
    ) {
      error!("Module did not import __guest_request - will not work with waPC");
      return Err("Module did not import __guest_request - will not work with waPC".into());
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::HOST_CONSOLE_LOG,
      move |ctx: CallContext, (ptr, len): (i32, i32)| {
        callbacks::console_log(&ctx, ptr, len, &h);
        Ok(())
      },
    ) {
      warn!("Module did not import __console_log");
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::HOST_RESPONSE_FN,
      move |ctx: CallContext, ptr: i32| {
        callbacks::host_response(&ctx, ptr, &h);
        Ok(())
      },
    ) {
      warn!("Module did not import __host_response");
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::HOST_RESPONSE_LEN_FN,
      move |ctx: CallContext, ()| -> Result<i32, Trap> { Ok(callbacks::host_response_length(&ctx, &h)) },
    ) {
      warn!("Module did not import __host_response_len");
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::GUEST_RESPONSE_FN,
      move |ctx: CallContext, (ptr, len): (i32, i32)| {
        callbacks::guest_response(&ctx, ptr, len, &h);
        Ok(())
      },
    ) {
      error!("Module did not import __guest_response");
      return Err("Module did not import __guest_response".into());
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::GUEST_ERROR_FN,
      move |ctx: CallContext, (ptr, len): (i32, i32)| {
        callbacks::guest_error(&ctx, ptr, len, &h);
        Ok(())
      },
    ) {
      error!("Module did not import __guest_error");
      return Err("Module did not import __guest_error".into());
    }

    let h = host.clone();
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::HOST_ERROR_FN,
      move |ctx: CallContext, ptr: i32| {
        callbacks::host_error(&ctx, ptr, &h);
        Ok(())
      },
    ) {
      warn!("Module did not import __host_error");
    }

    let h = host;
    if let Err(_e) = module.link_closure(
      HOST_NAMESPACE,
      wapc_functions::HOST_ERROR_LEN_FN,
      move |_ctx: CallContext, ()| -> Result<i32, Trap> { Ok(callbacks::host_error_length(&h)) },
    ) {
      warn!("Module did not import __host_error_len");
    }

    let _ = module.link_closure(
      WASI_UNSTABLE,
      "fd_write",
      move |_ctx: CallContext, (_, _, _, _): (i32, i32, i32, i32)| -> Result<i32, Trap> {
        warn!("Use of prohibited (WASI) fd_write function - suppressing output");
        Ok(0)
      },
    ); // don't care if this function is missing

    // Fail the initialization if we can't find the guest call function
    if let Err(_e) = module.find_function::<(i32, i32), i32>(wapc_functions::GUEST_CALL) {
      error!("Could not find __guest_call function in WebAssembly module");
      return Err("Could not find __guest_call function in WebAssembly module".into());
    }

    // Invoke all the starters in order (if they exist)
    for starter in wapc::wapc_functions::REQUIRED_STARTS.iter() {
      let func = module.find_function::<(), ()>(starter);
      if let Ok(func) = func {
        if let Err(e) = func.call() {
          error!("Failed during invocation of starter function '{}': {}.", starter, e);
          return Err(format!("Failed during starter initialization '{}': {}", starter, e).into());
        }
      }
    }

    self.inner = Some(InnerProvider { rt });

    Ok(())
  }

  fn call(&mut self, op_length: i32, msg_length: i32) -> Result<i32, Box<dyn Error + Send + Sync + 'static>> {
    if let Some(ref i) = self.inner {
      let func = i
        .rt
        .find_function::<(i32, i32), i32>(wapc_functions::GUEST_CALL)
        .to_wapc()?;
      let res = func.call(op_length, msg_length).to_wapc()?;
      Ok(res)
    } else {
      Err("Module call failure - no module was initialized".into())
    }
  }

  fn replace(&mut self, _bytes: &[u8]) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    unimplemented!()
  }
}
