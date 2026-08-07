use std::error::Error;
use std::ffi::OsStr;
use std::path::{Component, Path};

use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtxBuilder};

pub(crate) fn init_ctx(
  preopen_dirs: &[(String, String)],
  argv: &[String],
  env: &[(String, String)],
) -> Result<WasiP1Ctx, Box<dyn Error + Send + Sync>> {
  let mut ctx_builder = WasiCtxBuilder::new();

  ctx_builder.inherit_stdio();
  ctx_builder.args(argv);
  ctx_builder.envs(env);

  for (guest, host) in preopen_dirs {
    ctx_builder.preopened_dir(host, guest, DirPerms::all(), FilePerms::all())?;
  }

  Ok(ctx_builder.build_p1())
}

pub(crate) fn compute_preopen_dirs(
  dirs: &[String],
  map_dirs: &[(String, String)],
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
  let mut preopen_dirs = Vec::new();

  for dir in dirs.iter() {
    preopen_dirs.push((dir.clone(), dir.clone()));
  }

  for (guest, host) in map_dirs.iter() {
    preopen_dirs.push((guest.clone(), host.clone()));
  }

  Ok(preopen_dirs)
}

#[allow(dead_code)]
pub(crate) fn compute_argv(module: &Path, module_args: &[String]) -> Vec<String> {
  // Add argv[0], which is the program name. Only include the base name of the
  // main wasm module, to avoid leaking path information.
  let mut result = vec![module
    .components()
    .next_back()
    .map(Component::as_os_str)
    .and_then(OsStr::to_str)
    .unwrap_or("")
    .to_owned()];

  // Add the remaining arguments.
  for arg in module_args.iter() {
    result.push(arg.clone());
  }

  result
}
