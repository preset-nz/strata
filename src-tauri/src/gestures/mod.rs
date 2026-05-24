//! Native gesture bridge — see `~/rhizomatic-preset/guidance/design/native-gesture-bridge.md`.
//!
//! Per-platform native input monitors that emit a stable Tauri event for each
//! gesture. Frontend listeners stay platform-agnostic; the per-OS plumbing
//! lives here and is cfg-gated so non-target builds compile to a no-op.

#[cfg(target_os = "macos")]
pub mod force_touch;
