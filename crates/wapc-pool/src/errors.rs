//! Library-specific error types and utility functions

/// Error type for this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// Error returned when we failed to receive a return from a worker.
  #[error("Request failed: {0}")]
  RequestFailed(String),

  /// Error returned when trying to shutdown a pool that's uninitialized or already shut down.
  #[error("No pool available. Have you initialized the HostPool or already shut it down?")]
  NoPool,
}

impl From<Error> for wapc::errors::Error {
  fn from(e: Error) -> Self {
    wapc::errors::Error::General(e.to_string())
  }
}

#[cfg(test)]
mod tests {
  #[allow(dead_code)]
  fn needs_sync_send<T: Send + Sync>() {}

  #[test]
  fn assert_sync_send() {
    needs_sync_send::<super::Error>();
  }
}
