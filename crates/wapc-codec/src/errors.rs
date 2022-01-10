//! # Errors
//!
//! This module generalizes errors for all the included codec functions.

use std::error::Error as StdError;
use std::fmt;

/// This crate's Error type
#[derive(Debug)]
pub struct Error(Box<ErrorKind>);

/// Create a new [Error] of the passed kind.
#[must_use]
pub fn new(kind: ErrorKind) -> Error {
  Error(Box::new(kind))
}

/// The kinds of errors this crate returns.
#[derive(Debug)]
pub enum ErrorKind {
  /// Error serializing into MessagePack bytes.
  #[cfg(feature = "messagepack")]
  MessagePackSerialization(rmp_serde::encode::Error),
  /// Error deserializing from MessagePack bytes.
  #[cfg(feature = "messagepack")]
  MessagePackDeserialization(rmp_serde::decode::Error),
}

impl StdError for Error {}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let errstr = match self.0.as_ref() {
      #[cfg(feature = "messagepack")]
      ErrorKind::MessagePackSerialization(e) => e.to_string(),
      #[cfg(feature = "messagepack")]
      ErrorKind::MessagePackDeserialization(e) => e.to_string(),
    };
    f.write_str(&errstr)
  }
}
