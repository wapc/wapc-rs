//! Verifies that `wasmtime_provider::ResourceLimits` is correctly enforced by
//! leveraging wasmtime's `ResourceLimiter` facility. A malicious (or
//! misbehaving) guest module that keeps growing its linear memory (or a
//! table) in a loop must be stopped by the host once the configured limit is
//! reached, instead of being able to exhaust the host's memory.
//!
//! Every scenario is covered for both the sync (`WapcHost`) and async
//! (`WapcHostAsync`) providers.

use wapc::errors::Error;
use wapc::WapcHost;
#[cfg(feature = "async")]
use wapc::WapcHostAsync;
use wasmtime_provider::{ResourceLimits, WasmtimeEngineProviderBuilder};

// Per the WebAssembly specification, linear memory is grown in units of
// pages, and a page is fixed at 64KiB.
const WASM_PAGE_SIZE: usize = 65536;

/// A minimal waPC-ish guest module (hand written in WAT) that, on every
/// `__guest_call`, keeps growing its linear memory by one page at a time
/// until `memory.grow` fails. It then reports back (as its response) the
/// final memory size, expressed in pages.
///
/// The module itself declares a maximum of 16 pages, so that even when no
/// host-side resource limit is configured, the test terminates quickly.
const GROW_MEMORY_WAT: &str = r#"
(module
  (import "wapc" "__guest_response" (func $guest_response (param i32 i32)))
  (memory (export "memory") 1 16)
  (func (export "__guest_call") (param i32 i32) (result i32)
    (local $pages i32)
    (block $done
      (loop $loop
        (local.set $pages (memory.grow (i32.const 1)))
        (br_if $done (i32.lt_s (local.get $pages) (i32.const 0)))
        (br $loop)
      )
    )
    (i32.store (i32.const 0) (memory.size))
    (call $guest_response (i32.const 0) (i32.const 4))
    (i32.const 1)
  )
)
"#;

/// Same idea as [`GROW_MEMORY_WAT`], but growing a `funcref` table instead of
/// linear memory. The table declares a maximum of 2000 elements.
const GROW_TABLE_WAT: &str = r#"
(module
  (import "wapc" "__guest_response" (func $guest_response (param i32 i32)))
  (memory (export "memory") 1)
  (table (export "tbl") 1 2000 funcref)
  (func (export "__guest_call") (param i32 i32) (result i32)
    (local $n i32)
    (block $done
      (loop $loop
        (local.set $n (table.grow (ref.null func) (i32.const 1)))
        (br_if $done (i32.lt_s (local.get $n) (i32.const 0)))
        (br $loop)
      )
    )
    (i32.store (i32.const 0) (table.size))
    (call $guest_response (i32.const 0) (i32.const 4))
    (i32.const 1)
  )
)
"#;

fn noop_callback(
  _id: u64,
  _bd: &str,
  _ns: &str,
  _op: &str,
  _payload: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  Ok(vec![])
}

/// Builds a `WapcHost` running the given WAT module, optionally enforcing the
/// given `ResourceLimits`.
fn create_guest(wat: &str, resource_limits: Option<ResourceLimits>) -> Result<WapcHost, Error> {
  let mut builder = WasmtimeEngineProviderBuilder::new().module_bytes(wat.as_bytes());
  if let Some(resource_limits) = resource_limits {
    builder = builder.enable_resource_limits(resource_limits);
  }
  let engine = builder.build().expect("Cannot create WebAssemblyEngineProvider");
  WapcHost::new(Box::new(engine), Some(Box::new(noop_callback)))
}

fn call_and_read_u32(guest: &WapcHost, op: &str) -> u32 {
  let callresult = guest.call(op, &[]).expect("call should succeed");
  u32::from_le_bytes(callresult.try_into().expect("response should be 4 bytes"))
}

#[cfg(feature = "async")]
async fn noop_callback_async(
  _id: u64,
  _bd: String,
  _ns: String,
  _op: String,
  _payload: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
  Ok(vec![])
}

/// Async counterpart of [`create_guest`].
#[cfg(feature = "async")]
async fn create_guest_async(wat: &str, resource_limits: Option<ResourceLimits>) -> Result<WapcHostAsync, Error> {
  let mut builder = WasmtimeEngineProviderBuilder::new().module_bytes(wat.as_bytes());
  if let Some(resource_limits) = resource_limits {
    builder = builder.enable_resource_limits(resource_limits);
  }
  let engine = builder
    .build_async()
    .expect("Cannot create WebAssemblyEngineProviderAsync");

  let callback: Box<wapc::HostCallbackAsync> =
    Box::new(move |id, bd, ns, op, payload| Box::pin(noop_callback_async(id, bd, ns, op, payload)));
  WapcHostAsync::new(Box::new(engine), Some(callback)).await
}

#[cfg(feature = "async")]
async fn call_and_read_u32_async(guest: &WapcHostAsync, op: &str) -> u32 {
  let callresult = guest.call(op, &[]).await.expect("call should succeed");
  u32::from_le_bytes(callresult.try_into().expect("response should be 4 bytes"))
}

#[test]
fn memory_growth_is_unbounded_without_resource_limits() -> Result<(), Error> {
  let guest = create_guest(GROW_MEMORY_WAT, None)?;

  // Without any resource limit configured, the guest can grow memory up to
  // the module's own declared maximum (16 pages).
  let pages = call_and_read_u32(&guest, "grow");
  assert_eq!(pages, 16);
  Ok(())
}

#[cfg(feature = "async")]
#[tokio::test]
async fn memory_growth_is_unbounded_without_resource_limits_async() -> Result<(), Error> {
  let guest = create_guest_async(GROW_MEMORY_WAT, None).await?;

  let pages = call_and_read_u32_async(&guest, "grow").await;
  assert_eq!(pages, 16);
  Ok(())
}

#[test]
fn memory_growth_is_capped_by_resource_limits() -> Result<(), Error> {
  let resource_limits = ResourceLimits {
    max_memory_size: Some(4 * WASM_PAGE_SIZE),
    ..Default::default()
  };
  let guest = create_guest(GROW_MEMORY_WAT, Some(resource_limits))?;

  // The host-enforced limit (4 pages) is lower than the module's own maximum
  // (16 pages), so it must be the one that stops the growth.
  let pages = call_and_read_u32(&guest, "grow");
  assert_eq!(pages, 4);
  Ok(())
}

#[cfg(feature = "async")]
#[tokio::test]
async fn memory_growth_is_capped_by_resource_limits_async() -> Result<(), Error> {
  let resource_limits = ResourceLimits {
    max_memory_size: Some(4 * WASM_PAGE_SIZE),
    ..Default::default()
  };
  let guest = create_guest_async(GROW_MEMORY_WAT, Some(resource_limits)).await?;

  let pages = call_and_read_u32_async(&guest, "grow").await;
  assert_eq!(pages, 4);
  Ok(())
}

#[test]
fn table_growth_is_unbounded_without_resource_limits() -> Result<(), Error> {
  let guest = create_guest(GROW_TABLE_WAT, None)?;

  let elements = call_and_read_u32(&guest, "grow");
  assert_eq!(elements, 2000);
  Ok(())
}

#[cfg(feature = "async")]
#[tokio::test]
async fn table_growth_is_unbounded_without_resource_limits_async() -> Result<(), Error> {
  let guest = create_guest_async(GROW_TABLE_WAT, None).await?;

  let elements = call_and_read_u32_async(&guest, "grow").await;
  assert_eq!(elements, 2000);
  Ok(())
}

#[test]
fn table_growth_is_capped_by_resource_limits() -> Result<(), Error> {
  let resource_limits = ResourceLimits {
    max_table_elements: Some(50),
    ..Default::default()
  };
  let guest = create_guest(GROW_TABLE_WAT, Some(resource_limits))?;

  let elements = call_and_read_u32(&guest, "grow");
  assert_eq!(elements, 50);
  Ok(())
}

#[cfg(feature = "async")]
#[tokio::test]
async fn table_growth_is_capped_by_resource_limits_async() -> Result<(), Error> {
  let resource_limits = ResourceLimits {
    max_table_elements: Some(50),
    ..Default::default()
  };
  let guest = create_guest_async(GROW_TABLE_WAT, Some(resource_limits)).await?;

  let elements = call_and_read_u32_async(&guest, "grow").await;
  assert_eq!(elements, 50);
  Ok(())
}

#[test]
fn initial_memory_above_limit_fails_at_init() {
  // The module requests 1 page (64KiB) of initial memory, which is already
  // above the configured limit.
  let resource_limits = ResourceLimits {
    max_memory_size: Some(WASM_PAGE_SIZE / 2),
    ..Default::default()
  };

  let err = create_guest(GROW_MEMORY_WAT, Some(resource_limits))
    .expect_err("instantiation should fail because initial memory exceeds the limit");
  assert!(err.to_string().contains("memory"), "unexpected error: {err}");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn initial_memory_above_limit_fails_at_init_async() {
  // The module requests 1 page (64KiB) of initial memory, which is already
  // above the configured limit.
  let resource_limits = ResourceLimits {
    max_memory_size: Some(WASM_PAGE_SIZE / 2),
    ..Default::default()
  };

  let err = create_guest_async(GROW_MEMORY_WAT, Some(resource_limits))
    .await
    .expect_err("instantiation should fail because initial memory exceeds the limit");
  assert!(err.to_string().contains("memory"), "unexpected error: {err}");
}
