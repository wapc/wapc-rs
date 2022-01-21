//! Glob imports for common guest module development

pub use crate::console_log;
pub use crate::host_call;
pub use crate::register_function;
pub use crate::CallResult;
pub use crate::HandlerResult;

#[cfg(feature = "codec")]
pub use wapc_codec::messagepack;
