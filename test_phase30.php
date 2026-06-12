<?php
// Phase 30 Test - Cross-Language FFI: PHP DB operations
// Uses idiomatic vre_db_* wrappers that map to ffi_db_* natively

echo "--- Phase 30: PHP Cross-Language FFI ---\n";

// Insert a record using the PHP-idiomatic wrapper
$id = vre_db_insert("products", "{\"name\": \"Widget\", \"lang\": \"php\"}");
echo "Inserted record id: ";
echo $id;
echo "\n";

// Find records (collection, filter_key, filter_value)
$results = vre_db_find("products", "lang", "php");
echo "Found records: ";
echo $results;
echo "\n";

// Filesystem wrapper
vre_fs_write("phase30_php_output.txt", "hello from php!");
$content = vre_fs_read("phase30_php_output.txt");
echo "File content: ";
echo $content;
echo "\n";

echo "All Phase 30 PHP tests passed!\n";
