#![deny(
  clippy::expect_used,
  clippy::explicit_deref_methods,
  clippy::option_if_let_else,
  clippy::await_holding_lock,
  clippy::cloned_instead_of_copied,
  clippy::explicit_into_iter_loop,
  clippy::flat_map_option,
  clippy::fn_params_excessive_bools,
  clippy::implicit_clone,
  clippy::inefficient_to_string,
  clippy::large_types_passed_by_value,
  clippy::manual_ok_or,
  clippy::map_flatten,
  clippy::map_unwrap_or,
  clippy::must_use_candidate,
  clippy::needless_for_each,
  clippy::needless_pass_by_value,
  clippy::option_option,
  clippy::redundant_else,
  clippy::semicolon_if_nothing_returned,
  clippy::too_many_lines,
  clippy::trivially_copy_pass_by_ref,
  clippy::unnested_or_patterns,
  clippy::future_not_send,
  clippy::useless_let_if_seq,
  clippy::str_to_string,
  clippy::inherent_to_string,
  clippy::let_and_return,
  clippy::try_err,
  clippy::unused_async,
  clippy::missing_enforced_import_renames,
  clippy::nonstandard_macro_braces,
  clippy::rc_mutex,
  clippy::unwrap_or_default,
  clippy::manual_split_once,
  clippy::derivable_impls,
  clippy::needless_option_as_deref,
  clippy::iter_not_returning_iterator,
  clippy::same_name_method,
  clippy::manual_assert,
  clippy::non_send_fields_in_send_ty,
  clippy::equatable_if_let,
  bad_style,
  clashing_extern_declarations,
  dead_code,
  deprecated,
  explicit_outlives_requirements,
  improper_ctypes,
  invalid_value,
  missing_copy_implementations,
  missing_debug_implementations,
  mutable_transmutes,
  no_mangle_generic_items,
  non_shorthand_field_patterns,
  overflowing_literals,
  path_statements,
  patterns_in_fns_without_body,
  private_interfaces,
  private_bounds,
  renamed_and_removed_lints,
  trivial_bounds,
  trivial_casts,
  trivial_numeric_casts,
  type_alias_bounds,
  unconditional_recursion,
  unreachable_pub,
  unsafe_code,
  unstable_features,
  unused,
  unused_allocation,
  unused_comparisons,
  unused_import_braces,
  unused_parens,
  unused_qualifications,
  while_true,
  missing_docs
)]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod callbacks;
#[cfg(feature = "async")]
mod callbacks_async;
#[cfg(feature = "wasi")]
mod wasi;

mod provider;
pub use provider::{WasmtimeEngineProvider, WasmtimeEngineProviderPre};

#[cfg(feature = "async")]
mod provider_async;
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use provider_async::{WasmtimeEngineProviderAsync, WasmtimeEngineProviderAsyncPre};

mod store;

#[cfg(feature = "async")]
mod store_async;

pub mod errors;

mod builder;
pub use builder::WasmtimeEngineProviderBuilder;
// export wasmtime and wasmtime_wasi, so that consumers of this crate can use
// the very same version
pub use wasmtime;
#[cfg(feature = "wasi")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasi")))]
pub use wasmtime_wasi;

/// Configure behavior of wasmtime [epoch-based interruptions](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.epoch_interruption)
///
/// There are two kind of deadlines that apply to waPC modules:
///
/// * waPC initialization code: this is the code defined by the module inside
///   of the `wapc_init` or the `_start` functions
/// * user function: the actual waPC guest function written by an user
///
/// Both these limits are expressed using the number of ticks that are allowed before the
/// WebAssembly execution is interrupted.
/// It's up to the embedder of waPC to define how much time a single tick is granted. This could
/// be 1 second, 10 nanoseconds, or whatever the user prefers.
#[derive(Clone, Copy, Debug)]
pub struct EpochDeadlines {
  /// Deadline for waPC initialization code. Expressed in number of epoch ticks
  pub wapc_init: u64,

  /// Deadline for user-defined waPC function computation. Expressed in number of epoch ticks
  pub wapc_func: u64,
}

/// Configure limits on the resources a single waPC WebAssembly instance is
/// allowed to consume, leveraging wasmtime's
/// [`ResourceLimiter`](https://docs.rs/wasmtime/latest/wasmtime/trait.ResourceLimiter.html)
/// facility (via [`wasmtime::StoreLimits`]).
///
/// This can be used to prevent a malicious, or misbehaving, WebAssembly
/// module from exhausting the host's memory, for example by growing its
/// linear memory in an unbounded loop.
///
/// When a limit is exceeded, the corresponding `memory.grow`/`table.grow`
/// wasm instruction fails and returns `-1` to the guest, following the
/// WebAssembly specification. Most language toolchains (Rust, TinyGo,
/// AssemblyScript, ...) treat a failed growth as a fatal allocation failure
/// and abort the guest, which is reported back to the host as a trap. The
/// memory/table cap itself is always enforced by the host regardless of how
/// the guest reacts to the failed growth.
#[derive(Clone, Copy, Debug, Default)]
pub struct ResourceLimits {
  /// Maximum size, in bytes, that each of the module's linear memories is
  /// allowed to grow to. This limit is applied to each linear memory
  /// individually.
  ///
  /// `None` (the default) means no limit is enforced.
  pub max_memory_size: Option<usize>,

  /// Maximum number of elements each of the module's tables is allowed to
  /// grow to. This limit is applied to each table individually.
  ///
  /// WebAssembly tables are used to hold indirect function references (e.g.
  /// Rust trait objects, Go interfaces, C function pointers) and each
  /// element costs roughly the size of a pointer of host memory. Most
  /// modules never grow their tables at runtime, so this is a secondary,
  /// defense-in-depth limit compared to [`ResourceLimits::max_memory_size`].
  /// A generous value such as `100_000` (~0.8 MB on a 64-bit host) is
  /// unlikely to affect legitimate modules.
  ///
  /// `None` (the default) means no limit is enforced.
  pub max_table_elements: Option<usize>,
}

// Builds a `wasmtime::StoreLimits` out of the (optional) `ResourceLimits`
// configuration. When `None` is provided, the resulting limits are
// effectively unlimited (i.e. wasmtime's defaults).
fn store_limits(resource_limits: Option<ResourceLimits>) -> wasmtime::StoreLimits {
  let mut builder = wasmtime::StoreLimitsBuilder::new();
  if let Some(limits) = resource_limits {
    if let Some(max_memory_size) = limits.max_memory_size {
      builder = builder.memory_size(max_memory_size);
    }
    if let Some(max_table_elements) = limits.max_table_elements {
      builder = builder.table_elements(max_table_elements);
    }
  }
  builder.build()
}
