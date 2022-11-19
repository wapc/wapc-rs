use std::fs::read;

use wapc::errors::Error;
use wapc::WapcHost;

fn create_guest(path: &str) -> Result<WapcHost, Error> {
  let buf = read(path)?;
  cfg_if::cfg_if! {
    if #[cfg(feature = "cache")] {
        let builder = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
            .module_bytes(&buf).
            enable_cache(None);
    } else {
        let builder = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
            .module_bytes(&buf);
    }
  }
  let engine = builder.build().expect("Cannot create WebAssemblyEngineProvider");
  WapcHost::new(Box::new(engine), Some(Box::new(move |_a, _b, _c, _d, _e| Ok(vec![]))))
}

fn create_guest_from_builder(builder: &wasmtime_provider::WasmtimeEngineProviderBuilder) -> Result<WapcHost, Error> {
  let engine = builder.build().expect("Cannot create WebAssemblyEngineProvider");
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
#[cfg(feature = "wasi")]
fn runs_wapc_timeout() -> Result<(), Error> {
  let path = "../../wasm/crates/wapc-guest-timeout/build/wapc_guest_timeout.wasm";
  let module_bytes = read(path)?;
  let wapc_init_deadline = 100;
  let wapc_func_deadline = 2;

  let mut engine_conf = wasmtime::Config::default();
  engine_conf.epoch_interruption(true);
  let engine = wasmtime::Engine::new(&engine_conf).expect("cannot create wasmtime engine");

  let wapc_engine_builder = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes)
    .engine(engine.clone())
    .enable_epoch_interruptions(wapc_init_deadline, wapc_func_deadline);
  let guest = create_guest_from_builder(&wapc_engine_builder)?;

  std::thread::spawn(move || {
    // Starting timer thread
    let interval = std::time::Duration::from_secs(1);
    loop {
      std::thread::sleep(interval);
      engine.increment_epoch();
    }
  });

  let callresult = guest.call("sleep", b"1")?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "slept for 1 seconds");

  let callresult = guest.call("sleep", b"10");
  let err = callresult.expect_err("a timeout error was supposed to happen");
  assert_eq!(
    err.to_string(),
    "Guest call failure: guest code interrupted, execution deadline exceeded".to_string()
  );
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
