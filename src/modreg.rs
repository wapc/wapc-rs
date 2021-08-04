//! Code borrowed from the wasmtime CLI

use cap_std::fs::Dir;
use std::{cell::RefCell, error::Error, rc::Rc};
use std::{
    ffi::OsStr,
    path::{Component, PathBuf},
};
use wasmtime::Store;

pub struct ModuleRegistry {
    pub wasi_snapshot_preview1: wasmtime_wasi::snapshots::preview_1::Wasi,
    pub wasi_unstable: wasmtime_wasi::snapshots::preview_0::Wasi,
}

impl ModuleRegistry {
    pub fn new(
        store: &Store,
        preopen_dirs: &[(String, Dir)],
        argv: &[String],
        vars: &[(String, String)],
    ) -> Result<ModuleRegistry, Box<dyn Error>> {
        let mut cx1 = wasi_cap_std_sync::WasiCtxBuilder::new();

        cx1 = cx1.inherit_stdio().args(argv)?.envs(vars)?;

        for (name, file) in preopen_dirs {
            cx1 = cx1.preopened_dir(file.try_clone()?, name)?;
        }

        let mut cx2 = wasi_cap_std_sync::WasiCtxBuilder::new();

        cx2 = cx2.inherit_stdio().args(argv)?.envs(vars)?;

        for (name, file) in preopen_dirs {
            cx2 = cx2.preopened_dir(file.try_clone()?, name)?;
        }

        Ok(ModuleRegistry {
            wasi_snapshot_preview1: wasmtime_wasi::snapshots::preview_1::Wasi::new(
                store,
                Rc::new(RefCell::new(cx1.build()?)),
            ),
            wasi_unstable: wasmtime_wasi::snapshots::preview_0::Wasi::new(
                store,
                Rc::new(RefCell::new(cx2.build()?)),
            ),
        })
    }
}

pub(crate) fn compute_preopen_dirs(
    dirs: &[String],
    map_dirs: &[(String, String)],
) -> Result<Vec<(String, Dir)>, Box<dyn Error>> {
    let mut preopen_dirs = Vec::new();

    for dir in dirs.iter() {
        preopen_dirs.push((dir.clone(), unsafe { Dir::open_ambient_dir(dir)? }));
    }

    for (guest, host) in map_dirs.iter() {
        preopen_dirs.push((guest.clone(), unsafe { Dir::open_ambient_dir(host)? }));
    }

    Ok(preopen_dirs)
}

#[allow(dead_code, clippy::vec_init_then_push)]
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
