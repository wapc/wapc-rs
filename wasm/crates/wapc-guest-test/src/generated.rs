#[cfg(feature = "guest")]
use wapc_guest::prelude::*;

#[cfg(feature = "guest")]
pub struct Host {
  binding: String,
}

#[cfg(feature = "guest")]
impl Default for Host {
  fn default() -> Self {
    Host {
      binding: "default".to_string(),
    }
  }
}

/// Creates a named host binding
#[cfg(feature = "guest")]
pub fn host(binding: &str) -> Host {
  Host {
    binding: binding.to_string(),
  }
}

/// Creates the default host binding
#[cfg(feature = "guest")]
pub fn default() -> Host {
  Host::default()
}

#[cfg(feature = "guest")]
impl Host {
  pub fn echo(&self, input: String) -> HandlerResult<String> {
    host_call(
      &self.binding,
      "example:interface",
      "echo",
      &messagepack::serialize(input)?,
    )
    .map(|vec| messagepack::deserialize::<String>(vec.as_ref()).unwrap())
  }
}

#[cfg(feature = "guest")]
pub struct Handlers {}

#[cfg(feature = "guest")]
impl Handlers {
  pub fn register_echo(f: fn(String) -> HandlerResult<String>) {
    *ECHO.write().unwrap() = Some(f);
    register_function("echo", echo_wrapper);
  }
}

#[cfg(feature = "guest")]
static ECHO: once_cell::sync::Lazy<std::sync::RwLock<Option<fn(String) -> HandlerResult<String>>>> =
  once_cell::sync::Lazy::new(|| std::sync::RwLock::new(None));

#[cfg(feature = "guest")]
fn echo_wrapper(input_payload: &[u8]) -> CallResult {
  let input = messagepack::deserialize::<String>(input_payload)?;
  let lock = ECHO.read().unwrap().unwrap();
  let result = lock(input)?;
  Ok(messagepack::serialize(result)?)
}
