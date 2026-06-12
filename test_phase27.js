// Phase 27 Test Script

ffi_console_println("--- Testing Native Database ---");

// Insert 2 records
let doc1 = ffi_json_stringify({ "name": "Alice", "role": "admin", "age": 30 });
let id1 = ffi_db_insert("users", doc1);
ffi_console_println("Inserted Alice:");
ffi_console_println(id1);

let doc2 = ffi_json_stringify({ "name": "Bob", "role": "user", "age": 25 });
let id2 = ffi_db_insert("users", doc2);
ffi_console_println("Inserted Bob:");
ffi_console_println(id2);

// Find Bob
let results_str = ffi_db_find("users", "name", "Bob");
ffi_console_println("Found Bob:");
ffi_console_println(results_str);
let results = ffi_json_parse(results_str);
ffi_assert_eq(ffi_array_len(results), 1, "Should find exactly 1 Bob");

// Delete Bob
let deleted = ffi_db_delete("users", "name", "Bob");
ffi_assert_eq(deleted, true, "Should have deleted Bob");

// Verify Bob is gone
let results_str_after = ffi_db_find("users", "name", "Bob");
let results_after = ffi_json_parse(results_str_after);
ffi_assert_eq(ffi_array_len(results_after), 0, "Bob should be gone");

// Clean up Alice
ffi_db_delete("users", "name", "Alice");

ffi_console_println("All Phase 27 tests passed!");

