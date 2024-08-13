use std::fs::read;

use wapc::{errors, WapcHost};
use wapc_codec::messagepack::{deserialize, serialize};

#[cfg(feature = "async")]
use wapc::WapcHostAsync;

#[test]
fn runs_wapc_guest() -> Result<(), errors::Error> {
  let buf = read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm")?;

  let engine = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&buf)
    .build()?;
  let guest = WapcHost::new(
    Box::new(engine),
    Some(Box::new(move |_a, _b, _c, _d, _e| {
      panic!("host callback should never be called by wapc_guest_test::echo");
    })),
  )?;

  let callresult = guest.call("echo", &serialize("hello world").unwrap())?;
  let result: String = deserialize(&callresult).unwrap();
  assert_eq!(result, "hello world");
  Ok(())
}

#[cfg(feature = "async")]
async fn host_callback_async(
  _id: u64,
  _bd: String,
  _ns: String,
  _op: String,
  _payload: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  panic!("host callback should never be called by wapc_guest_test::echo");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn runs_wapc_guest_async() -> Result<(), errors::Error> {
  let buf = read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm")?;

  let engine = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&buf)
    .build_async()?;

  let host_callback: Box<wapc::HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
    let fut = host_callback_async(id, bd, ns, op, payload);
    Box::pin(fut)
  });

  let host = WapcHostAsync::new(Box::new(engine), Some(host_callback)).await?;

  let callresult = host.call("echo", &serialize("hello world").unwrap()).await?;
  let result: String = deserialize(&callresult).unwrap();
  assert_eq!(result, "hello world");
  Ok(())
}
