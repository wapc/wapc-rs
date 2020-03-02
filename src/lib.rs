// Copyright 2015-2019 Capital One Services, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # wapc-guest
//!
//! The `wapc-guest` library provides WebAssembly module developers with access to a
//! [waPC](https://wascap.io/comms)-compliant host runtime. Each guest module has a single
//! call handler, declared with the `wapc_handler!` macro. Inside this call handler, the guest
//! module should check the operation of the delivered message and handle it accordingly,
//! returning any binary payload in response. 
//!
//! # Example
//! ```
//! extern crate wapc_guest as guest;
//!
//! use guest::prelude::*;
//!
//! wapc_handler!(handle_wapc);
//!
//! pub fn handle_wapc(operation: &str, msg: &[u8]) -> CallResult {
//!     match operation {
//!         "sample:Guest!Hello" => hello_world(msg),
//!         _ => Err("bad dispatch".into()),
//!     }     
//! }
//!
//! fn hello_world(
//!    _msg: &[u8]) -> CallResult {
//!    let _res = host_call("sample:Host", "Call", b"hello")?;
//!     Ok(vec![])
//! }
//! ```

/// WaPC Guest SDK result type
pub type Result<T> = std::result::Result<T, errors::Error>;

#[link(wasm_import_module = "wapc")]
extern "C" {
    pub fn __console_log(ptr: *const u8, len: usize);
    pub fn __host_call(ns_ptr: *const u8, ns_len: usize, op_ptr: *const u8, op_len: usize, ptr: *const u8, len: usize) -> usize;
    pub fn __host_response(ptr: *const u8);
    pub fn __host_response_len() -> usize;
    pub fn __host_error_len() -> usize;
    pub fn __host_error(ptr: *const u8);
    pub fn __guest_response(ptr: *const u8, len: usize);
    pub fn __guest_error(ptr: *const u8, len: usize);    
    pub fn __guest_request(op_ptr: *const u8, ptr: *const u8);
}


/// The function through which all host calls take place. 
pub fn host_call(ns: &str, op: &str, msg: &[u8]) -> Result<Vec<u8>> {
    
    let callresult = unsafe {
        __host_call(ns.as_ptr() as _, ns.len() as _, op.as_ptr() as _, op.len() as _, msg.as_ptr() as _, msg.len() as _)            
    };
    if callresult != 1 { // call was not successful
        let errlen = unsafe { __host_error_len() };
        let buf = Vec::with_capacity(errlen as _);
        let retptr = buf.as_ptr();
        let slice = unsafe {
            __host_error(retptr);
            std::slice::from_raw_parts(retptr as _, errlen as _)
        };
        Err(errors::new(errors::ErrorKind::HostError(
            String::from_utf8(slice.to_vec()).unwrap(),
        )))
    } else { // call succeeded
        let len = unsafe { __host_response_len() };
        let buf = Vec::with_capacity(len as _);
        let retptr = buf.as_ptr();
        let slice = unsafe {
            __host_response(retptr);
            std::slice::from_raw_parts(retptr as _, len as _)
        };
        Ok(slice.to_vec())
    }
}


#[macro_export]
macro_rules! wapc_handler {
    ($user_handler:ident) => {
        #[no_mangle]
        pub extern "C" fn __guest_call(op_len: i32, req_len: i32) -> i32 {            
            use std::slice; 
            use $crate::console_log;          

            let buf: Vec<u8> = Vec::with_capacity(req_len as _);
            let req_ptr = buf.as_ptr();

            let opbuf: Vec<u8> = Vec::with_capacity(op_len as _);
            let op_ptr = opbuf.as_ptr();

            let (slice, op) = unsafe {
                $crate::__guest_request(op_ptr, req_ptr);
                (
                    slice::from_raw_parts(req_ptr, req_len as _),
                    slice::from_raw_parts(op_ptr, op_len as _),
                )
            };

            let opstr = ::std::str::from_utf8(op).unwrap();            

            console_log(&format!(
                "Performing guest call, operation - {}",
                opstr                
            ));
            match $user_handler(&opstr, slice) {
                Ok(msg) => unsafe {
                    $crate::__guest_response(msg.as_ptr(), msg.len() as _);
                    1
                },
                Err(e) => {
                    let errmsg = format!("Guest call failed: {}", e);
                    console_log(&errmsg);
                    unsafe {
                        $crate::__guest_error(errmsg.as_ptr(), errmsg.len() as _);
                    }
                    0
                }
            }
        }
    };
}

#[cold]
#[inline(never)]
pub fn console_log(s: &str) {
    unsafe {
        __console_log(s.as_ptr(), s.len());
    }
}

pub mod errors;
pub mod prelude;
