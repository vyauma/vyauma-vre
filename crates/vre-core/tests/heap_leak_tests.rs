//! Heap Leak Detection Tests
//!
//! Verifies that Heap::leak_report() and has_leaks() correctly identify
//! un-freed objects after allocation and GC sweeps.

use vre_core::vm::memory::{Heap, HeapObject};

// ── helpers ─────────────────────────────────────────────────────────────────

fn make_array() -> HeapObject {
    HeapObject::Array(vec![])
}

fn make_struct() -> HeapObject {
    HeapObject::Struct(std::collections::HashMap::new())
}

fn make_string(s: &str) -> HeapObject {
    HeapObject::String(s.to_string())
}

// ── tests ────────────────────────────────────────────────────────────────────

#[test]
fn fresh_heap_has_no_leaks() {
    let heap = Heap::new();
    assert!(!heap.has_leaks());
    let report = heap.leak_report();
    assert!(!report.has_leaks());
    assert_eq!(report.leaked_count, 0);
    assert_eq!(report.total_allocations, 0);
}

#[test]
fn allocate_without_free_reports_leak() {
    let mut heap = Heap::new();
    heap.allocate(make_array());
    heap.allocate(make_array());

    assert!(heap.has_leaks());
    let report = heap.leak_report();
    assert!(report.has_leaks());
    assert_eq!(report.leaked_count, 2);
    assert_eq!(report.total_allocations, 2);
    assert_eq!(*report.by_kind.get("Array").unwrap(), 2);
}

#[test]
fn allocate_then_free_no_leak() {
    let mut heap = Heap::new();
    let id = heap.allocate(make_array());
    heap.deallocate(id).expect("dealloc should succeed");

    assert!(!heap.has_leaks());
    let report = heap.leak_report();
    assert!(!report.has_leaks());
    assert_eq!(report.leaked_count, 0);
    assert_eq!(report.total_allocations, 1); // still counts total
}

#[test]
fn mixed_types_leak_report_by_kind() {
    let mut heap = Heap::new();
    heap.allocate(make_array());
    heap.allocate(make_array());
    heap.allocate(make_struct());
    heap.allocate(make_string("hello"));

    let report = heap.leak_report();
    assert_eq!(report.leaked_count, 4);
    assert_eq!(*report.by_kind.get("Array").unwrap(), 2);
    assert_eq!(*report.by_kind.get("Struct").unwrap(), 1);
    assert_eq!(*report.by_kind.get("String").unwrap(), 1);
}

#[test]
fn partial_free_shows_remaining_leaks() {
    let mut heap = Heap::new();
    let id0 = heap.allocate(make_array());
    let _id1 = heap.allocate(make_struct());
    let id2 = heap.allocate(make_string("data"));

    // Free id0 and id2, leave id1 (Struct) alive
    heap.deallocate(id0).unwrap();
    heap.deallocate(id2).unwrap();

    let report = heap.leak_report();
    assert_eq!(report.leaked_count, 1);
    assert_eq!(*report.by_kind.get("Struct").unwrap(), 1);
    assert!(!report.by_kind.contains_key("Array"));
    assert!(!report.by_kind.contains_key("String"));
}

#[test]
fn sweep_clears_unmarked_objects() {
    let mut heap = Heap::new();
    let id0 = heap.allocate(make_array());  // will be marked
    let _id1 = heap.allocate(make_struct()); // will NOT be marked — leaked

    // Mark only id0 as live
    let mut marked = vec![false; heap.objects.len()];
    marked[id0] = true;

    heap.sweep(&marked);

    let report = heap.leak_report();
    // After sweep: only id0 (Array) remains — id1 (Struct) was swept
    assert_eq!(report.leaked_count, 1);
    assert_eq!(*report.by_kind.get("Array").unwrap(), 1);
    assert!(!report.by_kind.contains_key("Struct"));
}

#[test]
fn leak_report_format_no_leaks() {
    let heap = Heap::new();
    let report = heap.leak_report();
    assert!(report.format().contains("Heap OK"));
    assert!(report.format().contains("0 leaks"));
}

#[test]
fn leak_report_format_with_leaks() {
    let mut heap = Heap::new();
    heap.allocate(make_array());
    heap.allocate(make_struct());
    let report = heap.leak_report();
    let fmt = report.format();
    assert!(fmt.contains("Heap Leak Detected"));
    assert!(fmt.contains("Array"));
    assert!(fmt.contains("Struct"));
}

#[test]
fn alloc_id_is_sequential() {
    let mut heap = Heap::new();
    let id0 = heap.allocate(make_array());
    let id1 = heap.allocate(make_array());
    // Verify GcObject alloc_ids are sequential
    let obj0 = heap.objects[id0].as_ref().unwrap();
    let obj1 = heap.objects[id1].as_ref().unwrap();
    assert_eq!(obj0.alloc_id, 1);
    assert_eq!(obj1.alloc_id, 2);
}
