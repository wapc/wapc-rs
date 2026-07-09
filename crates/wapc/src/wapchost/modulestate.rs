use log::info;
use parking_lot::RwLock;

use crate::{HostCallback, Invocation};

#[derive(Default)]
/// Module state is essentially a 'handle' that is passed to a runtime engine to allow it
/// to read and write relevant data as different low-level functions are executed during
/// a waPC conversation
pub struct ModuleState {
  pub(super) guest_request: RwLock<Option<Invocation>>,
  pub(super) guest_response: RwLock<Option<Vec<u8>>>,
  pub(super) host_response: RwLock<Option<Vec<u8>>>,
  pub(super) guest_error: RwLock<Option<String>>,
  pub(super) host_error: RwLock<Option<String>>,
  pub(super) host_callback: Option<Box<HostCallback>>,
  pub(super) id: u64,
}

impl ModuleState {
  pub(crate) fn new(host_callback: Option<Box<HostCallback>>, id: u64) -> ModuleState {
    ModuleState {
      host_callback,
      id,
      guest_request: RwLock::new(None),
      guest_response: RwLock::new(None),
      host_response: RwLock::new(None),
      guest_error: RwLock::new(None),
      host_error: RwLock::new(None),
    }
  }
}

impl ModuleState {
  /// Retrieves the value, if any, of the current guest request
  pub fn get_guest_request(&self) -> Option<Invocation> {
    self.guest_request.read().clone()
  }

  /// Retrieves the value of the current host response
  #[deprecated(
    note = "This clones the whole host response buffer. Prefer `host_response_len()` and/or \
            `with_host_response()`, which avoid the clone."
  )]
  pub fn get_host_response(&self) -> Option<Vec<u8>> {
    self.host_response.read().clone()
  }

  /// Returns the length of the current host response, without cloning it.
  ///
  /// This exists to avoid the double-clone previously incurred by calling
  /// `get_host_response()`.
  pub fn host_response_len(&self) -> usize {
    self.host_response.read().as_ref().map_or(0, |v| v.len())
  }

  /// Runs `f` with a borrowed view of the current host response, without
  /// cloning it. Returns `None` if there is no host response set.
  ///
  /// This exists to avoid the double-clone previously incurred by calling
  /// `get_host_response()`.
  pub fn with_host_response<R>(&self, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
    self.host_response.read().as_ref().map(|v| f(v.as_slice()))
  }

  /// Sets a value indicating that an error occurred inside the execution of a guest call
  pub fn set_guest_error(&self, error: String) {
    *self.guest_error.write() = Some(error);
  }

  /// Sets the value indicating the response data from a guest call
  pub fn set_guest_response(&self, response: Vec<u8>) {
    *self.guest_response.write() = Some(response);
  }

  /// Queries the value of the current guest response
  pub fn get_guest_response(&self) -> Option<Vec<u8>> {
    self.guest_response.read().clone()
  }

  /// Queries the value of the current host error
  pub fn get_host_error(&self) -> Option<String> {
    self.host_error.read().clone()
  }

  /// Invoked when the guest module wishes to make a call on the host
  pub fn do_host_call(
    &self,
    binding: &str,
    namespace: &str,
    operation: &str,
    payload: &[u8],
  ) -> Result<i32, Box<dyn std::error::Error>> {
    let id = {
      *self.host_response.write() = None;
      *self.host_error.write() = None;
      self.id
    };
    let result = self.host_callback.as_ref().map_or_else(
      || Err("Missing host callback function!".into()),
      |f| f(id, binding, namespace, operation, payload),
    );
    Ok(match result {
      Ok(v) => {
        *self.host_response.write() = Some(v);
        1
      }
      Err(e) => {
        *self.host_error.write() = Some(format!("{e}"));
        0
      }
    })
  }

  /// Invoked when the guest module wants to write a message to the host's `stdout`
  pub fn do_console_log(&self, msg: &str) {
    info!("Guest module {}: {}", self.id, msg);
  }
}

impl std::fmt::Debug for ModuleState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ModuleState")
      .field("guest_request", &self.guest_request)
      .field("guest_response", &self.guest_response)
      .field("host_response", &self.host_response)
      .field("guest_error", &self.guest_error)
      .field("host_error", &self.host_error)
      .field("host_callback", &self.host_callback.as_ref().map(|_| Some("Some(Fn)")))
      .field("id", &self.id)
      .finish()
  }
}

#[cfg(test)]
mod tests {
  use rstest::rstest;

  use super::*;

  fn state_with_callback(response: Vec<u8>) -> ModuleState {
    let callback: Box<HostCallback> =
      Box::new(move |_id, _binding, _namespace, _operation, _payload| Ok(response.clone()));
    ModuleState::new(Some(callback), 0)
  }

  #[rstest]
  #[case::no_host_call_made(None, false, None, None)]
  #[case::successful_host_call(Some(vec![1, 2, 3, 4, 5]), true, Some(1), Some(vec![1, 2, 3, 4, 5]))]
  #[case::failed_host_call_missing_callback(None, true, Some(0), None)]
  fn host_response_accessors(
    #[case] callback_response: Option<Vec<u8>>,
    #[case] invoke_call: bool,
    #[case] expected_return_code: Option<i32>,
    #[case] expected_response: Option<Vec<u8>>,
  ) {
    let state = callback_response.map_or_else(|| ModuleState::new(None, 0), state_with_callback);

    if invoke_call {
      let result = state.do_host_call("binding", "namespace", "operation", b"payload");
      assert_eq!(Some(result.unwrap()), expected_return_code);
    }

    match expected_response {
      Some(bytes) => {
        assert_eq!(state.host_response_len(), bytes.len());
        assert_eq!(state.with_host_response(|b| b.to_vec()), Some(bytes));
      }
      None => {
        assert_eq!(state.host_response_len(), 0);
        assert_eq!(state.with_host_response(|b| b.to_vec()), None);
      }
    }
  }

  #[test]
  #[allow(deprecated)]
  fn get_host_response_still_works_for_backward_compatibility() {
    let expected = vec![1, 2, 3, 4, 5];
    let state = state_with_callback(expected.clone());

    assert_eq!(state.get_host_response(), None);

    let result = state.do_host_call("binding", "namespace", "operation", b"payload");
    assert_eq!(result.unwrap(), 1);

    assert_eq!(state.get_host_response(), Some(expected));
  }
}
