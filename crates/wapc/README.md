# waPC

![crates.io](https://img.shields.io/crates/v/wapc.svg)
![license](https://img.shields.io/crates/l/wapc.svg)

This is the Rust implementation of the **waPC** host protocol. This crate defines the `WapcHost` struct and the `WebAssemblyEngineProvider` trait to provide dynamic execution of WebAssembly modules. For information on the implementations with WebAssembly engines, check out the provider crates below:

- [wasmtime-provider](https://github.com/wapc/wapc-rs/blob/master/crates/wasmtime-provider)
- [wasm3-provider](https://github.com/wapc/wapc-rs/blob/master/crates/wasm3-provider)

# wapc

The `wapc` crate provides a high-level WebAssembly host runtime that conforms to an RPC mechanism called **waPC** (WebAssembly Procedure Calls). waPC is designed to be a fixed, lightweight standard allowing both sides of the guest/host boundary to make method calls containing arbitrary binary payloads. Neither side
of the contract is ever required to perform explicit allocation, ensuring maximum portability for wasm targets that might behave differently in the presence of garbage collectors and memory
relocation, compaction, etc.

To use `wapc`, first you'll need a waPC-compliant WebAssembly module (referred to as the _guest_) to load and execute. You can find a number of these samples available in the [GitHub repository](https://github.com/wapc/wapc-rs/blob/master/wasm/crates/).

Next, you will need to chose a runtime engine. waPC describes the function call flow required for wasm-RPC, but it does not dictate how the low-level WebAssembly function calls are made. This allows you to select whatever engine best suits your needs, whether it's a JIT-based engine or an interpreter-based one. Simply instantiate anything that implements the
[WebAssemblyEngineProvider](https://docs.rs/wapc/latest/wapc/trait.WebAssemblyEngineProvider.html) trait and pass it to the WapcHost constructor and the [WapcHost](https://docs.rs/wapc/latest/wapc/struct.WapcHost.html) will facilitate all RPCs.

To make function calls, ensure that you provided a suitable host callback function (or closure) when you created your WapcHost. Then invoke the `call` function to initiate the RPC flow.

## Example

The following is an example of synchronous, bi-directional procedure calls between a WebAssembly host runtime and the guest module.

```rust
use wasmtime_provider::WasmtimeEngineProvider; // Or Wasm3EngineProvider
use wapc::WapcHost;
use std::error::Error;
pub fn main() -> Result<(), Box<dyn Error>> {

  // Sample host callback that prints the operation a WASM module requested.
  let host_callback = |id: u64, bd: &str, ns: &str, op: &str, payload: &[u8]| {
    println!("Guest {} invoked '{}->{}:{}' with a {} byte payload",
    id, bd, ns, op, payload.len());
    // Return success with zero-byte payload.
    Ok(vec![])
  };

  let file = "../../wasm/crates/wasm-basic/build/wasm_basic.wasm";
  let module_bytes = std::fs::read(file)?;

  let engine = WasmtimeEngineProvider::new(&module_bytes, None)?;
  let host = WapcHost::new(Box::new(engine), Some(Box::new(host_callback)))?;

  let res = host.call("ping", b"payload bytes")?;
  assert_eq!(res, b"payload bytes");

  Ok(())
}
```

For running examples, take a look at the examples available in the individual engine provider
repositories:

- [wasmtime-provider](https://github.com/wapc/wapc-rs/blob/master/crates/wasmtime-provider/examples) - Utilizes the [Bytecode Alliance](https://bytecodealliance.org/) runtime [wasmtime](https://github.com/bytecodealliance/wasmtime) for WebAssembly JIT compilation and execution.
- [wasm3-provider](https://github.com/wapc/wapc-rs/blob/master/crates/wasm3-provider/examples) - Uses the [wasm3](https://github.com/wasm3) C interpreter runtime (with a [Rust wrapper](https://github.com/wasm3/wasm3-rs))

# Notes

waPC is _reactive_. Hosts make requests and guests respond. During a request, guests can initiate calls back to the host and interact with the environment (via WASI). When a request is done the guest should be considered parked until the next request.

## RPC Exchange Flow

The following is a detailed outline of which functions are invoked and in which order to support
a waPC exchange flow, which is always triggered by a consumer invoking the `call` function. Invoking
and handling these low-level functions is the responsibility of the _engine provider_, while
orchestrating the high-level control flow is the job of the `WapcHost`.

1. Host invokes `__guest_call` on the WebAssembly module (via the engine provider)
1. Guest calls the `__guest_request` function to instruct the host to write the request parameters to linear memory
1. Guest uses the `op_len` and `msg_len` parameters long with the pointer values it generated in step 2 to retrieve the operation (UTF-8 string) and payload (opaque byte array)
1. Guest performs work
1. _(Optional)_ Guest invokes `__host_call` on host with pointers and lengths indicating the `binding`, `namespace`, `operation`, and payload.
1. _(Optional)_ Guest can use `__host_response` and `host_response_len` functions to obtain and interpret results
1. _(Optional)_ Guest can use `__host_error_len` and `__host_error` to obtain the host error if indicated (`__host_call` returns 0)
   1. Steps 5-7 can repeat with as many different host calls as the guest needs
1. Guest will call `guest_error` to indicate if an error occurred during processing
1. Guest will call `guest_response` to store the opaque response payload
1. Guest will return 0 (error) or 1 (success) at the end of `__guest_call`

## Required Host Exports

List of functions that must be exported by the host (imported by the guest)

| Module | Function              | Parameters                                                                                                                       | Description                                                                                      |
| ------ | --------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| wapc   | \_\_host_call         | br_ptr: i32<br/>bd_len: i32<br/>ns_ptr: i32<br/>ns_len: i32<br/>op_ptr: i32<br/>op_len: i32<br/>ptr: i32<br/>len: i32<br/>-> i32 | Invoked to initiate a host call                                                                  |
| wapc   | \_\_console_log       | ptr: i32, len: i32                                                                                                               | Allows guest to log to `stdout`                                                                  |
| wapc   | \_\_guest_request     | op_ptr: i32<br/>ptr: i32                                                                                                         | Writes the guest request payload and operation name to linear memory at the designated locations |
| wapc   | \_\_host_response     | ptr: i32                                                                                                                         | Instructs host to write the host response payload to the given location in linear memory         |
| wapc   | \_\_host_response_len | -> i32                                                                                                                           | Obtains the length of the current host response                                                  |
| wapc   | \_\_guest_response    | ptr: i32<br/>len: i32                                                                                                            | Tells the host the size and location of the current guest response payload                       |
| wapc   | \_\_guest_error       | ptr: i32<br/>len: i32                                                                                                            | Tells the host the size and location of the current guest error payload                          |
| wapc   | \_\_host_error        | ptr: i32                                                                                                                         | Instructs the host to write the host error payload to the given location                         |
| wapc   | \_\_host_error_len    | -> i32                                                                                                                           | Queries the host for the length of the current host error (0 if none)                            |

## Required Guest Exports

List of functions that must be exported by the guest (invoked by the host)

| Function       | Parameters                   | Description                                                        |
| -------------- | ---------------------------- | ------------------------------------------------------------------ |
| \_\_guest_call | op_len: i32<br/>msg_len: i32 | Invoked by the host to start an RPC exchange with the guest module |
