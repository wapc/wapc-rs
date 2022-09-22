/// A convenience wrapper of `Result` that relies on
/// [`wasmtime_provider::errors::Error`](crate::errors::Error)
/// to hold errors
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// This crate's Error type
#[derive(thiserror::Error, Debug)]
pub enum Error {
  /// Wasmtime initialization failed
  #[error("Initialization failed: {0}")]
  InitializationFailed(Box<dyn std::error::Error + Send + Sync>),

  /// Wasmtime initialization failed
  #[error("Initialization failed: {0} init interrupted, execution deadline exceeded")]
  InitializationFailedTimeout(String),

  /// The guest call function was not exported by the guest.
  #[error("Guest call function (__guest_call) not exported by wasm module.")]
  GuestCallNotFound,

  /// Error originating when wasi feature is disabled, but the user provides wasi related params
  #[error("WASI related parameter provided, but wasi feature is disabled")]
  WasiDisabled,

  /// Error originating when wasi context initialization fails
  #[error("WASI context initialization failed: {0}")]
  WasiInitCtxError(String),

  /// Error caused when a host function cannot be registered into a wasmtime::Linker
  #[error("Linker cannot register function '{func}': {err}")]
  LinkerFuncDef {
    /// wasm function that was being defined
    func: String,
    /// error reported
    err: String,
  },

  /// Error caused by an invalid configuration of the [`WasmtimeEngineProviderBuilder`]
  #[error("Invalid WasmtimeEngineProviderBuilder configuration: {0}")]
  BuilderInvalidConfig(String),

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
