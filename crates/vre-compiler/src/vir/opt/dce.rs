use super::OptimizationPass;
use crate::vir::{Module, Instruction, Value};
use std::collections::HashSet;

pub struct DeadCodeElimination;

impl OptimizationPass for DeadCodeElimination {
    fn run(&mut self, module: &mut Module) -> bool {
        let mut changed = false;
        
        for func in &mut module.functions {
            // Very simple DCE:
            // Find all used values
            let mut used_values = HashSet::new();
            for block in &func.blocks {
                for (_, inst) in &block.instructions {
                    match inst {
                        Instruction::Add(l, r) | Instruction::Sub(l, r) | 
                        Instruction::Mul(l, r) | Instruction::Div(l, r) | 
                        Instruction::Rem(l, r) | Instruction::Eq(l, r) | 
                        Instruction::NotEq(l, r) | Instruction::Lt(l, r) | 
                        Instruction::Lte(l, r) | Instruction::Gt(l, r) | 
                        Instruction::Gte(l, r) | Instruction::And(l, r) | 
                        Instruction::Or(l, r) | Instruction::IndexAccess(l, r) => {
                            used_values.insert(*l);
                            used_values.insert(*r);
                        }
                        Instruction::Not(v) | Instruction::Throw(v) | Instruction::PropertyAccess(v, _) => {
                            used_values.insert(*v);
                        }
                        Instruction::Return(Some(v)) => {
                            used_values.insert(*v);
                        }
                        Instruction::CondBranch(v, _, _) => {
                            used_values.insert(*v);
                        }
                        Instruction::Call(_, args) | Instruction::NewClass(_, args) | Instruction::ArrayLiteral(args) => {
                            for a in args { used_values.insert(*a); }
                        }
                        Instruction::MethodCall(obj, _, args) => {
                            used_values.insert(*obj);
                            for a in args { used_values.insert(*a); }
                        }
                        Instruction::DictLiteral(pairs) => {
                            for (k, v) in pairs {
                                used_values.insert(*k);
                                used_values.insert(*v);
                            }
                        }
                        Instruction::StructInit(_, fields) => {
                            for (_, v) in fields {
                                used_values.insert(*v);
                            }
                        }
                        Instruction::AssignIndex(arr, idx, val) => {
                            used_values.insert(*arr);
                            used_values.insert(*idx);
                            used_values.insert(*val);
                        }
                        Instruction::AssignProperty(obj, _, val) => {
                            used_values.insert(*obj);
                            used_values.insert(*val);
                        }
                        Instruction::StoreVar(_, v) => {
                            used_values.insert(*v);
                        }
                        _ => {}
                    }
                }
            }
            
            // Remove definitions of unused values that don't have side effects
            for block in &mut func.blocks {
                let initial_len = block.instructions.len();
                block.instructions.retain(|(val, inst)| {
                    if used_values.contains(val) {
                        return true;
                    }
                    // Keep instructions with side effects even if their result is unused
                    matches!(inst, 
                        Instruction::Call(..) | Instruction::MethodCall(..) | 
                        Instruction::NewClass(..) | Instruction::AssignIndex(..) | 
                        Instruction::AssignProperty(..) | Instruction::StoreVar(..) | 
                        Instruction::Return(..) | Instruction::Throw(..) | 
                        Instruction::Branch(..) | Instruction::CondBranch(..) |
                        Instruction::SetupTry(..) | Instruction::PopTry |
                        Instruction::Yield
                    )
                });
                
                if block.instructions.len() != initial_len {
                    changed = true;
                }
            }
        }
        
        changed
    }
}
