//! Capability identifiers
//!
//! Minimal, explicit capability id type. No policy here — ids are numeric and small.

/// Capability identifier type (explicitly small and stable)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(pub u8);

impl From<u8> for CapabilityId {
    fn from(b: u8) -> Self {
        CapabilityId(b)
    }
}

impl From<CapabilityId> for u8 {
    fn from(c: CapabilityId) -> Self {
        c.0
    }
}
