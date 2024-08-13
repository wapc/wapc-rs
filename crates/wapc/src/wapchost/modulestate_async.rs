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
  pub async fn get_host_response(&self) -> Option<Vec<u8>> {
    self.host_response.read().await.clone()
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
        *self.host_error.write().await = Some(format!("{}", e));
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
