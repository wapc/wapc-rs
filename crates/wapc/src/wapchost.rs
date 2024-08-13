mod host;
pub(crate) mod modulestate;

#[cfg(feature = "async")]
mod host_async;
#[cfg(feature = "async")]
pub(crate) mod modulestate_async;

pub(crate) mod traits;

use std::sync::atomic::AtomicU64;

use crate::{errors, HostCallback, Invocation};

pub(crate) static GLOBAL_MODULE_COUNT: AtomicU64 = AtomicU64::new(1);

pub(crate) type Result<T> = std::result::Result<T, errors::Error>;

pub use host::WapcHost;

#[cfg(feature = "async")]
pub use host_async::WapcHostAsync;
