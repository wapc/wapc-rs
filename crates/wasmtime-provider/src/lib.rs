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
  clippy::string_to_string,
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
#[derive(Clone, Copy, Debug)]
struct EpochDeadlines {
  /// Deadline for waPC initialization code. Expressed in number of epoch ticks
  wapc_init: u64,

  /// Deadline for user-defined waPC function computation. Expressed in number of epoch ticks
  wapc_func: u64,
}
