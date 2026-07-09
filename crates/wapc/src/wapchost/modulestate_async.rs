use log::info;
use tokio::sync::RwLock;

use crate::{HostCallbackAsync, Invocation};

#[derive(Default)]
/// Module state is essentially a 'handle' that is passed to a runtime engine to allow it
/// to read and write relevant data as different low-level functions are executed during
/// a waPC conversation
///
/// This version of `ModuleState` is designed for use in async contexts
pub struct ModuleStateAsync {
  pub(crate) guest_request: RwLock<Option<Invocation>>,
  pub(crate) guest_response: RwLock<Option<Vec<u8>>>,
  pub(crate) host_response: RwLock<Option<Vec<u8>>>,
  pub(crate) guest_error: RwLock<Option<String>>,
  pub(crate) host_error: RwLock<Option<String>>,
  pub(crate) host_callback: Option<Box<HostCallbackAsync>>,
  pub(crate) id: u64,
}

impl ModuleStateAsync {
  pub(crate) fn new(host_callback: Option<Box<HostCallbackAsync>>, id: u64) -> ModuleStateAsync {
    ModuleStateAsync {
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

impl ModuleStateAsync {
  /// Retrieves the value, if any, of the current guest request
  pub async fn get_guest_request(&self) -> Option<Invocation> {
    self.guest_request.read().await.clone()
  }

  /// Retrieves the value of the current host response
  #[deprecated(
    note = "This clones the whole host response buffer. Prefer `host_response_len()` and/or \
            `with_host_response()`, which avoid the clone."
  )]
  pub async fn get_host_response(&self) -> Option<Vec<u8>> {
    self.host_response.read().await.clone()
  }

  /// Returns the length of the current host response, without cloning it.
  ///
  /// This exists to avoid the double-clone previously incurred by calling
  /// `get_host_response()`.
  pub async fn host_response_len(&self) -> usize {
    self.host_response.read().await.as_ref().map_or(0, |v| v.len())
  }

  /// Runs `f` with a borrowed view of the current host response, without
  /// cloning it. Returns `None` if there is no host response set.
  ///
  /// Note: `f` must be a plain (non-async) closure since it's called
  /// synchronously while holding the read guard.
  ///
  /// This exists to avoid the double-clone previously incurred by calling
  /// `get_host_response()`.
  pub async fn with_host_response<R>(&self, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
    self.host_response.read().await.as_ref().map(|v| f(v.as_slice()))
  }

  /// Sets a value indicating that an error occurred inside the execution of a guest call
  pub async fn set_guest_error(&self, error: String) {
    *self.guest_error.write().await = Some(error);
  }

  /// Sets the value indicating the response data from a guest call
  pub async fn set_guest_response(&self, response: Vec<u8>) {
    *self.guest_response.write().await = Some(response);
  }

  /// Queries the value of the current guest response
  pub async fn get_guest_response(&self) -> Option<Vec<u8>> {
    self.guest_response.read().await.clone()
  }

  /// Queries the value of the current host error
  pub async fn get_host_error(&self) -> Option<String> {
    self.host_error.read().await.clone()
  }

  /// Invoked when the guest module wishes to make a call on the host
  pub async fn do_host_call(
    &self,
    binding: String,
    namespace: String,
    operation: String,
    payload: Vec<u8>,
  ) -> Result<i32, Box<dyn std::error::Error>> {
    let id = {
      *self.host_response.write().await = None;
      *self.host_error.write().await = None;
      self.id
    };
    let result = match self.host_callback.as_ref() {
      None => Err("Missing host callback function!".into()),
      Some(f) => f(id, binding, namespace, operation, payload).await,
    };
    Ok(match result {
      Ok(v) => {
        *self.host_response.write().await = Some(v);
        1
      }
      Err(e) => {
        *self.host_error.write().await = Some(format!("{e}"));
        0
      }
    })
  }

  /// Invoked when the guest module wants to write a message to the host's `stdout`
  pub fn do_console_log(&self, msg: &str) {
    info!("Guest module {}: {}", self.id, msg);
  }
}

impl std::fmt::Debug for ModuleStateAsync {
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

  fn state_with_callback(response: Vec<u8>) -> ModuleStateAsync {
    let callback: Box<HostCallbackAsync> = Box::new(move |_id, _binding, _namespace, _operation, _payload| {
      let response = response.clone();
      Box::pin(async move { Ok(response) })
    });
    ModuleStateAsync::new(Some(callback), 0)
  }

  #[rstest]
  #[case::no_host_call_made(None, false, None, None)]
  #[case::successful_host_call(Some(vec![1, 2, 3, 4, 5]), true, Some(1), Some(vec![1, 2, 3, 4, 5]))]
  #[case::failed_host_call_missing_callback(None, true, Some(0), None)]
  #[tokio::test]
  async fn host_response_accessors(
    #[case] callback_response: Option<Vec<u8>>,
    #[case] invoke_call: bool,
    #[case] expected_return_code: Option<i32>,
    #[case] expected_response: Option<Vec<u8>>,
  ) {
    let state = callback_response.map_or_else(|| ModuleStateAsync::new(None, 0), state_with_callback);

    if invoke_call {
      let result = state
        .do_host_call(
          "binding".into(),
          "namespace".into(),
          "operation".into(),
          b"payload".to_vec(),
        )
        .await;
      assert_eq!(Some(result.unwrap()), expected_return_code);
    }

    match expected_response {
      Some(bytes) => {
        assert_eq!(state.host_response_len().await, bytes.len());
        assert_eq!(state.with_host_response(|b| b.to_vec()).await, Some(bytes));
      }
      None => {
        assert_eq!(state.host_response_len().await, 0);
        assert_eq!(state.with_host_response(|b| b.to_vec()).await, None);
      }
    }
  }

  #[tokio::test]
  #[allow(deprecated)]
  async fn get_host_response_still_works_for_backward_compatibility() {
    let expected = vec![1, 2, 3, 4, 5];
    let state = state_with_callback(expected.clone());

    assert_eq!(state.get_host_response().await, None);

    let result = state
      .do_host_call(
        "binding".into(),
        "namespace".into(),
        "operation".into(),
        b"payload".to_vec(),
      )
      .await;
    assert_eq!(result.unwrap(), 1);

    assert_eq!(state.get_host_response().await, Some(expected));
  }
}
