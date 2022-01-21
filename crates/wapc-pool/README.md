# waPC Pool

![crates.io](https://img.shields.io/crates/v/wapc-pool.svg)
![license](https://img.shields.io/crates/l/wapc-pool.svg)

This crate implements a multi-threaded pool of waPC hosts. You'll typically use the `HostPoolBuilder` to create a `HostPool` and use `.call()` to initiate requests as you would on a standard `WapcHost`.

The `HostPool` has basic elasticity built in. Specify the minimum number of threads to start with and the maximum number to grow to. Give the pool a `max_wait` duration before starting a new worker and a `max_idle` duration to auto-kill workers above the minimum size.

```rust
use std::fs::read;

use wapc::WapcHost;
use wapc_codec::messagepack::{deserialize, serialize};
use wapc_pool::HostPoolBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let file = read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm")?;

  let engine = wasmtime_provider::WasmtimeEngineProvider::new(&file, None)?;

  let pool = HostPoolBuilder::new()
    .name("pool example")
    .factory(move || {
      let engine = engine.clone();
      WapcHost::new(Box::new(engine), None).unwrap()
    })
    .max_threads(5)
    .build();

  let bytes = pool.call("echo", serialize("Hello!")?).await?;

  let result: String = deserialize(&bytes)?;

  println!("Wasm module returned: {}", result);

  Ok(())
}
```
