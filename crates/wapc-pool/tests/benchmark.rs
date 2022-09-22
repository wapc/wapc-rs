use std::fs::read;
use std::time::{Duration, Instant};

use futures::future::try_join_all;
use wapc::{errors, WapcHost};
use wapc_codec::messagepack::{deserialize, serialize};
use wapc_pool::HostPoolBuilder;

// Naive benchmark test to make sure this is actually faster.
#[test_log::test(tokio::test)]
async fn benchmark() -> Result<(), errors::Error> {
  let buf = read("../../wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm")?;

  let num_threads: u32 = 10;
  let num_calls: u32 = 100;

  let engine = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&buf)
    .build()?;
  let pool = HostPoolBuilder::new()
    .name("wasmtime-test")
    .factory(move || {
      let engine = engine.clone();
      WapcHost::new(Box::new(engine), None).unwrap()
    })
    .min_threads(num_threads as _)
    .max_threads(num_threads as _)
    .build();

  println!("Waiting for threads to spin up");
  std::thread::sleep(Duration::from_millis(3000));

  let hello = "hello world".to_owned();

  // Prime all the engines
  println!("Priming WASM engines");
  let priming_futs = (0..num_threads).map(|_| pool.call("echo", serialize(&hello).unwrap()));
  futures::future::join_all(priming_futs).await;
  println!("Priming finished");

  // Establish a baseline
  let now = Instant::now();
  let callresult = pool.call("echo", serialize(&hello).unwrap()).await?;
  let base_duration = now.elapsed();
  println!("Base duration of one call is {}μs", base_duration.as_micros());

  let result: String = deserialize(&callresult).unwrap();
  assert_eq!(result, "hello world");

  let now = Instant::now();
  let result =
    try_join_all((0..num_calls).map(|num| pool.call("echo", serialize(&format!("hello world: {}", num)).unwrap())))
      .await;
  let duration_all = now.elapsed();

  println!(
    "{} calls across {} threads took {}μs",
    num_calls,
    num_threads,
    duration_all.as_micros()
  );
  let per_thread_margin = 75;
  let buffer = Duration::from_micros((per_thread_margin * (num_calls / num_threads)) as _);
  let expected_max = base_duration * (num_calls / num_threads);

  assert!(result.is_ok());
  let returns = result.unwrap();

  // Assert correct ordering
  for (i, bytes) in returns.iter().enumerate() {
    let result: String = deserialize(bytes).unwrap();
    assert_eq!(result, format!("hello world: {}", i));
  }

  println!(
    "Expecting {} calls to take around {}μs - {}μs",
    num_calls,
    expected_max.as_micros(),
    (expected_max + buffer).as_micros()
  );

  Ok(())
}
