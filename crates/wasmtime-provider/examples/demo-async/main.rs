use std::time::Instant;

use wapc::WapcHostAsync;
use wasmtime_provider::WasmtimeEngineProviderBuilder;

async fn host_callback(
  id: u64,
  bd: String,
  ns: String,
  op: String,
  payload: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  println!(
    "Guest {} invoked '{}->{}:{}' on the host with a payload of '{}'",
    id,
    bd,
    ns,
    op,
    ::std::str::from_utf8(&payload).unwrap()
  );
  Ok(vec![])
}

#[tokio::main]
pub async fn main() -> Result<(), wapc::errors::Error> {
  env_logger::init();
  let n = Instant::now();
  let file = &std::env::args()
    .nth(1)
    .expect("WASM file should be passed as the first CLI parameter");
  let func = &std::env::args()
    .nth(2)
    .expect("waPC guest function to call should be passed as the second CLI parameter");
  let payload = &std::env::args()
    .nth(3)
    .expect("The string payload to send should be passed as the third CLI parameter");

  let module_bytes = std::fs::read(file).expect("WASM could not be read");
  let engine = WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes)
    .build_async()?;

  let callback: Box<wapc::HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
    let fut = host_callback(id, bd, ns, op, payload);
    Box::pin(fut)
  });

  let host = WapcHostAsync::new(Box::new(engine), Some(callback)).await?;

  println!("Calling guest (wasm) function '{}'", func);
  let res = host.call(func, payload.to_owned().as_bytes()).await?;
  println!("Result - {}", ::std::str::from_utf8(&res).unwrap());
  println!("Elapsed - {}ms", n.elapsed().as_millis());
  Ok(())
}
