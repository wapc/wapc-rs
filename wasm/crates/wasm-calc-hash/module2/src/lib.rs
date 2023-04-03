#![allow(unused_imports)]
use serde::{Deserialize, Serialize};

use wapc_codec::messagepack::{deserialize, serialize};
use wapc_guest as wapc;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
struct PersonSend {
  first_name: String,
}
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
struct PersonHashRecv {
  first_name: String,
  hash: u64,
}

#[no_mangle]
pub fn wapc_init() {
  wapc::register_function("serdes_example", serdes_example);
}
//just return hardcoded
fn serdes_example(msg: &[u8]) -> wapc::CallResult {
  wapc::console_log(&String::from(
    "IN_WASM: Received request for `serdes_and_hash`: MODULE 2",
  ));
  let inputstruct: PersonSend = deserialize(msg)?; // deser Name
  let msg_back = PersonHashRecv {
    first_name: inputstruct.first_name,
    hash: 42_u64,
  };
  let bytes = serialize(&msg_back)?;
  let _res = wapc::host_call("binding", "sample:namespace", "serdes_and_hash", &bytes)?;
  Ok(bytes.to_vec())
}
