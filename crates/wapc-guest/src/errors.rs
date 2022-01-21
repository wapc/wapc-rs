// Copyright 2015-2020 Capital One Services, LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Errors
//!
//! This module contains types and utility functions for error handling

use std::error::Error as StdError;
use std::fmt;

/// This crate's Error type
#[derive(Debug)]
pub struct Error(ErrorKind);

/// Create a new [Error] of the passed kind.
#[must_use]
pub fn new(kind: ErrorKind) -> Error {
  Error(kind)
}

/// The kinds of errors this crate returns.
#[derive(Debug)]
pub enum ErrorKind {
  /// Error returned when a host call fails.
  HostError(Vec<u8>),
}

impl StdError for Error {}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.0 {
      ErrorKind::HostError(ref e) => write!(f, "Host error: {}", String::from_utf8_lossy(e)),
    }
  }
}
