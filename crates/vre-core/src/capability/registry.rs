//! Capability Registry (Suraksha)
//!
//! Very small, explicit registry that keeps granted capability ids.
//! Behavior: default deny-all; checks fail-closed.

use std::collections::HashSet;
use crate::error::{VreError, VreResult};
use super::capability::CapabilityId;

/// Minimal capability registry used by the VM for Suraksha checks.
#[derive(Debug)]
pub struct CapabilityRegistry {
    granted: HashSet<CapabilityId>,
}

impl CapabilityRegistry {
    /// New registry denies everything by default
    pub fn new() -> Self {
        CapabilityRegistry { granted: HashSet::new() }
    }

    /// Grant a capability (host-level operation)
    pub fn grant(&mut self, id: CapabilityId) {
        self.granted.insert(id);
    }

    /// Revoke a capability
    pub fn revoke(&mut self, id: &CapabilityId) {
        self.granted.remove(id);
    }

    /// Check a capability id and fail-closed if not granted
    pub fn check(&self, raw_id: u8) -> VreResult<()> {
        let id = CapabilityId(raw_id);
        if self.granted.contains(&id) {
            Ok(())
        } else {
            Err(VreError::CapabilityDenied)
        }
    }
}
