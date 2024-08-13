use serde::{Deserialize, Serialize};
use wapc::{HostCallbackAsync, WapcHostAsync};
use wapc_codec::messagepack::{deserialize, serialize};
use wasmtime_provider::WasmtimeEngineProviderBuilder;

//simple struct to pass to wasm module and calc hash inside
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
struct PersonSend {
  first_name: String,
}
// recv struct
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
struct PersonHashedRecv {
  first_name: String,
  hash: u64,
}

const WAPC_FUNCTION_NAME: &str = "serdes_example";

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
    hex::encode(payload)
  );
  Ok(vec![])
}

#[tokio::main]
pub async fn main() -> Result<(), wapc::errors::Error> {
  env_logger::init();

  println!("Starting demo");

  let name = &std::env::args().nth(1).expect("pass some name to serde");

  let module_bytes1 = std::fs::read("../../wasm/crates/wasm-calc-hash/module1/build/module1_hash.wasm")
    .expect("WASM module 1 could not be read, run example from wasmtime-provider folder"); // read module 1
  let module_bytes2 = std::fs::read("../../wasm/crates/wasm-calc-hash/module2/build/module2_hash.wasm")
    .expect("WASM module 2 could not be read, run example from wasmtime-provider folder"); // read module 2
  assert_ne!(module_bytes1, module_bytes2); // test modules binaries not equal

  let engine = WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes1)
    .build_async()?;

  let callback: Box<HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
    let fut = host_callback(id, bd, ns, op, payload);
    Box::pin(fut)
  });

  let host = WapcHostAsync::new(Box::new(engine), Some(callback)).await?;

  println!("Calling guest (wasm) function: {}", WAPC_FUNCTION_NAME);
  // supply person struct
  let person = PersonSend {
    first_name: name.clone(),
  };
  let serbytes: Vec<u8> = serialize(&person).unwrap(); // serialize
  let encoded = hex::encode(serbytes.clone()); // examine
  println!("serialized message: {}", encoded);
  println!("calling wasm guest function to process text [{}]", name);
  println!("---------------CALLING MAIN MODULE------------------");
  let res = host.call(WAPC_FUNCTION_NAME, &serbytes).await?;
  let recv_struct: PersonHashedRecv = deserialize(&res).unwrap();
  println!("Deserialized : {:?}", recv_struct);

  println!("---------------REPLACING MODULE------------------");
  host.replace_module(&module_bytes2).await.unwrap(); // hotswapping

  let serbytes2: Vec<u8> = serialize(&person).unwrap();
  let encoded2 = hex::encode(serbytes2.clone());
  println!("serialized message: {}", encoded2);
  println!("calling wasm guest function to process text [{}]", name);
  println!("Calling guest (wasm) function: {}", WAPC_FUNCTION_NAME);
  let res2 = host.call(WAPC_FUNCTION_NAME, &serbytes2).await?; //calling
  let recv_struct2: PersonHashedRecv = deserialize(&res2).unwrap();
  println!("Deserialized : {:?}", recv_struct2);

  assert_ne!(recv_struct, recv_struct2);

  Ok(())
}
