//! Phase 5 — Async Runtime Tests
//!
//! Tests for the cooperative task scheduler, coroutines, fibers,
//! event loop timers, Spawn/Yield/Await opcodes, and sleep_async.

use vre_core::config::VreConfig;
use vre_core::error::VreResult;
use vre_core::vm::vm::VirtualMachine;
use vre_core::vm::value::Value;
use vre_core::bytecode::opcode::OpCode;
use vre_core::{Capability, CapabilityRegistry};
use vre_core::scheduler::Scheduler;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_caps() -> CapabilityRegistry {
    let mut caps = CapabilityRegistry::new();
    caps.grant(Capability::new("io.read"));
    caps.grant(Capability::new("io.write"));
    caps
}

fn run_async_vm(constants: Vec<Value>, instructions: Vec<u8>) -> VreResult<Option<Value>> {
    let mut vm = VirtualMachine::new(
        VreConfig::default(),
        instructions,
        constants,
        vec![],
        make_caps(),
        std::collections::HashMap::new(),
    )
    .unwrap();
    vm.execute()?;
    Ok(vm.peek_stack().cloned().ok())
}

// ─── Scheduler Unit Tests ─────────────────────────────────────────────────────

#[test]
fn test_scheduler_spawn_and_pop() {
    let mut sched = Scheduler::new();
    let id1 = sched.spawn(0, 1024, 256);
    let id2 = sched.spawn(100, 1024, 256);
    assert_eq!(sched.task_count(), 2);

    let t1 = sched.pop_next().expect("Should have task 1");
    assert_eq!(t1.id, id1);
    let t2 = sched.pop_next().expect("Should have task 2");
    assert_eq!(t2.id, id2);
    assert!(sched.pop_next().is_none());
}

#[test]
fn test_scheduler_block_and_unblock() {
    let mut sched = Scheduler::new();
    let id = sched.spawn(0, 1024, 256);
    assert_eq!(sched.task_count(), 1);

    let task = sched.pop_next().unwrap();
    sched.block_task(task);
    assert_eq!(sched.task_count(), 1);
    assert!(!sched.has_ready_tasks());
    assert!(sched.has_active_tasks());
    assert!(sched.is_blocked(id));

    sched.unblock_task(id);
    assert!(sched.has_ready_tasks());
    assert!(!sched.is_blocked(id));
}

#[test]
fn test_scheduler_yield_task() {
    let mut sched = Scheduler::new();
    let _id = sched.spawn(0, 1024, 256);

    let task = sched.pop_next().unwrap();
    sched.yield_task(task);

    // Should be back in the ready queue
    assert!(sched.has_ready_tasks());
    assert_eq!(sched.task_count(), 1);
}

#[test]
fn test_scheduler_timer_fires_and_unblocks() {
    use std::time::Duration;

    let mut sched = Scheduler::new();
    let _id = sched.spawn(0, 1024, 256);
    let task = sched.pop_next().unwrap();
    let task_id = task.id;

    // Schedule a 0-ms timer (fires immediately)
    sched.schedule_timer(task, 0);
    assert!(sched.is_blocked(task_id));
    assert_eq!(sched.task_count(), 1);

    // Let the timer expire
    std::thread::sleep(Duration::from_millis(5));
    sched.check_timers();

    // Task should be back in the ready queue
    assert!(!sched.is_blocked(task_id));
    assert!(sched.has_ready_tasks());
}

#[test]
fn test_scheduler_await_waiter_unblocked_on_completion() {
    let mut sched = Scheduler::new();
    let target_id = sched.spawn(0, 1024, 256);
    let waiter_id = sched.spawn(100, 1024, 256);

    let waiter_task = {
        // Pop the target first
        let _ = sched.pop_next();
        // Pop the waiter
        sched.pop_next().unwrap()
    };
    assert_eq!(waiter_task.id, waiter_id);

    // Register waiter as blocked on target
    sched.await_task(waiter_task, target_id);
    assert!(sched.is_blocked(waiter_id));
    assert_eq!(sched.task_count(), 1); // only waiter remains

    // Simulate target completing: unblock waiters manually
    if let Some(waiters) = sched.task_waiters.remove(&target_id) {
        for wid in waiters {
            sched.unblock_task(wid);
        }
    }

    assert!(!sched.is_blocked(waiter_id));
    assert!(sched.has_ready_tasks());
}

#[test]
fn test_scheduler_iter_blocked_tasks() {
    let mut sched = Scheduler::new();
    sched.spawn(0, 1024, 256);
    let t1 = sched.pop_next().unwrap();
    let id1 = t1.id;
    sched.block_task(t1);

    sched.spawn(100, 1024, 256);
    let t2 = sched.pop_next().unwrap();
    let id2 = t2.id;
    sched.block_task(t2);

    let blocked_ids: Vec<u64> = sched.iter_blocked_tasks().map(|t| t.id).collect();
    assert!(blocked_ids.contains(&id1));
    assert!(blocked_ids.contains(&id2));
    assert_eq!(blocked_ids.len(), 2);
}

// ─── VM Opcode Tests ──────────────────────────────────────────────────────────

/// spawn a task, yield from main, then halt.
/// The spawned task pushes 99.0, the main task pushes 10.0.
/// After yield, main resumes and 10.0 should be on top.
#[test]
fn test_vm_spawn_and_yield() {
    let constants = vec![Value::Float64(10.0), Value::Float64(99.0)];
    let instructions = vec![
        // Main (offset 0):
        OpCode::Spawn as u8, 0, 0, 0, 11,    // Spawn → task at offset 11, push task_id
        OpCode::Pop as u8,                     // offset 5
        OpCode::Push as u8, 0, 0,             // offset 6-8
        OpCode::Yield as u8,                   // offset 9
        OpCode::Halt as u8,                    // offset 10

        // Spawned task (offset 11):
        OpCode::Push as u8, 0, 1,             // offset 11-13
        OpCode::Return as u8,                  // offset 14
    ];

    let result = run_async_vm(constants, instructions).unwrap();
    // Main task's stack top (10.0) should still be there after halt
    assert_eq!(result, Some(Value::Float64(10.0)));
}

/// Spawn two tasks. Each pushes a value. Verify cooperative scheduling.
#[test]
fn test_vm_two_coroutines_interleave() {
    // Layout: main spawns A and B, then halts.
    // A pushes 1.0 then returns.
    // B pushes 2.0 then returns.
    // After all tasks complete, main stack should be empty (we just Halt).
    let constants = vec![Value::Float64(1.0), Value::Float64(2.0)];
    let offset_a: u32 = 13;
    let offset_b: u32 = 17;
    let instructions = vec![
        // Main (offset 0):
        OpCode::Spawn as u8,
            ((offset_a >> 24) & 0xFF) as u8,
            ((offset_a >> 16) & 0xFF) as u8,
            ((offset_a >>  8) & 0xFF) as u8,
            (offset_a        & 0xFF) as u8,  // spawn A → offset 13
        OpCode::Pop as u8,

        OpCode::Spawn as u8,
            ((offset_b >> 24) & 0xFF) as u8,
            ((offset_b >> 16) & 0xFF) as u8,
            ((offset_b >>  8) & 0xFF) as u8,
            (offset_b        & 0xFF) as u8,  // spawn B → offset 17
        OpCode::Pop as u8,
        OpCode::Halt as u8,                   // offset 12

        // Task A (offset 13):
        OpCode::Push as u8, 0, 0,            // push 1.0
        OpCode::Return as u8,                 // offset 16

        // Task B (offset 17):
        OpCode::Push as u8, 0, 1,            // push 2.0
        OpCode::Return as u8,
    ];

    // Should run without errors
    assert!(run_async_vm(constants, instructions).is_ok());
}

/// Test Await: spawn a task that produces a result, await it from main.
#[test]
fn test_vm_await_task_result() {
    // Main spawns a worker, then awaits the worker's return value.
    // Worker pushes 42.0, then Returns.
    // After await resolves, 42.0 should land on main's stack.
    let constants = vec![Value::Float64(42.0)];
    let worker_offset: u32 = 7;
    let instructions = vec![
        // Main (offset 0):
        OpCode::Spawn as u8,
            ((worker_offset >> 24) & 0xFF) as u8,
            ((worker_offset >> 16) & 0xFF) as u8,
            ((worker_offset >>  8) & 0xFF) as u8,
            (worker_offset        & 0xFF) as u8,  // offset 0-4
        // Stack: [task_id]
        OpCode::Await as u8,               // offset 5
        // Stack: [42.0]  (worker's result pushed here)
        OpCode::Halt as u8,                // offset 6

        // Worker (offset 7):
        OpCode::Push as u8, 0, 0,         // offset 7-9
        OpCode::Return as u8,             // offset 10
    ];

    let result = run_async_vm(constants, instructions).unwrap();
    assert_eq!(result, Some(Value::Float64(42.0)));
}

/// sleep_async (syscall 0x08) should not block the VM — it parks the current
/// task and resumes it after the timeout.
#[test]
fn test_vm_sleep_async_resumes() {
    let constants = vec![Value::Float64(1.0)]; // sleep 1 ms
    let instructions = vec![
        OpCode::Push as u8, 0, 0,     // push 1.0 (ms)
        OpCode::Syscall as u8, 0x08,  // sleep_async(1) — non-blocking
        OpCode::Pop as u8,            // discard 0.0 return value
        // After sleep resolves, push a sentinel value
        OpCode::Push as u8, 0, 0,     // push 1.0 again as sentinel
        OpCode::Halt as u8,
    ];

    let result = run_async_vm(constants, instructions).unwrap();
    assert_eq!(result, Some(Value::Float64(1.0)));
}

/// GC must not collect heap objects held by sleeping (blocked) tasks.
#[test]
fn test_gc_does_not_collect_blocked_task_heap_refs() {
    use vre_core::bytecode::opcode::OpCode;

    // Main spawns a worker that allocates an array on the heap.
    // Main does sleep_async to park itself briefly.
    // GC runs while main is blocked.
    // After resuming, main should still work correctly (no UB/crash).
    //
    // This is a smoke test: if GC incorrectly sweeps blocked-task roots, 
    // the array reference in the worker's stack would be invalidated.

    let worker_offset: u32 = 19;
    let constants = vec![
        Value::Float64(1.0),   // 0: array size 1
        Value::Float64(0.0),   // 1: index 0
        Value::Float64(99.0),  // 2: stored value
        Value::Float64(5.0),   // 3: sleep 5ms
    ];

    let instructions = vec![
        // Main (offset 0):
        OpCode::Spawn as u8,
            ((worker_offset >> 24) & 0xFF) as u8,
            ((worker_offset >> 16) & 0xFF) as u8,
            ((worker_offset >>  8) & 0xFF) as u8,
            (worker_offset        & 0xFF) as u8, // offset 0-4
        OpCode::Pop as u8,                       // offset 5

        // Main sleeps 5ms (non-blocking)
        OpCode::Push as u8, 0, 3,     // offset 6-8
        OpCode::Syscall as u8, 0x08,  // offset 9-10
        OpCode::Pop as u8,            // offset 11

        // GC — should NOT collect worker's array
        OpCode::Syscall as u8, 0x07,  // offset 12-13
        OpCode::Pop as u8,            // offset 14

        OpCode::Push as u8, 0, 2,     // offset 15-17
        OpCode::Halt as u8,           // offset 18

        // Worker (offset 19):
        OpCode::Push as u8, 0, 0,     // offset 19-21
        OpCode::NewArray as u8,       // offset 22
        OpCode::Dup as u8,            // offset 23
        OpCode::Push as u8, 0, 1,     // offset 24-26
        OpCode::Push as u8, 0, 2,     // offset 27-29
        OpCode::StoreElement as u8,   // offset 30
        OpCode::Return as u8,         // offset 31
    ];

    // Must run without panicking or causing heap corruption
    let result = run_async_vm(constants, instructions).unwrap();
    assert_eq!(result, Some(Value::Float64(99.0)));
}
