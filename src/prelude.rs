//! Glob imports for common guest module development

pub use crate::host_call;
pub use crate::wapc_handler;
pub use crate::Result;

pub type CallResult = ::std::result::Result<Vec<u8>, Box<dyn std::error::Error + Sync + Send>>;
