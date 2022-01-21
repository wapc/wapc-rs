use std::fs::read;

use wapc::errors::Error;
use wapc::WapcHost;

fn create_guest(path: &str) -> Result<WapcHost, Error> {
  let buf = read(path)?;

  let engine = wasm3_provider::Wasm3EngineProvider::new(&buf);

  WapcHost::new(Box::new(engine), Some(Box::new(move |_a, _b, _c, _d, _e| Ok(vec![]))))
}

#[test]
fn runs_hello() -> Result<(), Error> {
  let guest = create_guest("../../wasm/crates/wasm-basic/build/wasm_basic.wasm")?;
  let payload = "this is a test";
  let callresult = guest.call("ping", payload.as_bytes())?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, payload);
  Ok(())
}

#[test]
fn runs_hello_as() -> Result<(), Error> {
  let guest = create_guest("../../wasm/hello_as.wasm")?;

  let callresult = guest.call("hello", b"this is a test")?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello");
  Ok(())
}

#[test]
fn runs_hello_zig() -> Result<(), Error> {
  let guest = create_guest("../../wasm/hello_zig.wasm")?;

  let callresult = guest.call("hello", b"this is a test")?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello, this is a test!");
  Ok(())
}
