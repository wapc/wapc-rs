use std::fs::read;

use wapc::{errors, WapcHost};
use wapc_codec::messagepack::{deserialize, serialize};

#[test]
fn runs_wapc_guest() -> Result<(), errors::Error> {
  let buf = read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm")?;

  let engine = wasm3_provider::Wasm3EngineProvider::new(&buf);
  let guest = WapcHost::new(Box::new(engine), Some(Box::new(move |_a, _b, _c, _d, _e| Ok(vec![]))))?;

  let callresult = guest.call("echo", &serialize("hello world").unwrap())?;
  let result: String = deserialize(&callresult).unwrap();
  assert_eq!(result, "hello world");
  Ok(())
}
