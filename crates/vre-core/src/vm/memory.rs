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
}

/// Dynamic Memory Heap
#[derive(Debug)]
pub struct Heap {
    pub objects: Vec<Option<HeapObject>>,
    pub free_list: Vec<u32>,
    pub live_objects: usize,
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            objects: Vec::new(),
            free_list: Vec::new(),
            live_objects: 0,
        }
    }

    pub fn allocate(&mut self, obj: HeapObject) -> u32 {
        self.live_objects += 1;
        if let Some(id) = self.free_list.pop() {
            self.objects[id as usize] = Some(obj);
            id
        } else {
            let id = self.objects.len() as u32;
            self.objects.push(Some(obj));
            id
        }
    }

    pub fn get(&self, id: u32) -> VreResult<&HeapObject> {
        self.objects
            .get(id as usize)
            .and_then(|opt| opt.as_ref())
            .ok_or(VreError::RuntimeFault)
    }

    pub fn get_mut(&mut self, id: u32) -> VreResult<&mut HeapObject> {
        self.objects
            .get_mut(id as usize)
            .and_then(|opt| opt.as_mut())
            .ok_or(VreError::RuntimeFault)
    }

    pub fn sweep(&mut self, marked: &[bool]) {
        for (i, opt_obj) in self.objects.iter_mut().enumerate() {
            if opt_obj.is_some() {
                if i >= marked.len() || !marked[i] {
                    // Unreachable object, reclaim
                    *opt_obj = None;
                    self.free_list.push(i as u32);
                    self.live_objects -= 1;
                }
            }
        }
    }
}
