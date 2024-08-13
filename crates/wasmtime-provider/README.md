# Wasmtime Engine Provider

![crates.io](https://img.shields.io/crates/v/wasmtime-provider.svg)
![license](https://img.shields.io/crates/l/wasmtime-provider.svg)

This is a pluggable engine provider for the [waPC](https://wapc.io) RPC exchange protocol. This engine implements `WebAssemblyEngineProvider` for the the Bytecode Alliance's [wasmtime](https://github.com/bytecodealliance/wasmtime) WebAssembly runtime.

## Usage

```rust
use wasmtime_provider::WasmtimeEngineProviderBuilder;
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

  let engine = WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes)
    .build()?;
  let host = WapcHost::new(Box::new(engine), Some(Box::new(host_callback)))?;

  let res = host.call("ping", b"payload bytes")?;
  assert_eq!(res, b"payload bytes");

  Ok(())
}
```

### `async` Support

The `async` feature enables the usage of this provider inside of an `async` context.

**Note:** this feature relies on the tokio runtime.

Check the [`WasmtimeEngineProviderAsync`] for more details.

### Creating a new instance

The [`WasmtimeEngineProviderBuilder`] is used to create new instances of [`WasmtimeEngineProvider`]
and [`WasmtimeEngineProviderAsync`].

Fresh instances of the engines can be created by using pre-initialized instances
like [`WasmtimeEngineProviderPre`] and [`WasmtimeEngineProviderAsyncPre`].

## Examples

### Running ping demo

```custom,{.language-bash}
cargo run -p wasmtime-provider \
    --example wasmtime-demo \
    ./wasm/crates/wasm-basic/build/wasm_basic.wasm \
    ping "hi"
```

### Running codec and module hotswapping demo

```custom,{.language-bash}
cargo run -p wasmtime-provider \
    --example wasmtime-hash-mreplace \
    AlexName
```

## See also

- [wasm3-provider](https://crates.io/crates/wasm3-provider)
