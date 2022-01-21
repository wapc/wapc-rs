# waPC messagepack codec

![crates.io](https://img.shields.io/crates/v/wapc-codec.svg)
![license](https://img.shields.io/crates/l/wapc-codec.svg)

This crates contains common serialization and deserialization methods for communicating in and out of waPC modules.

**_waPC does not require MessagePack_** but it does require a communication contract between hosts and guests. The waPC CLI code generator uses this crate but you are free to use what you want.

## Example

The following is a simple example of synchronous, bi-directional procedure calls between a WebAssembly host runtime and the guest module.

```rust
use serde::{Deserialize, Serialize};
use wapc_codec::messagepack::{deserialize, serialize};

#[derive(Deserialize, Serialize, Debug)]
struct Person {
  first_name: String,
  last_name: String,
  age: u8,
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
  let person = Person {
    first_name: "Samuel".to_owned(),
    last_name: "Clemens".to_owned(),
    age: 49,
  };

  println!("Original : {:?}", person);

  let bytes = serialize(&person)?;

  println!("Serialized messagepack bytes: {:?}", bytes);

  let round_trip: Person = deserialize(&bytes)?;

  println!("Deserialized : {:?}", round_trip);

  Ok(())
}
```
