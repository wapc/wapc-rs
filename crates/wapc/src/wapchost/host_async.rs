use std::sync::{atomic::Ordering, Arc};

use tokio::sync::Mutex;

use crate::{
  wapchost::{
    errors, modulestate_async::ModuleStateAsync, traits::WebAssemblyEngineProviderAsync, Invocation, Result,
    GLOBAL_MODULE_COUNT,
  },
  HostCallbackAsync,
};

/// A WebAssembly host runtime for waPC-compliant modules that can be used in async contexts
///
/// Use an instance of this struct to provide a means of invoking procedure calls by
/// specifying an operation name and a set of bytes representing the opaque operation payload.
/// `WapcHostAsync` makes no assumptions about the contents or format of either the payload or the
/// operation name, other than that the operation name is a UTF-8 encoded string.
#[must_use]
pub struct WapcHostAsync {
  engine: Mutex<Box<dyn WebAssemblyEngineProviderAsync + Send>>,
  state: Arc<ModuleStateAsync>,
}

impl std::fmt::Debug for WapcHostAsync {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("WapcHostAsync").field("state", &self.state).finish()
  }
}

impl WapcHostAsync {
  /// Creates a new instance of a waPC-compliant host runtime paired with a given
  /// low-level engine provider
  pub async fn new(
    engine: Box<dyn WebAssemblyEngineProviderAsync + Send>,
    host_callback: Option<Box<HostCallbackAsync>>,
  ) -> Result<Self> {
    let id = GLOBAL_MODULE_COUNT.fetch_add(1, Ordering::SeqCst);

    let state = Arc::new(ModuleStateAsync::new(host_callback, id));

    let mh = WapcHostAsync {
      engine: Mutex::new(engine),
      state: state.clone(),
    };

    mh.initialize(state).await?;

    Ok(mh)
  }

  async fn initialize(&self, state: Arc<ModuleStateAsync>) -> Result<()> {
    match self.engine.lock().await.init(state).await {
      Ok(_) => Ok(()),
      Err(e) => Err(errors::Error::InitFailed(e.to_string())),
    }
  }

  /// Returns a reference to the unique identifier of this module. If a parent process
  /// has instantiated multiple `WapcHost`s, then the single static host callback function
  /// will contain this value to allow disambiguation of modules
  pub fn id(&self) -> u64 {
    self.state.id
  }

  /// Invokes the `__guest_call` function within the guest module as per the waPC specification.
  /// Provide an operation name and an opaque payload of bytes and the function returns a `Result`
  /// containing either an error or an opaque reply of bytes.
  ///
  /// It is worth noting that the _first_ time `call` is invoked, the WebAssembly module
  /// might incur a "cold start" penalty, depending on which underlying engine you're using. This
  /// might be due to lazy initialization or JIT-compilation.
  pub async fn call(&self, op: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let inv = Invocation::new(op, payload.to_vec());
    let op_len = inv.operation.len();
    let msg_len = inv.msg.len();

    {
      *self.state.guest_response.write().await = None;
      *self.state.guest_request.write().await = Some(inv);
      *self.state.guest_error.write().await = None;
      *self.state.host_response.write().await = None;
      *self.state.host_error.write().await = None;
    }

    let callresult = match self.engine.lock().await.call(op_len as i32, msg_len as i32).await {
      Ok(c) => c,
      Err(e) => {
        return Err(errors::Error::GuestCallFailure(e.to_string()));
      }
    };

    if callresult == 0 {
      // invocation failed
      let lock = self.state.guest_error.read().await;
      lock.as_ref().map_or_else(
        || {
          Err(errors::Error::GuestCallFailure(
            "No error message set for call failure".to_owned(),
          ))
        },
        |s| Err(errors::Error::GuestCallFailure(s.clone())),
      )
    } else {
      // invocation succeeded
      match self.state.guest_response.read().await.as_ref() {
        Some(r) => Ok(r.clone()),
        None => {
          let lock = self.state.guest_error.read().await;
          lock.as_ref().map_or_else(
            || {
              Err(errors::Error::GuestCallFailure(
                "No error message OR response set for call success".to_owned(),
              ))
            },
            |s| Err(errors::Error::GuestCallFailure(s.clone())),
          )
        }
      }
    }
  }

  /// Performs a live "hot swap" of the WebAssembly module. Since all internal waPC execution is assumed to be
  /// single-threaded and non-reentrant, this call is synchronous and so
  /// you should never attempt to invoke `call` from another thread while performing this hot swap.
  ///
  /// **Note**: if the underlying engine you've chosen is a JITting engine, then performing a swap
  /// will re-introduce a "cold start" delay upon the next function call.
  ///
  /// If you perform a hot swap of a WASI module, you cannot alter the parameters used to create the WASI module
  /// like the environment variables, mapped directories, pre-opened files, etc. Not abiding by this could lead
  /// to privilege escalation attacks or non-deterministic behavior after the swap.
  pub async fn replace_module(&self, module: &[u8]) -> Result<()> {
    match self.engine.lock().await.replace(module).await {
      Ok(_) => Ok(()),
      Err(e) => Err(errors::Error::ReplacementFailed(e.to_string())),
    }
  }
}
