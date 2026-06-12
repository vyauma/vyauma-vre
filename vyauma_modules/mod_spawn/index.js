export function spawn_worker() {
    vre_spawn(function() {
        ffi_console_println("Worker running from module!");
    });
}
