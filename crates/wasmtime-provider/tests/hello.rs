use std::fs::read;

use wapc::{errors::Error, WapcHost};

fn create_guest(path: &str) -> Result<WapcHost, Error> {
  let buf = read(path)?;
  cfg_if::cfg_if! {
    if #[cfg(feature = "cache")] {
      let engine =
      wasmtime_provider::WasmtimeEngineProvider::new_with_cache(&buf, None, None).unwrap();
    } else {
      let engine =
      wasmtime_provider::WasmtimeEngineProvider::new(&buf, None).unwrap();
    }
  }
  WapcHost::new(
    Box::new(engine),
    Some(Box::new(move |_a, _b, _c, _d, _e| Ok(vec![]))),
  )
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
#[cfg(feature = "wasi")]
fn runs_hello_wasi() -> Result<(), Error> {
  let guest = create_guest("../../wasm/crates/wasi-basic/build/wasi_basic.wasm")?;
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
#[cfg(feature = "wasi")]
fn runs_hello_tinygo() -> Result<(), Error> {
  let guest = create_guest("../../wasm/hello_tinygo.wasm")?;

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
