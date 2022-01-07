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

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

/// [CallResult] is the result for all waPC host and guest calls.
pub type CallResult = Result<Vec<u8>, Box<dyn std::error::Error + Sync + Send>>;

/// A generic type for the result of waPC operation handlers.
pub type HandlerResult<T> = Result<T, Box<dyn std::error::Error + Sync + Send>>;

#[link(wasm_import_module = "wapc")]
extern "C" {
  /// The host's exported __console_log function.
  pub fn __console_log(ptr: *const u8, len: usize);
  /// The host's exported __host_call function.
  pub fn __host_call(
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
  pub fn __host_response(ptr: *mut u8);
  /// The host's exported __host_response_len function.
  pub fn __host_response_len() -> usize;
  /// The host's exported __host_error_len function.
  pub fn __host_error_len() -> usize;
  /// The host's exported __host_error function.
  pub fn __host_error(ptr: *mut u8);
  /// The host's exported __guest_response function.
  pub fn __guest_response(ptr: *const u8, len: usize);
  /// The host's exported __guest_error function.
  pub fn __guest_error(ptr: *const u8, len: usize);
  /// The host's exported __guest_request function.
  pub fn __guest_request(op_ptr: *mut u8, ptr: *mut u8);
}

lazy_static! {
  static ref REGISTRY: RwLock<HashMap<String, fn(&[u8]) -> CallResult>> =
    RwLock::new(HashMap::new());
}

/// Register a handler for a waPC operation
pub fn register_function(name: &str, f: fn(&[u8]) -> CallResult) {
  REGISTRY.write().unwrap().insert(name.to_owned(), f);
}

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

  let opstr = ::std::str::from_utf8(&opbuf).unwrap();

  match REGISTRY.read().unwrap().get(opstr) {
    Some(handler) => match handler(&buf) {
      Ok(result) => {
        unsafe {
          __guest_response(result.as_ptr(), result.len());
        }
        1
      }
      Err(e) => {
        let errmsg = format!("Guest call failed: {}", e);
        unsafe {
          __guest_error(errmsg.as_ptr(), errmsg.len());
        }
        0
      }
    },
    None => {
      let errmsg = format!("No handler registered for function \"{}\"", opstr);
      unsafe {
        __guest_error(errmsg.as_ptr(), errmsg.len());
      }
      0
    }
  }
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

    Err(Box::new(errors::new(errors::ErrorKind::HostError(
      String::from_utf8(buf).unwrap(),
    ))))
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

pub mod errors;
pub mod prelude;
