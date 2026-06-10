import os
import re

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Rename Token to TokenKind where it's an enum
    content = re.sub(r'pub enum Token \{', 'pub enum TokenKind {', content)
    
    # Replace Token:: with TokenKind::
    content = content.replace('Token::', 'TokenKind::')

    # Fix usages of Token as a type, but be careful not to replace the newly created TokenKind
    # Actually, if we change parser to accept TokenKind instead, we don't need to wrap it?
    # No, we want Token to be a struct containing line and col.
    
    with open(filepath, 'w') as f:
        f.write(content)

src_dir = 'crates/vre-compiler/src'
for root, _, files in os.walk(src_dir):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))

print("Tokens replaced!")
