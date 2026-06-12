//! Capability Registry
//!
//! Tracks which capabilities have been granted to the current execution context.
//! All capability checks MUST go through this registry.
//! The registry is intentionally immutable after construction (v0.1).

use std::collections::HashSet;
use super::capability::Capability;
use crate::error::{VreError, VreResult};

/// Registry of granted capabilities
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    granted: HashSet<&'static str>,
}

impl CapabilityRegistry {
    /// Create an empty registry (no capabilities granted)
    pub fn new() -> Self {
        CapabilityRegistry {
            granted: HashSet::new(),
        }
    }

    /// Grant a capability
    pub fn grant(&mut self, capability: Capability) {
        self.granted.insert(capability.name);
    }

    /// Check if a capability is granted, returning an error if not
    pub fn require(&self, capability: &Capability) -> VreResult<()> {
        if self.granted.contains(capability.name) {
            Ok(())
        } else {
            Err(VreError::CapabilityNotGranted)
        }
    }

    /// Returns true if the capability is granted
    pub fn has(&self, capability: &Capability) -> bool {
        self.granted.contains(capability.name)
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
