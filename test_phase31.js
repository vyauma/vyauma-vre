function background_task() {
    ffi_console_println("Background task started.");
    ffi_task_sleep(50);
    ffi_console_println("Background task finished.");
}

function main() {
    ffi_console_println("Main task started.");
    var task_id = ffi_task_spawn("background_task");
    ffi_console_println("Spawned task with ID: ");
    ffi_console_println(task_id);
    ffi_console_println("Main task waiting...");
    ffi_task_sleep(100);
    ffi_console_println("Main task done.");
}

main();
