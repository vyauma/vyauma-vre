//! VM Memory Model
//!
//! Defines memory structures used during VM execution.
//! This layer is index-based and language-neutral.

use crate::error::{VreError, VreResult};
use super::value::Value;

/// Global variable storage (index-based)
#[derive(Debug)]
pub struct Globals {
    values: Vec<Value>,
}

impl Globals {
    pub fn new(size: usize) -> Self {
        Globals {
            values: vec![Value::Null; size],
        }
    }

    pub fn load(&self, index: usize) -> VreResult<Value> {
        self.values
            .get(index)
            .cloned()
            .ok_or(VreError::InvalidLocalAccess(index))
    }

    pub fn store(&mut self, index: usize, value: Value) -> VreResult<()> {
        if index >= self.values.len() {
            return Err(VreError::InvalidLocalAccess(index));
        }
        self.values[index] = value;
        Ok(())
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

/// Local variables for a single call frame
#[derive(Debug)]
pub struct Locals {
    values: Vec<Value>,
}

impl Locals {
    pub fn new(size: usize) -> Self {
        Locals {
            values: vec![Value::Null; size],
        }
    }

    pub fn elements(&self) -> &[Value] {
        &self.values
    }

    pub fn load(&self, index: usize) -> VreResult<Value> {
        self.values
            .get(index)
            .cloned()
            .ok_or(VreError::InvalidLocalAccess(index))
    }

    pub fn store(&mut self, index: usize, value: Value) -> VreResult<()> {
        if index >= self.values.len() {
            return Err(VreError::InvalidLocalAccess(index));
        }
        self.values[index] = value;
        Ok(())
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

/// Constant pool (read-only)
#[derive(Debug)]
pub struct ConstantPool {
    values: Vec<Value>,
}

impl ConstantPool {
    pub fn new(values: Vec<Value>) -> Self {
        ConstantPool { values }
    }

    pub fn get(&self, index: usize) -> VreResult<Value> {
        self.values
            .get(index)
            .cloned()
            .ok_or(VreError::InvalidConstantAccess(index))
    }

    pub fn values(&self) -> &[Value] {
        &self.values
    }
}

use std::collections::HashMap;

/// Dynamic Heap Object
#[derive(Debug, Clone, PartialEq)]
pub enum HeapObject {
    Array(Vec<Value>),
    String(String),
    Struct(HashMap<String, Value>),
    Function(usize), // instruction pointer
}

#[derive(Debug, Clone)]
pub struct GcObject {
    pub data: HeapObject,
}

/// Dynamic Memory Heap
#[derive(Debug)]
pub struct Heap {
    pub objects: Vec<Option<GcObject>>,
    pub free_list: Vec<usize>,
    pub live_objects: usize,
    pub total_allocations: usize,
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            objects: Vec::new(),
            free_list: Vec::new(),
            live_objects: 0,
            total_allocations: 0,
        }
    }

    pub fn allocate(&mut self, obj: HeapObject) -> usize {
        self.live_objects += 1;
        self.total_allocations += 1;
        let gc_obj = GcObject { data: obj };
        if let Some(id) = self.free_list.pop() {
            self.objects[id] = Some(gc_obj);
            id
        } else {
            let id = self.objects.len();
            self.objects.push(Some(gc_obj));
            id
        }
    }

    pub fn deallocate(&mut self, id: usize) -> VreResult<()> {
        if let Some(slot) = self.objects.get_mut(id) {
            if slot.is_some() {
                *slot = None;
                self.free_list.push(id);
                self.live_objects -= 1;
                return Ok(());
            }
        }
        Err(VreError::RuntimeFault)
    }

    pub fn get(&self, id: usize) -> VreResult<&HeapObject> {
        self.objects
            .get(id)
            .and_then(|opt| opt.as_ref())
            .map(|gc_obj| &gc_obj.data)
            .ok_or(VreError::RuntimeFault)
    }

    pub fn get_mut(&mut self, id: usize) -> VreResult<&mut HeapObject> {
        self.objects
            .get_mut(id)
            .and_then(|opt| opt.as_mut())
            .map(|gc_obj| &mut gc_obj.data)
            .ok_or(VreError::RuntimeFault)
    }

    pub fn sweep(&mut self, marked: &[bool]) {
        for (i, opt_obj) in self.objects.iter_mut().enumerate() {
            if opt_obj.is_some() {
                if i >= marked.len() || !marked[i] {
                    *opt_obj = None;
                    self.free_list.push(i);
                    self.live_objects -= 1;
                }
            }
        }
    }
}
