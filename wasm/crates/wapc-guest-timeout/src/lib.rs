use std::{thread, time};
use wapc_guest::prelude::*;

#[no_mangle]
pub extern "C" fn wapc_init() {
  register_function("sleep", sleep);
}

fn sleep(payload: &[u8]) -> CallResult {
  let timeout_str = std::str::from_utf8(payload)?;
  let timeout: u64 = timeout_str.parse()?;

  let sleep_duration = time::Duration::from_secs(timeout.into());
  thread::sleep(sleep_duration);

  Ok(format!("slept for {} seconds", timeout).as_bytes().to_owned())
}
