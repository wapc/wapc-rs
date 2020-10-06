![crates.io](https://img.shields.io/crates/v/wapc-guest.svg)&nbsp;
![travis](https://travis-ci.org/wapc/wapc-guest-rust.svg?branch=master)&nbsp;
![license](https://img.shields.io/crates/l/wapc-guest.svg)

# waPC Guest SDK

The waPC Guest SDK is used by Rust developers building workloads for the `wasm32-unknown-unknown` target that will conform to the [waPC](https://wascap.io/comms) (WebAssembly Procedure Calls) specification. 

This crate is used by [waSCC](https://wascc.dev) as a foundation for its secure, dynamic binding of cloud capabilities on top of the **waPC** spec.

# Example

```rust
extern crate wapc_guest as guest;

use guest::prelude::*;

#[no_mangle]
pub fn _start() {
    register_function("sample:Guest!Hello", hello_world);   
}

fn hello_world(_msg: &[u8]) -> CallResult {
    let _res = host_call("myBinding", "sample:Host", "Call", b"hello")?;
    Ok(vec![])
}
```
