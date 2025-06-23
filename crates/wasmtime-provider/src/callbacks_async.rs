use anyhow::anyhow;
use wapc::{wapc_functions, HOST_NAMESPACE};
use wasmtime::{AsContext, AsContextMut, Caller, Linker, Memory, StoreContext};

use crate::errors::{Error, Result};
use crate::store_async::WapcStoreAsync;

pub(crate) fn add_to_linker(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  register_guest_request_func(linker)?;
  register_console_log_func(linker)?;
  register_host_call_func(linker)?;
  register_host_response_func(linker)?;
  register_host_response_len_func(linker)?;
  register_guest_response_func(linker)?;
  register_guest_error_func(linker)?;
  register_host_error_func(linker)?;
  register_host_error_len_func(linker)?;

  Ok(())
}

fn register_guest_request_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::GUEST_REQUEST_FN,
      |mut caller: Caller<'_, WapcStoreAsync>, (op_ptr, ptr): (i32, i32)| {
        Box::new(async move {
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;
          let invocation = host.get_guest_request().await;
          let memory = get_caller_memory(&mut caller)?;
          if let Some(inv) = invocation {
            write_bytes_to_memory(caller.as_context_mut(), memory, ptr, &inv.msg)?;
            write_bytes_to_memory(caller.as_context_mut(), memory, op_ptr, inv.operation.as_bytes())?;
          };
          Ok(())
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::GUEST_REQUEST_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_console_log_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::HOST_CONSOLE_LOG,
      |mut caller: Caller<'_, WapcStoreAsync>, (ptr, len): (i32, i32)| {
        Box::new(async move {
          let memory = get_caller_memory(&mut caller)?;
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;
          let vec = get_vec_from_memory(caller.as_context(), memory, ptr, len);

          let msg = std::str::from_utf8(&vec)
            .map_err(|e| anyhow!(format!("console_log: cannot convert message to UTF8: {:?}", e)))?;

          host.do_console_log(msg);
          Ok(())
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::HOST_CONSOLE_LOG),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_host_call_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::HOST_CALL,
      |mut caller: Caller<'_, WapcStoreAsync>,
       (bd_ptr, bd_len, ns_ptr, ns_len, op_ptr, op_len, ptr, len): (i32, i32, i32, i32, i32, i32, i32, i32)| {
        Box::new(async move {
          let memory = get_caller_memory(&mut caller)?;

          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          let vec = get_vec_from_memory(caller.as_context(), memory, ptr, len);
          let bd_vec = get_vec_from_memory(caller.as_context(), memory, bd_ptr, bd_len);
          let bd = std::str::from_utf8(&bd_vec)
            .map_err(|e| anyhow!(format!("host_call: cannot convert bd to UTF8: {:?}", e)))?
            .to_owned();
          let ns_vec = get_vec_from_memory(caller.as_context(), memory, ns_ptr, ns_len);
          let ns = std::str::from_utf8(&ns_vec)
            .map_err(|e| anyhow!(format!("host_call: cannot convert ns to UTF8: {:?}", e)))?
            .to_owned();
          let op_vec = get_vec_from_memory(caller.as_context(), memory, op_ptr, op_len);
          let op = std::str::from_utf8(&op_vec)
            .map_err(|e| anyhow!(format!("host_call: cannot convert op to UTF8: {:?}", e)))?
            .to_owned();

          let result = host.do_host_call(bd, ns, op, vec).await;
          Ok(result.unwrap_or(0))
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::HOST_CALL),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_host_response_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::HOST_RESPONSE_FN,
      |mut caller: Caller<'_, WapcStoreAsync>, (ptr,): (i32,)| {
        Box::new(async move {
          let memory = get_caller_memory(&mut caller)?;
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          if let Some(ref e) = host.get_host_response().await {
            write_bytes_to_memory(caller.as_context_mut(), memory, ptr, e)?;
          }
          Ok(())
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::HOST_RESPONSE_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_host_response_len_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::HOST_RESPONSE_LEN_FN,
      |caller: Caller<'_, WapcStoreAsync>, ()| {
        Box::new(async move {
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          let len = host.get_host_response().await.map_or_else(|| 0, |r| r.len()) as i32;
          Ok(len)
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::HOST_RESPONSE_LEN_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_guest_response_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::GUEST_RESPONSE_FN,
      |mut caller: Caller<'_, WapcStoreAsync>, (ptr, len): (i32, i32)| {
        Box::new(async move {
          let memory = get_caller_memory(&mut caller)?;

          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          let vec = get_vec_from_memory(caller.as_context(), memory, ptr, len);
          host.set_guest_response(vec).await;
          Ok(())
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::GUEST_RESPONSE_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_guest_error_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::GUEST_ERROR_FN,
      |mut caller: Caller<'_, WapcStoreAsync>, (ptr, len): (i32, i32)| {
        Box::new(async move {
          let memory = get_caller_memory(&mut caller)?;
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          let vec = get_vec_from_memory(caller.as_context(), memory, ptr, len);
          let guest_err_msg = String::from_utf8(vec)
            .map_err(|e| anyhow!(format!("guest_error_func: cannot convert message to UTF8: {:?}", e)))?;
          host.set_guest_error(guest_err_msg).await;
          Ok(())
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::GUEST_ERROR_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_host_error_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::HOST_ERROR_FN,
      |mut caller: Caller<'_, WapcStoreAsync>, (ptr,): (i32,)| {
        Box::new(async move {
          let memory = get_caller_memory(&mut caller)?;
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          if let Some(ref e) = host.get_host_error().await {
            write_bytes_to_memory(caller.as_context_mut(), memory, ptr, e.as_bytes())?;
          }
          Ok(())
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::HOST_ERROR_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn register_host_error_len_func(linker: &mut Linker<WapcStoreAsync>) -> Result<()> {
  linker
    .func_wrap_async(
      HOST_NAMESPACE,
      wapc_functions::HOST_ERROR_LEN_FN,
      |caller: Caller<'_, WapcStoreAsync>, ()| {
        Box::new(async move {
          let host = caller
            .data()
            .host
            .as_ref()
            .ok_or_else(|| anyhow!("host should have been set during the init"))?;

          let len = host.get_host_error().await.map_or_else(|| 0, |r| r.len()) as i32;
          Ok(len)
        })
      },
    )
    .map_err(|e| Error::LinkerFuncDef {
      func: format!("{}.{}", HOST_NAMESPACE, wapc_functions::HOST_ERROR_LEN_FN),
      err: e.to_string(),
    })?;
  Ok(())
}

fn get_caller_memory<T>(caller: &mut Caller<T>) -> anyhow::Result<Memory> {
  let memory_export = caller
    .get_export("memory")
    .ok_or_else(|| anyhow!("Cannot find 'mem' export"))?;
  memory_export
    .into_memory()
    .ok_or_else(|| anyhow!("'mem' export cannot be converted into a Memory instance"))
}

fn get_vec_from_memory<'a, T: 'static>(
  store: impl Into<StoreContext<'a, T>>,
  mem: Memory,
  ptr: i32,
  len: i32,
) -> Vec<u8> {
  let data = mem.data(store);
  data[ptr as usize..(ptr + len) as usize].to_vec()
}

fn write_bytes_to_memory(store: impl AsContextMut, memory: Memory, ptr: i32, slice: &[u8]) -> anyhow::Result<()> {
  memory
    .write(store, ptr as usize, slice)
    .map_err(|e| anyhow!(e.to_string()))
}
