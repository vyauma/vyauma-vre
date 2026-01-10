//! VOL / Host integration helpers
//!
//! Minimal functions that consume `ExternalCallRequest` state changes and
//! call into host-provided handlers. This module contains no policy — the VM
//! already enforces capability checks; VOL only performs the mechanical handoff.

use crate::error::VreResult;
use crate::vm::vm::StateChange;
use crate::vm::value::Value;
use crate::vm::VirtualMachine;

/// Handler type provided by the host. Receives `cap_id` and argument slice,
/// returns a vector of result values to be pushed back onto the VM stack.
pub type HostHandler = fn(u8, &[Value]) -> VreResult<Vec<Value>>;

/// Consume the VM's drained state changes and handle the first ExternalCallRequest
/// found by invoking the provided `handler`. On success the handler results are
/// applied to the VM stack and the VM is resumed. This function intentionally
/// consumes the state's change buffer (deterministic handoff).
pub fn consume_external_call(vm: &mut VirtualMachine, handler: HostHandler) -> VreResult<()> {
    let changes = vm.drain_state_changes();

    // Find the first ExternalCallRequest
    for change in changes {
        match change {
            StateChange::ExternalCallRequest { cap_id, args } => {
                let results = handler(cap_id, &args)?;
                vm.apply_external_results(results)?;
                vm.resume();
                return Ok(());
            }
            // ignore other changes
            _ => continue,
        }
    }

    // No external call request found
    Err(crate::VreError::RuntimeFault)
}
