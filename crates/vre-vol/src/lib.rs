//! VOL crate: host/OS integration helpers for VRE
//!
//! This crate contains the small mechanical helper to consume `ExternalCallRequest`
//! and invoke a host handler. It intentionally contains no policy.

pub use vre_core::VreResult;
pub use vre_core::vm::value::Value;
pub use vre_core::vm::VirtualMachine;

pub mod policy;

use vre_core::vm::vm::StateChange;

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
    Err(vre_core::VreError::RuntimeFault)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vre_core::vm::value::Value;
    use vre_core::config::VreConfig;

    #[test]
    fn host_handler_integration() {
        let config = VreConfig::new();
        let constants = vec![Value::Number(3.0)];
        let instructions = vec![
            vre_core::bytecode::OpCode::Push as u8,
            0u8,
            vre_core::bytecode::OpCode::ExternalCall as u8,
            5u8,
            1u8,
            vre_core::bytecode::OpCode::Halt as u8,
        ];

        let mut vm = VirtualMachine::new(config, constants, instructions, 0);
        vm.grant_capability(5u8);
        vm.execute().expect("execution failed");

        fn handler(_cap: u8, args: &[Value]) -> VreResult<Vec<Value>> {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Value::Number(3.0));
            Ok(vec![Value::Number(42.0)])
        }

        consume_external_call(&mut vm, handler).expect("host handler failed");
        vm.execute().expect("resume failed");
        let top = vm.peek_top().expect("peek failed");
        assert_eq!(top, Value::Number(42.0));
    }
}
