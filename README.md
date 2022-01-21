# waPC implementation for Rust

waPC is a protocol for communicating in and out of WebAssembly. This repository contains the Rust implementations for waPC hosts, guests, compatible codecs, and implementations for `wasmtime` and `wasm3` engines.

For more information about waPC, see [https://wapc.io]()

[![Apache 2.0 licensed][license]][license-url]
[![Build Status][actions-badge]][actions-url]

[license]: https://img.shields.io/github/license/wapc/wapc-rs
[license-url]: https://github.com/wapc/wapc-rs/blob/master/LICENSE
[actions-badge]: https://github.com/wapc/wapc-rs/workflows/CI/badge.svg
[actions-url]: https://github.com/wapc/wapc-rs/actions?query=workflow%3ACI+branch%3Amaster

[Website](https://wapc.io) |
[Docs and Tutorials](https://wapc.io/docs/) |

## Example

This code sets up a waPC host using the wasmtime WebAssembly engine. It loads a waPC guest WebAssembly module created by the waPC CLI and executes the operation "echo" with the `string` payload `"hello world"`.

```rs
use std::fs::read;

use wapc::{errors, WapcHost};
use wapc_codec::messagepack::{deserialize, serialize};

#[test]
fn runs_wapc_guest() -> Result<(), errors::Error> {
    let buf = read("../wapc-guest-test/build/wapc_guest_test.wasm")?;

    let engine = wasmtime_provider::WasmtimeEngineProvider::new(&buf, None)?;
    let guest = WapcHost::new(
        Box::new(engine),
        Some(Box::new(move |_a, _b, _c, _d, _e| Ok(vec![]))),
    )?;

    let callresult = guest.call("echo", &serialize("hello world").unwrap())?;
    let result: String = deserialize(&callresult).unwrap();
    assert_eq!(result, "hello world");
    Ok(())
}
```

## Projects

### `wapc-guest`

[![Crates badge][https://img.shields.io/crates/v/wapc-guest.svg]][https://crates.io/crates/wapc-guest]

The `wapc-guest` crate is used for Rust projects that will compile down to WebAssembly. It's typically used by code generated from the [`wapc`](https://github.com/wapc/cli) CLI tool.

### `wapc` (host)

[![Crates badge][https://img.shields.io/crates/v/wapc.svg]][https://crates.io/crates/wapc]

The `wapc` crate is for projects that want to run waPC WebAssembly modules. It contains the `WebAssemblyEngineProvider` trait which is used by the following projects to provide compatible implementations across multiple WebAssembly engines.

A full waPC host requires `wapc` combined with one of the WebAssembly engine providers below.

### `wasmtime-provider`

[![Crates badge][https://img.shields.io/crates/v/wasmtime-provider.svg]][https://crates.io/crates/wasmtime-provider]

The `wasmtime-provider` crate implements the `WebAssemblyEngineProvider` trait for the wasmtime engine.

#### Demo

```console
$ cargo run -p wasmtime-provider --example wasmtime-demo ./wasm/crates/wasm-basic/build/wasm_basic.wasm ping "hi"
```

### `wasm3-provider`

[![Crates badge][https://img.shields.io/crates/v/wasm3-provider.svg]][https://crates.io/crates/wasm3-provider]

The `wasm3-provider` crate implements the `WebAssemblyEngineProvider` trait for the wasm3 engine.

#### Demo

```console
$ cargo run -p wasm3-provider --example wasm3-demo ./wasm/crates/wasm-basic/build/wasm_basic.wasm ping "hi"
```

### `wapc-codec`

[![Crates badge][https://img.shields.io/crates/v/wapc-codec.svg]][https://crates.io/crates/wapc-codec]

The `wapc-codec` crate exposes compatible serialization and deserialization methods for sending data in and out of WASM modules.

#### Demo

```console
$ cargo run -p wapc-codec --example roundtrip
```

### `wapc-guest-codegen`

[![Npm badge][https://img.shields.io/npm/v/@wapc/codegen-rust-guest]][https://www.npmjs.com/package/@wapc/codegen-rust-guest]

The `wapc-guest-codegen` project includes the JavaScript source that the `wapc` CLI uses to generate code for Rust guests.

#### Demo

```console
$ wapc new rust <directory>
```

## Running tests

_Note: rebuilding the `wapc-guest-test` project requires the [waPC CLI](https://wapc.io)._

Run `make test` from the root to build the guest wasm and run the tests for all projects in the workspace.

### Note: Test wasm files

The wasm files and projects in this repository are for testing. They may not be good examples of practical wasm projects. Don't use them as examples of best practice.

#### waPC Guest Test

- `wasm/crates/wapc-guest-test/build/wapc_guest_test.wasm` - operation is `echo` and takes a string.

#### wasm + wasi

Both these projects respondo to an operation named `ping`. `ping` makes one host call for `pong` and then returns the payload back to the caller.

- `wasm/crates/wasm-basic/build/wasm_basic.wasm`
- `wasm/crates/wasi-basic/build/wasi_basic.wasm`

#### Cross-language

- `./wasm/hello_as.wasm` - operation is `hello`
- `./wasm/hello_tinygo.wasm` - operation is `hello`
- `./wasm/hello_zig.wasm` - operation is `hello`
