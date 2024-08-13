use serde::{Deserialize, Serialize};

use wapc::{errors, WapcHost};
use wapc_codec::messagepack::{deserialize, serialize};

#[cfg(feature = "async")]
use wapc::WapcHostAsync;

const WAPC_FUNCTION_NAME: &str = "serdes_example";

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

#[test]
fn runs_wasm_calc_hash() -> Result<(), errors::Error> {
  let module_bytes1 = std::fs::read("../../wasm/crates/wasm-calc-hash/module1/build/module1_hash.wasm")?;
  let module_bytes2 = std::fs::read("../../wasm/crates/wasm-calc-hash/module2/build/module2_hash.wasm")?;
  // test modules binaries not equal
  assert_ne!(module_bytes1, module_bytes2);

  let engine = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes1)
    .build()?;
  let host = WapcHost::new(
    Box::new(engine),
    Some(Box::new(move |_id, _bd, _ns, _op, _payload| Ok(vec![]))),
  )?;

  let name = "John Doe".to_string();

  // supply person struct
  let person = PersonSend {
    first_name: name.clone(),
  };
  let serbytes: Vec<u8> = serialize(&person).unwrap();

  let res = host.call(WAPC_FUNCTION_NAME, &serbytes)?;
  let recv_struct: PersonHashedRecv = deserialize(&res).unwrap();

  // hotswapping
  host.replace_module(&module_bytes2)?;

  let res2 = host.call(WAPC_FUNCTION_NAME, &serbytes)?;
  let recv_struct2: PersonHashedRecv = deserialize(&res2).unwrap();

  assert_ne!(recv_struct, recv_struct2);
  assert_eq!(recv_struct.first_name, name);
  assert_eq!(recv_struct2.first_name, name);

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
  Ok(vec![])
}

#[cfg(feature = "async")]
#[tokio::test]
async fn runs_wapc_guest_async() -> Result<(), errors::Error> {
  let module_bytes1 = std::fs::read("../../wasm/crates/wasm-calc-hash/module1/build/module1_hash.wasm")?;
  let module_bytes2 = std::fs::read("../../wasm/crates/wasm-calc-hash/module2/build/module2_hash.wasm")?;
  // test modules binaries not equal
  assert_ne!(module_bytes1, module_bytes2);

  let engine = wasmtime_provider::WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes1)
    .build_async()?;

  let host_callback: Box<wapc::HostCallbackAsync> = Box::new(move |id, bd, ns, op, payload| {
    let fut = host_callback_async(id, bd, ns, op, payload);
    Box::pin(fut)
  });

  let host = WapcHostAsync::new(Box::new(engine), Some(host_callback)).await?;

  let name = "John Doe".to_string();

  // supply person struct
  let person = PersonSend {
    first_name: name.clone(),
  };
  let serbytes: Vec<u8> = serialize(&person).unwrap();

  let res = host.call(WAPC_FUNCTION_NAME, &serbytes).await?;
  let recv_struct: PersonHashedRecv = deserialize(&res).unwrap();

  // hotswapping
  host.replace_module(&module_bytes2).await?;

  let res2 = host.call(WAPC_FUNCTION_NAME, &serbytes).await?;
  let recv_struct2: PersonHashedRecv = deserialize(&res2).unwrap();

  assert_ne!(recv_struct, recv_struct2);
  assert_eq!(recv_struct.first_name, name);
  assert_eq!(recv_struct2.first_name, name);

  Ok(())
}
