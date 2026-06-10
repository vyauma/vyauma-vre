pub mod dce;
pub mod constant_fold;

use super::Module;

pub trait OptimizationPass {
    fn run(&mut self, module: &mut Module) -> bool;
}

pub struct PassManager {
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
        }
    }
    
    pub fn add_pass(&mut self, pass: Box<dyn OptimizationPass>) {
        self.passes.push(pass);
    }
    
    pub fn run(&mut self, module: &mut Module) {
        let mut changed = true;
        while changed {
            changed = false;
            for pass in &mut self.passes {
                if pass.run(module) {
                    changed = true;
                }
            }
        }
    }
}
