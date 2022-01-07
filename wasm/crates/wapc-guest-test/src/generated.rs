extern crate rmp_serde as rmps;
use rmps::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[cfg(feature = "guest")]
extern crate wapc_guest as guest;
#[cfg(feature = "guest")]
use guest::prelude::*;

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
      &serialize(input)?,
    )
    .map(|vec| {
      let resp = deserialize::<String>(vec.as_ref()).unwrap();
      resp
    })
    .map_err(|e| e.into())
  }
}

#[cfg(feature = "guest")]
pub struct Handlers {}

#[cfg(feature = "guest")]
impl Handlers {
  pub fn register_echo(f: fn(String) -> HandlerResult<String>) {
    *ECHO.write().unwrap() = Some(f);
    register_function(&"echo", echo_wrapper);
  }
}

#[cfg(feature = "guest")]
lazy_static::lazy_static! {
static ref ECHO: std::sync::RwLock<Option<fn(String) -> HandlerResult<String>>> = std::sync::RwLock::new(None);
}

#[cfg(feature = "guest")]
fn echo_wrapper(input_payload: &[u8]) -> CallResult {
  let input = deserialize::<String>(input_payload)?;
  let lock = ECHO.read().unwrap().unwrap();
  let result = lock(input)?;
  serialize(result)
}

/// The standard function for serializing codec structs into a format that can be
/// used for message exchange between actor and host. Use of any other function to
/// serialize could result in breaking incompatibilities.
pub fn serialize<T>(
  item: T,
) -> ::std::result::Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>
where
  T: Serialize,
{
  let mut buf = Vec::new();
  item.serialize(&mut Serializer::new(&mut buf).with_struct_map())?;
  Ok(buf)
}

/// The standard function for de-serializing codec structs from a format suitable
/// for message exchange between actor and host. Use of any other function to
/// deserialize could result in breaking incompatibilities.
pub fn deserialize<'de, T: Deserialize<'de>>(
  buf: &[u8],
) -> ::std::result::Result<T, Box<dyn std::error::Error + Send + Sync>> {
  let mut de = Deserializer::new(Cursor::new(buf));
  match Deserialize::deserialize(&mut de) {
    Ok(t) => Ok(t),
    Err(e) => Err(format!("Failed to de-serialize: {}", e).into()),
  }
}
