use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    StructInstance(StructInstanceData),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructInstanceData {
    pub name: String,
    pub fields: HashMap<String, Value>,
}

#[derive(Debug)]
pub struct Object {
    pub is_marked: bool,
    pub obj_type: ObjectType,
}

pub struct Heap {
    pub objects: Vec<Option<Box<Object>>>,
    pub free_slots: Vec<usize>,
    pub allocated_objects: usize,
    pub gc_threshold: usize,
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            objects: Vec::new(),
            free_slots: Vec::new(),
            allocated_objects: 0,
            gc_threshold: 1000,
        }
    }

    pub fn allocate(&mut self, obj_type: ObjectType) -> usize {
        let obj = Box::new(Object {
            is_marked: false,
            obj_type,
        });

        self.allocated_objects += 1;

        if let Some(idx) = self.free_slots.pop() {
            self.objects[idx] = Some(obj);
            idx
        } else {
            self.objects.push(Some(obj));
            self.objects.len() - 1
        }
    }

    pub fn get(&self, handle: usize) -> &Object {
        self.objects[handle].as_ref().expect("Invalid heap handle")
    }

    pub fn get_mut(&mut self, handle: usize) -> &mut Object {
        self.objects[handle].as_mut().expect("Invalid heap handle")
    }

    pub fn mark_value(&mut self, value: &Value) {
        if let Value::HeapRef(handle) = value {
            self.mark_object(*handle);
        }
    }

    pub fn mark_object(&mut self, handle: usize) {
        let obj = if let Some(o) = &mut self.objects[handle] {
            o
        } else {
            return;
        };

        if obj.is_marked {
            return; // Already marked, break cycle
        }
        obj.is_marked = true;

        // Trace children
        // Since Rust doesn't easily allow mutating 'self' while holding a reference to 'obj',
        // we can clone the references to trace, or use indices.
        // It's safer to extract the child values to a local Vec, then mark them.
        let mut children_to_mark = Vec::new();
        match &obj.obj_type {
            ObjectType::Array(elements) => {
                for val in elements {
                    if let Value::HeapRef(child_handle) = val {
                        children_to_mark.push(*child_handle);
                    }
                }
            }
            ObjectType::Map(map) => {
                for val in map.values() {
                    if let Value::HeapRef(child_handle) = val {
                        children_to_mark.push(*child_handle);
                    }
                }
            }
            ObjectType::StructInstance(data) => {
                for val in data.fields.values() {
                    if let Value::HeapRef(child_handle) = val {
                        children_to_mark.push(*child_handle);
                    }
                }
            }
            ObjectType::String(_) => {}
        }

        // Recursively mark children
        for child_handle in children_to_mark {
            self.mark_object(child_handle);
        }
    }

    pub fn sweep(&mut self) -> usize {
        let mut swept = 0;
        for i in 0..self.objects.len() {
            if let Some(obj) = &mut self.objects[i] {
                if obj.is_marked {
                    obj.is_marked = false; // reset for next cycle
                } else {
                    self.objects[i] = None;
                    self.free_slots.push(i);
                    self.allocated_objects -= 1;
                    swept += 1;
                }
            }
        }
        
        // Adjust threshold
        self.gc_threshold = std::cmp::max(self.allocated_objects * 2, 1000);
        swept
    }

    pub fn clear_marks(&mut self) {
        for obj_opt in &mut self.objects {
            if let Some(obj) = obj_opt {
                obj.is_marked = false;
            }
        }
    }
}
