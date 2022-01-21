use wapc::CallResult;
use wapc_guest as wapc;

#[no_mangle]
pub fn wapc_init() {
  wapc::register_function("ping", ping);
}

fn ping(msg: &[u8]) -> CallResult {
  // Note how this uses println!() directly vs the non-wasi sample which logs
  // via the host and console_log()
  println!(
    "IN_WASI: Received request for `ping` operation with payload : {}",
    std::str::from_utf8(msg).unwrap()
  );
  let _res = wapc::host_call("binding", "sample:namespace", "pong", msg)?;
  Ok(msg.to_vec())
}
