use std::fs::read;

use wapc::errors::Error;
use wapc::WapcHost;

#[cfg(feature = "async")]
use wapc::WapcHostAsync;

const PAYLOAD: &str = "this is a test";

fn create_guest(path: &str, callback: Box<wapc::HostCallback>) -> Result<WapcHost, Error> {
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
  WapcHost::new(Box::new(engine), Some(callback))
}

#[cfg(feature = "wasi")]
fn create_guest_from_builder(builder: &wasmtime_provider::WasmtimeEngineProviderBuilder) -> Result<WapcHost, Error> {
  let engine = builder.build().expect("Cannot create WebAssemblyEngineProvider");
  WapcHost::new(Box::new(engine), Some(Box::new(move |_a, _b, _c, _d, _e| Ok(vec![]))))
}

fn host_callback_basic(
  _id: u64,
  bd: &str,
  ns: &str,
  op: &str,
  payload: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  assert_eq!(bd, "binding");
  assert_eq!(ns, "sample:namespace");
  assert_eq!(op, "pong");
  assert_eq!(payload, PAYLOAD.as_bytes());

  Ok(vec![])
}

fn host_callback_hello(
  _id: u64,
  bd: &str,
  ns: &str,
  op: &str,
  payload: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  assert_eq!(bd, "myBinding");
  assert_eq!(ns, "sample");
  assert_eq!(op, "hello");
  assert_eq!(payload, "Simon".as_bytes());

  Ok(vec![])
}

#[cfg(feature = "async")]
async fn host_callback_basic_async(
  _id: u64,
  bd: String,
  ns: String,
  op: String,
  payload: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  assert_eq!(bd, "binding".to_string());
  assert_eq!(ns, "sample:namespace".to_string());
  assert_eq!(op, "pong".to_string());
  assert_eq!(payload, PAYLOAD.as_bytes());

  Ok(vec![])
}

#[cfg(feature = "async")]
async fn host_callback_hello_async(
  _id: u64,
  bd: String,
  ns: String,
  op: String,
  payload: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  assert_eq!(bd, "myBinding".to_string());
  assert_eq!(ns, "sample".to_string());
  assert_eq!(op, "hello".to_string());
  assert_eq!(payload, "Simon".as_bytes());

  Ok(vec![])
}

#[cfg(feature = "async")]
async fn create_guest_async<F, Fut>(path: &str, callback: F) -> Result<WapcHostAsync, Error>
where
  F: Fn(u64, String, String, String, Vec<u8>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
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
  let engine = builder
    .build_async()
    .expect("Cannot create WebAssemblyEngineProviderAsync");

  let host_callback: Box<wapc::HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
    let fut = callback(id, bd, ns, op, payload);
    Box::pin(fut)
  });

  WapcHostAsync::new(Box::new(engine), Some(host_callback)).await
}

#[cfg(all(feature = "wasi", feature = "async"))]
async fn create_guest_async_from_builder<F, Fut>(
  builder: &wasmtime_provider::WasmtimeEngineProviderBuilder<'_>,
  callback: F,
) -> Result<WapcHostAsync, Error>
where
  F: Fn(u64, String, String, String, Vec<u8>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
  let engine = builder
    .build_async()
    .expect("Cannot create WebAssemblyEngineProviderAsync");
  let host_callback: Box<wapc::HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
    let fut = callback(id, bd, ns, op, payload);
    Box::pin(fut)
  });

  WapcHostAsync::new(Box::new(engine), Some(host_callback)).await
}

#[test]
fn runs_wasm_basic() -> Result<(), Error> {
  let guest = create_guest(
    "../../wasm/crates/wasm-basic/build/wasm_basic.wasm",
    Box::new(host_callback_basic),
  )?;
  let callresult = guest.call("ping", PAYLOAD.as_bytes())?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, PAYLOAD);
  Ok(())
}

#[cfg(feature = "async")]
#[tokio::test]
async fn runs_wasm_basic_async() -> Result<(), Error> {
  let guest = create_guest_async(
    "../../wasm/crates/wasm-basic/build/wasm_basic.wasm",
    host_callback_basic_async,
  )
  .await?;
  let callresult = guest.call("ping", PAYLOAD.as_bytes()).await?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, PAYLOAD);
  Ok(())
}

#[test]
#[cfg(feature = "wasi")]
fn runs_wasi_basic() -> Result<(), Error> {
  let guest = create_guest(
    "../../wasm/crates/wasi-basic/build/wasi_basic.wasm",
    Box::new(host_callback_basic),
  )?;
  let callresult = guest.call("ping", PAYLOAD.as_bytes())?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, PAYLOAD);
  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(all(feature = "wasi", feature = "async"))]
async fn runs_wasi_basic_async() -> Result<(), Error> {
  let guest = create_guest_async(
    "../../wasm/crates/wasi-basic/build/wasi_basic.wasm",
    host_callback_basic_async,
  )
  .await?;
  let callresult = guest.call("ping", PAYLOAD.as_bytes()).await?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, PAYLOAD);
  Ok(())
}

#[test]
fn runs_hello_as() -> Result<(), Error> {
  let guest = create_guest("../../wasm/hello_as.wasm", Box::new(host_callback_hello))?;

  let callresult = guest.call("hello", PAYLOAD.as_bytes())?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello");
  Ok(())
}

#[tokio::test]
#[cfg(feature = "async")]
async fn runs_hello_as_async() -> Result<(), Error> {
  let guest = create_guest_async("../../wasm/hello_as.wasm", host_callback_hello_async).await?;

  let callresult = guest.call("hello", PAYLOAD.as_bytes()).await?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello");
  Ok(())
}

#[test]
#[cfg(feature = "wasi")]
fn runs_hello_tinygo() -> Result<(), Error> {
  let guest = create_guest("../../wasm/hello_tinygo.wasm", Box::new(host_callback_hello))?;

  let callresult = guest.call("hello", PAYLOAD.as_bytes())?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello");
  Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[cfg(all(feature = "wasi", feature = "async"))]
async fn runs_hello_tinygo_async() -> Result<(), Error> {
  let guest = create_guest_async("../../wasm/hello_tinygo.wasm", host_callback_hello_async).await?;

  let callresult = guest.call("hello", PAYLOAD.as_bytes()).await?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello");
  Ok(())
}

#[test]
fn runs_hello_zig() -> Result<(), Error> {
  let guest = create_guest("../../wasm/hello_zig.wasm", Box::new(host_callback_hello))?;

  let callresult = guest.call("hello", PAYLOAD.as_bytes())?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello, this is a test!");
  Ok(())
}

#[tokio::test]
#[cfg(feature = "async")]
async fn runs_hello_zig_async() -> Result<(), Error> {
  let guest = create_guest_async("../../wasm/hello_zig.wasm", host_callback_hello_async).await?;

  let callresult = guest.call("hello", PAYLOAD.as_bytes()).await?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "Hello, this is a test!");
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

#[tokio::test(flavor = "multi_thread")]
#[cfg(all(feature = "wasi", feature = "async"))]
async fn runs_wapc_timeout_async() -> Result<(), Error> {
  let path = "../../wasm/crates/wapc-guest-timeout/build/wapc_guest_timeout.wasm";
  let module_bytes = read(path)?;
  let wapc_init_deadline = 100;
  let wapc_func_deadline = 2;

  let mut engine_conf = wasmtime::Config::default();
  engine_conf.epoch_interruption(true);
  engine_conf.async_support(true);
  let engine = wasmtime::Engine::new(&engine_conf).expect("cannot create wasmtime engine");

  let wapc_engine_builder = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes)
    .engine(engine.clone())
    .enable_epoch_interruptions(wapc_init_deadline, wapc_func_deadline);
  let guest = create_guest_async_from_builder(&wapc_engine_builder, host_callback_basic_async).await?;

  tokio::spawn(async move {
    // Starting timer thread
    let interval = std::time::Duration::from_secs(1);
    loop {
      tokio::time::sleep(interval).await;
      engine.increment_epoch();
    }
  });

  let callresult = guest.call("sleep", b"1").await?;
  let result = String::from_utf8_lossy(&callresult);
  assert_eq!(result, "slept for 1 seconds");

  let callresult = guest.call("sleep", b"10").await;
  let err = callresult.expect_err("a timeout error was supposed to happen");
  assert_eq!(
    err.to_string(),
    "Guest call failure: guest code interrupted, execution deadline exceeded".to_string()
  );
  Ok(())
}
