pub(crate) mod modulestate;
pub(crate) mod traits;

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use self::modulestate::ModuleState;
use self::traits::WebAssemblyEngineProvider;
use crate::{errors, HostCallback, Invocation};

static GLOBAL_MODULE_COUNT: AtomicU64 = AtomicU64::new(1);

type Result<T> = std::result::Result<T, crate::errors::Error>;

/// A WebAssembly host runtime for waPC-compliant modules
///
/// Use an instance of this struct to provide a means of invoking procedure calls by
/// specifying an operation name and a set of bytes representing the opaque operation payload.
/// `WapcHost` makes no assumptions about the contents or format of either the payload or the
/// operation name, other than that the operation name is a UTF-8 encoded string.
#[must_use]
pub struct WapcHost {
  engine: RefCell<Box<dyn WebAssemblyEngineProvider>>,
  state: Arc<ModuleState>,
}

impl std::fmt::Debug for WapcHost {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("WapcHost").field("state", &self.state).finish()
  }
}

impl WapcHost {
  /// Creates a new instance of a waPC-compliant host runtime paired with a given
  /// low-level engine provider
  pub fn new(engine: Box<dyn WebAssemblyEngineProvider>, host_callback: Option<Box<HostCallback>>) -> Result<Self> {
    let id = GLOBAL_MODULE_COUNT.fetch_add(1, Ordering::SeqCst);

    let state = Arc::new(ModuleState::new(host_callback, id));

    let mh = WapcHost {
      engine: RefCell::new(engine),
      state: state.clone(),
    };

    mh.initialize(state)?;

    Ok(mh)
  }

  fn initialize(&self, state: Arc<ModuleState>) -> Result<()> {
    match self.engine.borrow_mut().init(state) {
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
  pub fn call(&self, op: &str, payload: &[u8]) -> Result<Vec<u8>> {
    let inv = Invocation::new(op, payload.to_vec());
    let op_len = inv.operation.len();
    let msg_len = inv.msg.len();

    {
      *self.state.guest_response.write() = None;
      *self.state.guest_request.write() = Some(inv);
      *self.state.guest_error.write() = None;
      *self.state.host_response.write() = None;
      *self.state.host_error.write() = None;
    }

    let callresult = match self.engine.borrow_mut().call(op_len as i32, msg_len as i32) {
      Ok(c) => c,
      Err(e) => {
        return Err(errors::Error::GuestCallFailure(e.to_string()));
      }
    };

    if callresult == 0 {
      // invocation failed
      let lock = self.state.guest_error.read();
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
      self.state.guest_response.read().as_ref().map_or_else(
        || {
          let lock = self.state.guest_error.read();
          lock.as_ref().map_or_else(
            || {
              Err(errors::Error::GuestCallFailure(
                "No error message OR response set for call success".to_owned(),
              ))
            },
            |s| Err(errors::Error::GuestCallFailure(s.clone())),
          )
        },
        |e| Ok(e.clone()),
      )
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
  pub fn replace_module(&self, module: &[u8]) -> Result<()> {
    match self.engine.borrow_mut().replace(module) {
      Ok(_) => Ok(()),
      Err(e) => Err(errors::Error::ReplacementFailed(e.to_string())),
    }
  }
}
