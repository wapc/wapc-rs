use std::fs::read;

use wapc::WapcHost;
use wapc_codec::messagepack::{deserialize, serialize};
use wapc_pool::HostPoolBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let buf = read("./wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm")?;

  let engine = wasmtime_provider::WasmtimeEngineProvider::new(&buf, None)?;

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
