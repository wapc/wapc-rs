use serde::{Deserialize, Serialize};
use wapc_codec::messagepack::{deserialize, serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
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
