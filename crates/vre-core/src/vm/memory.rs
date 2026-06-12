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
    Box(Value),      // boxed value for closures
    Closure(usize, Vec<usize>), // instruction pointer, upvalue heap ids
}

impl HeapObject {
    /// Human-readable type label for leak reporting
    pub fn kind(&self) -> &'static str {
        match self {
            HeapObject::Array(_)    => "Array",
            HeapObject::String(_)   => "String",
            HeapObject::Struct(_)   => "Struct",
            HeapObject::Function(_) => "Function",
            HeapObject::Box(_)      => "Box",
            HeapObject::Closure(_, _) => "Closure",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GcObject {
    pub data: HeapObject,
    /// Serial number of this allocation (1-based, for diagnostics)
    pub alloc_id: usize,
}

/// Summary produced by `Heap::leak_report()`
#[derive(Debug, Default)]
pub struct LeakReport {
    /// Total number of objects still live at report time
    pub leaked_count: usize,
    /// Breakdown: how many leaked objects of each kind
    pub by_kind: HashMap<String, usize>,
    /// Total bytes allocated during VM lifetime
    pub total_allocations: usize,
}

impl LeakReport {
    pub fn has_leaks(&self) -> bool {
        self.leaked_count > 0
    }

    /// Pretty-print the report
    pub fn format(&self) -> String {
        if !self.has_leaks() {
            return format!(
                "Heap OK — {} allocations, 0 leaks.",
                self.total_allocations
            );
        }
        let mut lines = vec![
            format!(
                "⚠ Heap Leak Detected: {} object(s) not freed (of {} total allocations)",
                self.leaked_count, self.total_allocations
            ),
        ];
        let mut kinds: Vec<(&String, &usize)> = self.by_kind.iter().collect();
        kinds.sort_by_key(|(k, _)| k.as_str());
        for (kind, count) in kinds {
            lines.push(format!("  • {} × {}", count, kind));
        }
        lines.join("\n")
    }
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
        let alloc_id = self.total_allocations;
        let gc_obj = GcObject { data: obj, alloc_id };
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

    /// Produce a leak report: count all objects still live after execution.
    /// Call this after the VM halts to detect heap leaks.
    pub fn leak_report(&self) -> LeakReport {
        let mut by_kind: HashMap<String, usize> = HashMap::new();
        let mut leaked_count = 0;

        for slot in &self.objects {
            if let Some(gc_obj) = slot {
                leaked_count += 1;
                *by_kind.entry(gc_obj.data.kind().to_string()).or_insert(0) += 1;
            }
        }

        LeakReport {
            leaked_count,
            by_kind,
            total_allocations: self.total_allocations,
        }
    }

    /// Quick boolean: are there any live objects still on the heap?
    pub fn has_leaks(&self) -> bool {
        self.live_objects > 0
    }
}

