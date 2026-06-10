import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Revert renaming TokenKind back to Token
    content = content.replace('TokenKind', 'Token')
    
    with open(filepath, 'w') as f:
        f.write(content)

src_dir = 'crates/vre-compiler/src'
for root, _, files in os.walk(src_dir):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))

print("Tokens restored!")
