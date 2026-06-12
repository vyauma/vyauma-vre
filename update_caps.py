import re

with open("crates/vre-cli/src/native.rs", "r", encoding="utf-8") as f:
    content = f.read()

caps = {
    "ffi_fs_read_file": "fs.read",
    "ffi_fs_exists": "fs.read",
    "ffi_fs_size": "fs.read",
    "ffi_fs_write_file": "fs.write",
    "ffi_fs_delete": "fs.write",
    "ffi_net_listen": "net.listen",
    "ffi_net_accept": "net.accept",
    "ffi_net_connect": "net.connect",
    "ffi_db_insert": "db.write",
    "ffi_db_find": "db.read",
    "ffi_db_delete": "db.write",
    "ffi_process_spawn": "sys.process",
    "ffi_process_exit": "sys.process",
    "ffi_env_get": "sys.env"
}

lines = content.split('\n')

for i in range(len(lines)):
    line = lines[i]
    if 'config.insert_ffi("' in line:
        match = re.search(r'config\.insert_ffi\("([^"]+)"\.to_string\(\)', line)
        if match:
            func_name = match.group(1)
            if func_name in caps:
                cap = caps[func_name]
                # Replace start
                lines[i] = line.replace('config.insert_ffi("{}".to_string()'.format(func_name), 'config.register_ffi("{}", '.format(func_name))
                
                # Find matching end
                brace_count = 0
                started = False
                for j in range(i, len(lines)):
                    brace_count += lines[j].count('{')
                    brace_count -= lines[j].count('}')
                    if '{' in lines[j]:
                        started = True
                    if started and brace_count == 0:
                        lines[j] = lines[j].replace('});', '}, vec![vre_core::capability::capability::Capability::new("' + cap + '")]);')
                        break

with open("crates/vre-cli/src/native.rs", "w", encoding="utf-8") as f:
    f.write('\n'.join(lines))
