/// This crate's generic Error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Error returned from the wasm3 rust wrapper.
  #[error("WASM3: {0}")]
  Wasm3(String),
}

impl From<wasm3::error::Error> for Error {
  fn from(e: wasm3::error::Error) -> Self {
    Error::Wasm3(e.to_string())
  }
}

// `wasm3`'s error type isn't Send or Sync since it contains a raw
// pointer. This trait is to normalize `Result`'s coming from wasm3 into ones that
// are easier to manage.
pub(crate) trait SendSyncResult<T> {
  fn to_wapc(self) -> Result<T, Error>;
}

impl<T> SendSyncResult<T> for Result<T, wasm3::error::Error> {
  fn to_wapc(self) -> Result<T, Error> {
    self.map_err(|e| e.into())
  }
}
