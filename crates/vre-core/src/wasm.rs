#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;
use crate::vm::vm::VirtualMachine;
use crate::config::VreConfig;
use crate::CapabilityRegistry;
use crate::vir::{Instruction, Module};

#[wasm_bindgen]
pub struct VreWasmContext {
    vm: VirtualMachine,
}

#[wasm_bindgen]
impl VreWasmContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<VreWasmContext, JsValue> {
        let config = VreConfig::default();
        let capabilities = CapabilityRegistry::new();
        // Provide empty instructions/constants for initialization, or expect them via a load call
        let vm = VirtualMachine::new(config, Vec::new(), Vec::new(), Vec::new(), capabilities)
            .map_err(|e| JsValue::from_str(&e))?;
        
        Ok(VreWasmContext { vm })
    }

    #[wasm_bindgen]
    pub fn load_and_run(&mut self, _bytecode: &[u8]) -> Result<(), JsValue> {
        // In a complete implementation, this would parse the bytecode into instructions
        // and inject them into the VM before calling execute.
        self.vm.execute().map_err(|e| JsValue::from_str(&e))?;
        Ok(())
    }
}

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    // This provides a panic hook when compiled to wasm, for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    Ok(())
}
