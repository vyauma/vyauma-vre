use super::OptimizationPass;
use crate::vir::{Module, Instruction, Value};
use std::collections::HashMap;

pub struct ConstantFolding;

impl OptimizationPass for ConstantFolding {
    fn run(&mut self, module: &mut Module) -> bool {
        let mut changed = false;
        
        for func in &mut module.functions {
            // value -> literal
            let mut const_values = HashMap::new();
            
            for block in &mut func.blocks {
                for (val, inst) in &mut block.instructions {
                    let mut folded = None;
                    
                    match inst {
                        Instruction::LoadConstNumber(n) => { const_values.insert(*val, *n); },
                        Instruction::Add(l, r) => {
                            if let (Some(left), Some(right)) = (const_values.get(l), const_values.get(r)) {
                                folded = Some(Instruction::LoadConstNumber(*left + *right));
                            }
                        }
                        Instruction::Sub(l, r) => {
                            if let (Some(left), Some(right)) = (const_values.get(l), const_values.get(r)) {
                                folded = Some(Instruction::LoadConstNumber(*left - *right));
                            }
                        }
                        Instruction::Mul(l, r) => {
                            if let (Some(left), Some(right)) = (const_values.get(l), const_values.get(r)) {
                                folded = Some(Instruction::LoadConstNumber(*left * *right));
                            }
                        }
                        Instruction::Div(l, r) => {
                            if let (Some(left), Some(right)) = (const_values.get(l), const_values.get(r)) {
                                if *right != 0.0 {
                                    folded = Some(Instruction::LoadConstNumber(*left / *right));
                                }
                            }
                        }
                        Instruction::Rem(l, r) => {
                            if let (Some(left), Some(right)) = (const_values.get(l), const_values.get(r)) {
                                if *right != 0.0 {
                                    folded = Some(Instruction::LoadConstNumber(*left % *right));
                                }
                            }
                        }
                        _ => {}
                    }
                    
                    if let Some(new_inst) = folded {
                        if let Instruction::LoadConstNumber(n) = &new_inst {
                            const_values.insert(*val, *n);
                        }
                        *inst = new_inst;
                        changed = true;
                    }
                }
            }
        }
        
        changed
    }
}
