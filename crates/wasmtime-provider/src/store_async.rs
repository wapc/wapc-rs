use std::sync::Arc;

use wapc::ModuleStateAsync;

pub(crate) struct WapcStoreAsync {
  #[cfg(feature = "wasi")]
  pub(crate) wasi_ctx: wasi_common::WasiCtx,
  pub(crate) host: Option<Arc<ModuleStateAsync>>,
}

impl WapcStoreAsync {
  #[cfg(feature = "wasi")]
  pub(crate) fn new(
    wasi_params: &wapc::WasiParams,
    host: Option<Arc<ModuleStateAsync>>,
  ) -> crate::errors::Result<Self> {
    let preopened_dirs = crate::wasi::compute_preopen_dirs(&wasi_params.preopened_dirs, &wasi_params.map_dirs)
      .map_err(|e| crate::errors::Error::WasiInitCtxError(format!("Cannot compute preopened dirs: {e:?}")))?;
    let wasi_ctx = crate::wasi::init_ctx_async(preopened_dirs.as_slice(), &wasi_params.argv, &wasi_params.env_vars)
      .map_err(|e| crate::errors::Error::WasiInitCtxError(e.to_string()))?;

    Ok(Self { wasi_ctx, host })
  }

  #[cfg(not(feature = "wasi"))]
  pub(crate) fn new(host: Option<Arc<ModuleStateAsync>>) -> Self {
    Self { host }
  }
}
