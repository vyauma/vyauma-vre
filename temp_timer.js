function delayed() { ffi_console_println("Timer fired!"); } function main() { ffi_set_timeout("delayed", 50); ffi_console_println("Main spawned timer"); } main();
