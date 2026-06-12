// Phase 26 Test Script

ffi_console_println("--- Testing Crypto ---");
let hash = ffi_crypto_sha256("hello world");
ffi_console_println(hash);
ffi_assert_eq(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9", "SHA-256 hash does not match expected output");

let random_b64 = ffi_crypto_random_bytes(16);
ffi_console_println(random_b64);

ffi_console_println("--- Testing Regex ---");
let is_match = ffi_regex_is_match("^[a-z]+$", "hello");
ffi_assert_eq(is_match, true, "Regex ^[a-z]+$ should match 'hello'");

let no_match = ffi_regex_is_match("^[a-z]+$", "hello123");
ffi_assert_eq(no_match, false, "Regex ^[a-z]+$ should not match 'hello123'");

let replaced = ffi_regex_replace("world", "hello world", "VRE");
ffi_console_println(replaced);
ffi_assert_eq(replaced, "hello VRE", "Regex replacement failed");

ffi_console_println("All Phase 26 tests passed!");
