/// This crate's Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
  /// WASMTime initialization failed
  #[error("Initialization failed: {0}")]
  InitializationFailed(Box<dyn std::error::Error + Send + Sync>),

  /// The guest call function was not exported by the guest.
  #[error("Guest call function (__guest_call) not exported by wasm module.")]
  GuestCallNotFound,

  /// Error originating when wasi feature is disabled, but the user provides wasi related params
  #[error("WASI related parameter provided, but wasi feature is disabled")]
  WasiDisabled,

  /// Generic error
  // wasmtime uses `anyhow::Error` inside of its public API
  #[error(transparent)]
  Generic(#[from] anyhow::Error),
}

impl From<Error> for wapc::errors::Error {
  fn from(e: Error) -> Self {
    wapc::errors::Error::ProviderFailure(Box::new(e))
  }
}
