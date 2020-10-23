//! Code borrowed from the wasmtime CLI

use anyhow::Context as _;
use std::error::Error;
use std::{
    ffi::OsStr,
    fs::File,
    path::{Component, PathBuf},
};
use wasi_common::preopen_dir;
use wasmtime::Store;
use wasmtime_wasi::{old::snapshot_0::Wasi as WasiSnapshot0, Wasi};

pub struct ModuleRegistry {
    pub wasi_snapshot_preview1: Wasi,
    pub wasi_unstable: WasiSnapshot0,
}

impl ModuleRegistry {
    pub fn new(
        store: &Store,
        preopen_dirs: &[(String, File)],
        argv: &[String],
        vars: &[(String, String)],
    ) -> Result<ModuleRegistry, Box<dyn Error>> {
        let mut cx1 = wasi_common::WasiCtxBuilder::new();

        cx1.inherit_stdio().args(argv).envs(vars);

        for (name, file) in preopen_dirs {
            cx1.preopened_dir(file.try_clone()?, name);
        }

        let cx1 = cx1.build().unwrap(); // TODO: get rid of unwrap

        let mut builder = wasi_common::old::snapshot_0::WasiCtxBuilder::new();

        let mut cx2 = builder.inherit_stdio().args(argv).envs(vars);

        for (name, file) in preopen_dirs {
            cx2 = cx2.preopened_dir(file.try_clone()?, name);
        }

        let cx2 = cx2.build().unwrap(); // TODO: get rid of unwrap

        Ok(ModuleRegistry {
            wasi_snapshot_preview1: Wasi::new(store, cx1),
            wasi_unstable: WasiSnapshot0::new(store, cx2),
        })
    }
}

pub(crate) fn compute_preopen_dirs(
    dirs: &Vec<String>,
    map_dirs: &Vec<(String, String)>,
) -> Result<Vec<(String, File)>, Box<dyn Error>> {
    let mut preopen_dirs = Vec::new();

    for dir in dirs.iter() {
        preopen_dirs.push((
            dir.clone(),
            preopen_dir(dir)
                .with_context(|| format!("failed to open directory '{}'", dir))
                .unwrap(), // TODO: get rid of unwrap
        ));
    }

    for (guest, host) in map_dirs.iter() {
        preopen_dirs.push((
            guest.clone(),
            preopen_dir(host)
                .with_context(|| format!("failed to open directory '{}'", host))
                .unwrap(), // TODO: get rid of unwrap
        ));
    }

    Ok(preopen_dirs)
}

#[allow(dead_code)]
pub(crate) fn compute_argv(module: PathBuf, module_args: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();

    // Add argv[0], which is the program name. Only include the base name of the
    // main wasm module, to avoid leaking path information.
    result.push(
        module
            .components()
            .next_back()
            .map(Component::as_os_str)
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_owned(),
    );

    // Add the remaining arguments.
    for arg in module_args.iter() {
        result.push(arg.clone());
    }

    result
}
