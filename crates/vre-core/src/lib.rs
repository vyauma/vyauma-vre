//! Vyauma Runtime Engine - Core Library
//!
//! Public API surface for the VRE core.

pub mod error;
pub mod config;
pub mod bytecode;
pub mod vm;
pub mod loader;
pub mod capability;

// Re-export commonly used types
pub use error::{VreError, VreResult};
pub use config::VreConfig;
pub use bytecode::opcode::OpCode;
pub use vm::vm::VirtualMachine;
pub use loader::BytecodeLoader;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::vm::vm::StateChange;
	// `Value` is referenced in some conditional test code; keep import for clarity
	use crate::vm::value::Value;

	#[test]
	fn store_local_emits_state_change() {
		let config = VreConfig::new();
		// constant 0 -> number 42.0
		let constants = vec![Value::Number(42.0)];
		let instructions = vec![
			OpCode::Push as u8,
			0u8,
			OpCode::StoreLocal as u8,
			0u8,
			OpCode::Halt as u8,
		];

		let mut vm = VirtualMachine::new(config, constants, instructions, 1);
		vm.execute().expect("execution failed");
		let changes = vm.drain_state_changes();
		assert_eq!(changes.len(), 1);
		match &changes[0] {
			StateChange::LocalStore { index, value } => {
				assert_eq!(*index, 0);
				assert_eq!(*value, Value::Number(42.0));
			}
			other => panic!("unexpected state change: {:?}", other),
		}
	}

	#[test]
	fn loader_rejects_pop_underflow() {
		// Build a minimal bytecode with a Pop at start (no pushes)
		// intentionally left: used in test output when enabled
		let mut buf = Vec::new();
		buf.extend(&0x5659_4D41u32.to_be_bytes()); // magic
		buf.push(1u8); buf.push(0u8); buf.push(0u8); buf.push(0u8); // version + reserved
		buf.extend(&(0u32.to_be_bytes())); // entry point
		buf.extend(&(0u32.to_be_bytes())); // 0 constants
		let instr = vec![
			crate::bytecode::OpCode::Pop as u8,
		];
		buf.extend(&(instr.len() as u32).to_be_bytes());
		buf.extend(&instr);

		let res = crate::loader::BytecodeLoader::load(&buf);
		assert!(res.is_err());
	}

	#[test]
	fn loader_rejects_externalcall_arg_mismatch() {
		// Bytecode: Push const, ExternalCall with argc=2 (but stack has 1)
		use crate::vm::value::Value;
		let mut buf = Vec::new();
		buf.extend(&0x5659_4D41u32.to_be_bytes());
		buf.push(1u8); buf.push(0u8); buf.push(0u8); buf.push(0u8);
		buf.extend(&(0u32.to_be_bytes())); // entry
		// 1 constant
		buf.extend(&(1u32.to_be_bytes()));
		buf.push(0x02u8); buf.extend(&3.14f64.to_be_bytes());
		let instr = vec![
			crate::bytecode::OpCode::Push as u8,
			0u8,
			crate::bytecode::OpCode::ExternalCall as u8,
			42u8,
			2u8, // argc=2, but only 1 value on stack
		];
		buf.extend(&(instr.len() as u32).to_be_bytes());
		buf.extend(&instr);

		let res = crate::loader::BytecodeLoader::load(&buf);
		assert!(res.is_err());
	}

	#[test]
	fn push_ref_preserved() {
		let config = VreConfig::new();
		let constants = vec![Value::Ref(123)];
		let instructions = vec![OpCode::Push as u8, 0u8, OpCode::Halt as u8];
		let mut vm = VirtualMachine::new(config, constants, instructions, 0);
		vm.execute().expect("execution failed");
		let top = vm.peek_top().expect("peek failed");
		assert_eq!(top, Value::Ref(123));
	}

	#[test]
	fn stack_overflow_trapped() {
		let mut cfg = VreConfig::new();
		cfg.max_stack_size = 1;
		let constants = vec![Value::Number(1.0)];
		let instructions = vec![
			OpCode::Push as u8,
			0u8,
			OpCode::Push as u8,
			0u8,
			OpCode::Halt as u8,
		];
		let mut vm = VirtualMachine::new(cfg, constants, instructions, 0);
		let res = vm.execute();
		assert!(res.is_err());
	}

	#[test]
	fn externalcall_with_ref_arg_emits_request() {
		let config = VreConfig::new();
		let constants = vec![Value::Ref(7)];
		let instructions = vec![
			OpCode::Push as u8,
			0u8,
			OpCode::ExternalCall as u8,
			42u8,
			1u8,
			OpCode::Halt as u8,
		];
		let mut vm = VirtualMachine::new(config, constants, instructions, 0);
		// grant capability so ExternalCall doesn't fail-closed
		vm.grant_capability(42);
		vm.execute().expect("execution failed");
		let changes = vm.drain_state_changes();
		assert_eq!(changes.len(), 1);
		match &changes[0] {
			StateChange::ExternalCallRequest { cap_id, args } => {
				assert_eq!(*cap_id, 42);
				assert_eq!(args.len(), 1);
				assert_eq!(args[0], Value::Ref(7));
			}
			_ => panic!("unexpected state change"),
		}
	}
}
