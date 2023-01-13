use std::collections::HashMap;
use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::errors;

/// [CallResult] is the result for all waPC host and guest calls.
pub type CallResult = Result<Vec<u8>, Box<dyn std::error::Error + Sync + Send>>;

/// A generic type for the result of waPC operation handlers.
pub type HandlerResult<T> = Result<T, Box<dyn std::error::Error + Sync + Send>>;

/// The [__guest_call] function is required by waPC guests and should only be called by waPC hosts.
#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn __guest_call(op_len: i32, req_len: i32) -> i32 {
  let mut buf: Vec<u8> = Vec::with_capacity(req_len as _);
  let mut opbuf: Vec<u8> = Vec::with_capacity(op_len as _);

  unsafe {
    __guest_request(opbuf.as_mut_ptr(), buf.as_mut_ptr());
    // The two buffers have now been initialized
    buf.set_len(req_len as usize);
    opbuf.set_len(op_len as usize);
  };

  REGISTRY.read().unwrap().get(&opbuf).map_or_else(
    || {
      let mut errmsg = b"No handler registered for function ".to_vec();
      errmsg.append(&mut opbuf);
      unsafe {
        __guest_error(errmsg.as_ptr(), errmsg.len());
      }
      0
    },
    |handler| match handler(&buf) {
      Ok(result) => {
        unsafe {
          __guest_response(result.as_ptr(), result.len());
        }
        1
      }
      Err(e) => {
        let errmsg = e.to_string();
        unsafe {
          __guest_error(errmsg.as_ptr(), errmsg.len());
        }
        0
      }
    },
  )
}

#[link(wasm_import_module = "wapc")]
extern "C" {
  /// The host's exported __console_log function.
  pub(crate) fn __console_log(ptr: *const u8, len: usize);
  /// The host's exported __host_call function.
  pub(crate) fn __host_call(
    bd_ptr: *const u8,
    bd_len: usize,
    ns_ptr: *const u8,
    ns_len: usize,
    op_ptr: *const u8,
    op_len: usize,
    ptr: *const u8,
    len: usize,
  ) -> usize;
  /// The host's exported __host_response function.
  pub(crate) fn __host_response(ptr: *mut u8);
  /// The host's exported __host_response_len function.
  pub(crate) fn __host_response_len() -> usize;
  /// The host's exported __host_error_len function.
  pub(crate) fn __host_error_len() -> usize;
  /// The host's exported __host_error function.
  pub(crate) fn __host_error(ptr: *mut u8);
  /// The host's exported __guest_response function.
  pub(crate) fn __guest_response(ptr: *const u8, len: usize);
  /// The host's exported __guest_error function.
  pub(crate) fn __guest_error(ptr: *const u8, len: usize);
  /// The host's exported __guest_request function.
  pub(crate) fn __guest_request(op_ptr: *mut u8, ptr: *mut u8);
}

type HandlerSignature = fn(&[u8]) -> CallResult;

static REGISTRY: Lazy<RwLock<HashMap<Vec<u8>, HandlerSignature>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a handler for a waPC operation
pub fn register_function(name: &str, f: fn(&[u8]) -> CallResult) {
  REGISTRY.write().unwrap().insert(name.as_bytes().to_vec(), f);
}

/// The function through which all host calls take place.
pub fn host_call(binding: &str, ns: &str, op: &str, msg: &[u8]) -> CallResult {
  #[allow(unsafe_code)]
  let callresult = unsafe {
    __host_call(
      binding.as_ptr(),
      binding.len(),
      ns.as_ptr(),
      ns.len(),
      op.as_ptr(),
      op.len(),
      msg.as_ptr(),
      msg.len(),
    )
  };
  if callresult != 1 {
    // call was not successful
    #[allow(unsafe_code)]
    let errlen = unsafe { __host_error_len() };

    let mut buf = Vec::with_capacity(errlen);
    let retptr = buf.as_mut_ptr();

    #[allow(unsafe_code)]
    unsafe {
      __host_error(retptr);
      buf.set_len(errlen);
    }

    Err(Box::new(errors::new(errors::ErrorKind::HostError(buf))))
  } else {
    // call succeeded
    #[allow(unsafe_code)]
    let len = unsafe { __host_response_len() };

    let mut buf = Vec::with_capacity(len);
    let retptr = buf.as_mut_ptr();

    #[allow(unsafe_code)]
    unsafe {
      __host_response(retptr);
      buf.set_len(len);
    }
    Ok(buf)
  }
}

/// Log function that delegates to the host's __console_log function
#[cold]
#[inline(never)]
pub fn console_log(s: &str) {
  #[allow(unsafe_code)]
  unsafe {
    __console_log(s.as_ptr(), s.len());
  }
}
