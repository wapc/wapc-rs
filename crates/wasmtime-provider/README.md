# Wasmtime Engine Provider

![crates.io](https://img.shields.io/crates/v/wasmtime-provider.svg)
![license](https://img.shields.io/crates/l/wasmtime-provider.svg)

This is a pluggable engine provider for the [waPC](https://wapc.io) RPC exchange protocol. This engine implements `WebAssemblyEngineProvider` for the the Bytecode Alliance's [wasmtime](https://github.com/bytecodealliance/wasmtime) WebAssembly runtime.

## Running the demo

```ignore
$ cargo run -p wasmtime-provider --example wasmtime-demo ./wasm/crates/wasm-basic/build/wasm_basic.wasm ping "hi"
```

## Example

```rust
use wasmtime_provider::WasmtimeEngineProvider;
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

## See also

- [wasm3-provider](https://crates.io/crates/wasm3-provider)
