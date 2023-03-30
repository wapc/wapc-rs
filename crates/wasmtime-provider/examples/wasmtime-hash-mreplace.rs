#![allow(unused_imports)]
use console::{style, Term};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::fs::File;
use std::io::Write;
use std::{io::Read, time::Instant};
use wapc_codec::messagepack::{deserialize, serialize};

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

use wapc::WapcHost;
use wasmtime_provider::WasmtimeEngineProviderBuilder;

pub fn main() -> Result<(), wapc::errors::Error> {
  env_logger::init();
  println!("{}", style("starting app!").yellow());
  /* let file = &std::env::args()
    .nth(1)
    .expect("WASM file should be passed as the first CLI parameter");
  let file2 = &std::env::args()
    .nth(2)
    .expect("WASM file should be passed as the second CLI parameter");
  let func = &std::env::args()
    .nth(3)
    .expect("waPC guest function to call should be passed as the second CLI parameter");*/
  let name = &std::env::args().nth(1).expect("pass a name to serde");
  //--------------------------------
  let module_bytes1 = std::fs::read("../../wasm/crates/wasm-calc-hash/module1/build/module1_hash.wasm")
    .expect("WASM module 1 could not be read, run example from wasmtime-provider folder"); // read module 1
  let module_bytes2 = std::fs::read("../../wasm/crates/wasm-calc-hash/module2/build/module2_hash.wasm")
    .expect("WASM module 2 could not be read, run example from wasmtime-provider folder"); // read module 2
  let func = "serdes_example".to_string();
  //let module_bytes2 = std::fs::read(file2).expect("WASM could not be read"); // read module 2
  //from env
  let engine = WasmtimeEngineProviderBuilder::new()
    .module_bytes(&module_bytes1)
    .build()?;
  assert_ne!(module_bytes1, module_bytes2); // test modules binaries not equal
  let host = WapcHost::new(Box::new(engine), Some(Box::new(host_callback)))?;
  println!(
    "{} {}",
    style("Calling guest (wasm) function ").cyan(),
    style(&func).cyan()
  );
  // supply person struct
  let person = PersonSend {
    first_name: name.clone(),
  };
  let serbytes: SmallVec<[u8; 1024]> = serialize(&person).unwrap().into(); // serialize
  let encoded = hex::encode(serbytes.clone()); // examine
  println!("serialized message: {}", encoded);
  println!(
    "{} {}",
    style("calling wasm guest funcion with name").yellow(),
    name.clone()
  );
  let res = host.call(func.as_str(), &serbytes)?;
  let round_trip: PersonHashedRecv = deserialize(&res).unwrap();
  println!("{}", style("DESERIALIZED RESULT:").blue());
  println!("Deserialized : {:?}", round_trip);
  //-------------------------------
  println!("{}", style("---------------REPLACING MODULE------------------").red());
  std::mem::drop(module_bytes1); // just in case
  host.replace_module(&module_bytes2).unwrap(); // hotswapping
                                                // supply new person just in case
  let person2 = PersonSend {
    first_name: name.clone(),
  };
  let serbytes2: SmallVec<[u8; 1024]> = serialize(&person2).unwrap().into();
  let encoded2 = hex::encode(serbytes2.clone());
  println!("serialized message: {}", encoded2);
  println!("{} {name}", style("calling wasm guest funcion with name").yellow());

  let res2 = host.call("serdes_example", &serbytes2)?; //calling
  let round_trip2: PersonHashedRecv = deserialize(&res2).unwrap();
  println!("{}", style("DESERIALIZED RESULT:").blue());
  println!("Deserialized : {:?}", round_trip2);

  Ok(())
}

fn host_callback(
  id: u64,
  bd: &str,
  ns: &str,
  op: &str,
  payload: &[u8],
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
