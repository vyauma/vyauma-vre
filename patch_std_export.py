import os
import re

std_dir = os.path.join(os.path.dirname(__file__), 'std')

for root, _, files in os.walk(std_dir):
    for f in files:
        if f.endswith('.vya') or f.endswith('.vym'):
            path = os.path.join(root, f)
            with open(path, 'r', encoding='utf-8') as file:
                content = file.read()
            
            # Add export keyword before fn, struct, class
            # Only if it's not already exported
            content = re.sub(r'^(?!\s*export\s+)(fn\s+[a-zA-Z_])', r'export \1', content, flags=re.MULTILINE)
            content = re.sub(r'^(?!\s*export\s+)(struct\s+[a-zA-Z_])', r'export \1', content, flags=re.MULTILINE)
            content = re.sub(r'^(?!\s*export\s+)(class\s+[a-zA-Z_])', r'export \1', content, flags=re.MULTILINE)
            
            with open(path, 'w', encoding='utf-8') as file:
                file.write(content)

print("Patched all std library files with export keyword.")
