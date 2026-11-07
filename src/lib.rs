//! Small, useful tools for the hyperb renderers. They need nothing from a
//! server, so they run natively and as wasm modules in the renderer.

pub mod billsplit;
pub mod common;
pub mod finance;
pub mod img;
pub mod qr;
pub mod textutils;
pub mod worldclock;

/// The wasm module ABI the renderer loads (`SPEC.md`, section 4). A wasm build
/// hosts exactly one app, chosen by feature.
#[cfg(target_arch = "wasm32")]
mod wasm;
