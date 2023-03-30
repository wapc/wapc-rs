use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

fn serdes_example(msg: &[u8]) -> wapc::CallResult {
    wapc::console_log(&format!(
        "IN_WASM: Received request for `serdes_and_hash`: MODULE 1",
    ));
    let inputstruct: PersonSend = deserialize(&msg)?; // deser Name
    let mut hasher = DefaultHasher::new();
    inputstruct.first_name.hash(&mut hasher); // hashing
    let calced_hash = hasher.finish();
    let msg_back = PersonHashRecv {
        first_name: inputstruct.first_name,
        hash: calced_hash,
    };
    let bytes = serialize(&msg_back)?;
    let _res = wapc::host_call("binding", "sample:namespace", "serdes_and_hash", &bytes)?;
    Ok(bytes.to_vec())
}
