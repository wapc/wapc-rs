use crate::errors::{Error, Result};
use crate::{WasmtimeEngineProvider, WasmtimeEngineProviderPre};

/// Used to build [`WasmtimeEngineProvider`](crate::WasmtimeEngineProvider) instances.
#[allow(missing_debug_implementations)]
#[derive(Default)]
pub struct WasmtimeEngineProviderBuilder<'a> {
  engine: Option<wasmtime::Engine>,
  module: Option<wasmtime::Module>,
  module_bytes: Option<&'a [u8]>,
  #[cfg(feature = "cache")]
  cache_enabled: bool,
  #[cfg(feature = "cache")]
  cache_path: Option<std::path::PathBuf>,
  wasi_params: Option<wapc::WasiParams>,
  epoch_deadlines: Option<crate::EpochDeadlines>,
}

#[allow(deprecated)]
impl<'a> WasmtimeEngineProviderBuilder<'a> {
  /// Create a builder instance
  #[must_use]
  pub fn new() -> Self {
    Default::default()
  }

  /// Provide contents of the WebAssembly module
  #[must_use]
  pub fn module_bytes(mut self, module_bytes: &'a [u8]) -> Self {
    self.module_bytes = Some(module_bytes);
    self
  }

  /// Provide a preloaded [`wasmtime::Module`]
  ///
  /// **Warning:** the [`wasmtime::Engine`] used to load it must be provided via the
  /// [`WasmtimeEngineProviderBuilder::engine`] method, otherwise the code
  /// will panic at runtime later.
  #[must_use]
  pub fn module(mut self, module: wasmtime::Module) -> Self {
    self.module = Some(module);
    self
  }

  /// Provide a preinitialized [`wasmtime::Engine`]
  ///
  /// **Warning:** when used, engine specific options like
  /// [`cache`](WasmtimeEngineProviderBuilder::enable_cache) and
  /// [`enable_epoch_interruptions`](WasmtimeEngineProviderBuilder::enable_epoch_interruptions)
  /// must be pre-configured by the user. `WasmtimeEngineProviderBuilder` won't be
  /// able to configure them at [`build`](WasmtimeEngineProviderBuilder::build) time.
  #[must_use]
  pub fn engine(mut self, engine: wasmtime::Engine) -> Self {
    self.engine = Some(engine);
    self
  }

  /// WASI params
  #[must_use]
  pub fn wasi_params(mut self, wasi: wapc::WasiParams) -> Self {
    self.wasi_params = Some(wasi);
    self
  }

  /// Enable Wasmtime cache feature
  ///
  /// **Warning:** this has no effect when a custom [`wasmtime::Engine`] is provided via
  /// the [`WasmtimeEngineProviderBuilder::engine`] helper. In that case, it's up to the
  /// user to provide a [`wasmtime::Engine`] instance with the cache values properly configured.
  #[cfg(feature = "cache")]
  #[must_use]
  pub fn enable_cache(mut self, path: Option<&std::path::Path>) -> Self {
    self.cache_enabled = true;
    self.cache_path = path.map(|p| p.to_path_buf());
    self
  }

  /// Enable Wasmtime [epoch-based interruptions](wasmtime::Config::epoch_interruption) and set
  /// the deadlines to be enforced
  ///
  /// Two kind of deadlines have to be set:
  ///
  /// * `wapc_init_deadline`: the number of ticks the waPC initialization code can take before the
  ///   code is interrupted. This is the code usually defined inside of the `wapc_init`/`_start`
  ///   functions
  /// * `wapc_func_deadline`: the number of ticks any regular waPC guest function can run before
  ///   its terminated by the host
  ///
  /// Both these limits are expressed using the number of ticks that are allowed before the
  /// WebAssembly execution is interrupted.
  /// It's up to the embedder of waPC to define how much time a single tick is granted. This could
  /// be 1 second, 10 nanoseconds, or whatever the user prefers.
  ///
  /// **Warning:** when providing an instance of `wasmtime::Engine` via the
  /// `WasmtimeEngineProvider::engine` helper, ensure the `wasmtime::Engine`
  /// has been created with the `epoch_interruption` feature enabled
  #[must_use]
  pub fn enable_epoch_interruptions(mut self, wapc_init_deadline: u64, wapc_func_deadline: u64) -> Self {
    self.epoch_deadlines = Some(crate::EpochDeadlines {
      wapc_init: wapc_init_deadline,
      wapc_func: wapc_func_deadline,
    });
    self
  }

  /// Create a [`WasmtimeEngineProviderPre`] instance. This instance can then
  /// be reused as many time as wanted to quickly instantiate a [`WasmtimeEngineProvider`]
  /// by using the [`WasmtimeEngineProviderPre::rehydrate`] method.
  pub(crate) fn build_pre(&self) -> Result<WasmtimeEngineProviderPre> {
    if self.module_bytes.is_some() && self.module.is_some() {
      return Err(Error::BuilderInvalidConfig(
        "`module_bytes` and `module` cannot be provided at the same time".to_owned(),
      ));
    }
    if self.module_bytes.is_none() && self.module.is_none() {
      return Err(Error::BuilderInvalidConfig(
        "Neither `module_bytes` nor `module` have been provided".to_owned(),
      ));
    }

    let mut pre = match &self.engine {
      Some(e) => {
        let module = self.module_bytes.as_ref().map_or_else(
          || Ok(self.module.as_ref().unwrap().clone()),
          |module_bytes| wasmtime::Module::new(e, module_bytes),
        )?;

        // note: we have to call `.clone()` because `e` is behind
        // a shared reference and `Engine` does not implement `Copy`.
        // However, cloning an `Engine` is a cheap operation because
        // under the hood wasmtime does not create a new `Engine`, but
        // rather creates a new reference to it.
        // See https://docs.rs/wasmtime/latest/wasmtime/struct.Engine.html#engines-and-clone
        WasmtimeEngineProviderPre::new(e.clone(), module, self.wasi_params.clone())
      }
      None => {
        let mut config = wasmtime::Config::default();
        if self.epoch_deadlines.is_some() {
          config.epoch_interruption(true);
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "cache")] {
                  if self.cache_enabled {
                    config.strategy(wasmtime::Strategy::Cranelift);
                    if let Some(cache) = &self.cache_path {
                      config.cache_config_load(cache)?;
                    } else if let Err(e) = config.cache_config_load_default() {
                      warn!("Wasmtime cache configuration not found ({}). Repeated loads will speed up significantly with a cache configuration. See https://docs.wasmtime.dev/cli-cache.html for more information.",e);
                    }
                }
            }
        }

        let engine = wasmtime::Engine::new(&config)?;

        let module = self.module_bytes.as_ref().map_or_else(
          || Ok(self.module.as_ref().unwrap().clone()),
          |module_bytes| wasmtime::Module::new(&engine, module_bytes),
        )?;

        WasmtimeEngineProviderPre::new(engine, module, self.wasi_params.clone())
      }
    }?;
    pre.epoch_deadlines = self.epoch_deadlines;

    Ok(pre)
  }

  /// Create a `WasmtimeEngineProvider` instance
  pub fn build(&self) -> Result<WasmtimeEngineProvider> {
    let pre = self.build_pre()?;
    pre.rehydrate()
  }
}
