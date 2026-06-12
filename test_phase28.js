// Phase 28 Test Script

ffi_console_println("--- Testing Task Concurrency ---");

ffi_console_println("Going to sleep for 500ms...");
let start = ffi_time_now_ms();

ffi_task_sleep(500);

let end = ffi_time_now_ms();
let elapsed = end - start;

ffi_console_println("Woke up!");
ffi_console_println("Elapsed MS:");
ffi_console_println(elapsed);

// Because timers are not perfectly exact, we just assert elapsed is >= 490
if (elapsed >= 490) {
    ffi_console_println("All Phase 28 tests passed!");
} else {
    ffi_assert(elapsed >= 490, "Elapsed time was less than 500ms");
}
