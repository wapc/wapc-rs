//! Serialization/Deserialization functions for transmitting data to waPC hosts and guests as MessagePack bytes.
//!
//!```
//! use serde::{Serialize, Deserialize};
//! use wapc_codec::messagepack::{serialize,deserialize};
//!
//! #[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
//! struct Person {
//!   first_name: String,
//!   last_name: String,
//!   age: u8,
//! }
//!
//! let person = Person {
//!   first_name: "Samuel".to_owned(),
//!   last_name: "Clemens".to_owned(),
//!   age: 49,
//! };
//!
//! println!("Original : {:?}", person);
//!
//! let bytes = serialize(&person).unwrap();
//!
//! println!("Serialized messagepack bytes: {:?}", bytes);
//!
//! let round_trip: Person = deserialize(&bytes).unwrap();
//!
//! assert_eq!(person, round_trip);
//!```

use std::io::Cursor;

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

use crate::errors;

/// [`serialize()`] serializes a structure into MessagePack bytes.
pub fn serialize<T>(item: T) -> Result<Vec<u8>, errors::Error>
where
  T: Serialize,
{
  let mut buf = Vec::new();
  item
    .serialize(&mut Serializer::new(&mut buf).with_struct_map())
    .map_err(|e| errors::new(errors::ErrorKind::MessagePackSerialization(e)))?;
  Ok(buf)
}

/// [`deserialize()`] converts a MessagePack-formatted list of bytes into the target data structure.
pub fn deserialize<'de, T: Deserialize<'de>>(buf: &[u8]) -> Result<T, errors::Error> {
  let mut de = Deserializer::new(Cursor::new(buf));
  Deserialize::deserialize(&mut de).map_err(|e| errors::new(errors::ErrorKind::MessagePackDeserialization(e)))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
  struct Person {
    first_name: String,
    last_name: String,
    age: u8,
  }

  #[test]
  fn test() {
    let person = Person {
      first_name: "Samuel".to_owned(),
      last_name: "Clemens".to_owned(),
      age: 49,
    };

    println!("Original : {:?}", person);

    let bytes = serialize(&person).unwrap();

    println!("Serialized messagepack bytes: {:?}", bytes);

    let round_trip: Person = deserialize(&bytes).unwrap();

    assert_eq!(person, round_trip);
  }
}
