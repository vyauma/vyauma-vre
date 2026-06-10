use crate::vm::vm::VirtualMachine;
use crate::error::VreResult;

pub trait VrePlugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    
    /// Called when the plugin is loaded into the engine.
    fn on_load(&mut self) -> VreResult<()>;
    
    /// Allow the plugin to register custom opcodes, types, or native functions.
    fn register_capabilities(&mut self, vm: &mut VirtualMachine) -> VreResult<()>;
    
    /// Called when the engine is shutting down.
    fn on_unload(&mut self) -> VreResult<()>;
}
