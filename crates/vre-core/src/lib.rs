//! Vyauma Runtime Engine (VRE) - Core Library
//!
//! This crate provides the core runtime components for executing
//! Vyauma bytecode in a deterministic, platform-agnostic manner.
//!
//! The public surface is intentionally minimal. Internal components
//! (VM, memory, execution model) are not exposed prematurely.

pub mod error;
pub mod bytecode;
pub mod config;
pub mod vm;
pub mod loader;
pub mod capability;
pub mod crypto;
pub mod jit;
pub mod scheduler;
pub mod module;
pub mod metrics;
pub mod plugin;
pub mod pal;
pub mod pal_android;
pub mod pal_ios;
pub mod hal;
pub mod distributed;
pub mod db;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
// Public error & configuration types
pub use error::{VreError, VreResult};
pub use config::VreConfig;

// Public-facing capability system
pub use capability::capability::Capability;
pub use capability::registry::CapabilityRegistry;

// Public-facing loader abstraction
pub use loader::loader::BytecodeLoader;

// Public-facing heap leak detection
pub use vm::memory::LeakReport;
