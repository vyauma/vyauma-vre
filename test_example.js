// Phase 25 Test Script

// Verify boolean assertion
ffi_assert(1 + 1 == 2, "Math is broken!");

// Verify equality assertion
let actual = 42;
let expected = 42;
ffi_assert_eq(actual, expected, "Numbers do not match");

// Test string equality
ffi_assert_eq("hello", "hello", "Strings do not match");

ffi_console_println("All assertions passed!");
