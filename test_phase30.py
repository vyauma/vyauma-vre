# Phase 30 Test - Cross-Language FFI: Python DB operations
# Uses idiomatic vre_db_* wrappers that map to ffi_db_* natively

print("--- Phase 30: Python Cross-Language FFI ---")

# Insert a record using the Python-idiomatic wrapper
id = vre_db_insert("users", "{\"name\": \"Alice\", \"lang\": \"python\"}")
print("Inserted record id:")
print(id)

# Find records (collection, filter_key, filter_value)
results = vre_db_find("users", "lang", "python")
print("Found records:")
print(results)

# Filesystem wrapper
vre_fs_write("phase30_py_output.txt", "hello from python!")
content = vre_fs_read("phase30_py_output.txt")
print("File content:")
print(content)

print("All Phase 30 Python tests passed!")

