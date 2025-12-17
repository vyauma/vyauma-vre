//! Vyauma Runtime Engine (VRE) - Core Library
//!
//! This crate provides the core runtime components for executing
//! Vyauma bytecode in a deterministic, platform-agnostic manner.
//!
//! The public surface is intentionally minimal. Internal components
//! (VM, memory, execution model) are not exposed prematurely.

pub mod error;
pub mod config;
pub mod bytecode;
pub mod vm;
pub mod loader;
pub mod capability;

// Public error & configuration types
pub use error::{VreError, VreResult};
pub use config::VreConfig;

// Public-facing capability system
pub use capability::registry::CapabilityRegistry;

// Public-facing loader abstraction
pub use loader::loader::BytecodeLoader;
