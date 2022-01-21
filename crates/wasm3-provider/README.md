# Wasm3 Engine Provider

![crates.io](https://img.shields.io/crates/v/wasm3-provider.svg)
![license](https://img.shields.io/crates/l/wasm3-provider.svg)

This is a pluggable engine provider for the [waPC](https://wapc.io) RPC exchange protocol. This engine implements `WebAssemblyEngineProvider` for the [wasm3](https://github.com/wasm3) C-based, interpreted WebAssembly runtime.

## Running the demo

```ignore
$ cargo run -p wasm3-provider --example wasm3-demo ./wasm/crates/wasm-basic/build/wasm_basic.wasm ping "hi"
```

## Example

```rust
use wasm3_provider::Wasm3EngineProvider;
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

  let engine = Wasm3EngineProvider::new(&module_bytes);
  let host = WapcHost::new(Box::new(engine), Some(Box::new(host_callback)))?;

  let res = host.call("ping", b"payload bytes")?;
  assert_eq!(res, b"payload bytes");

  Ok(())
}
```

## See also

- [wasmtime-provider](https://crates.io/crates/wasmtime-provider)
