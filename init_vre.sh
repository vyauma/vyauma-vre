#!/bin/bash

set -e

echo "Initializing Vyauma VRE repository structure..."

mkdir -p docs
mkdir -p crates/vre-core/src/{bytecode,vm,loader,capability}
mkdir -p crates/vre-cli/src

# Root files
touch README.md LICENSE .gitignore

# Docs
touch docs/{vision.md,architecture.md,roadmap.md,glossary.md}

# vre-core
touch crates/vre-core/Cargo.toml
touch crates/vre-core/src/{lib.rs,error.rs,config.rs}

# Bytecode
touch crates/vre-core/src/bytecode/{mod.rs,opcode.rs,instruction.rs}

# VM
touch crates/vre-core/src/vm/{mod.rs,vm.rs,stack.rs,memory.rs,value.rs}

# Loader
touch crates/vre-core/src/loader/{mod.rs,loader.rs}

# Capability
touch crates/vre-core/src/capability/{mod.rs,capability.rs,registry.rs}

# CLI
touch crates/vre-cli/Cargo.toml
touch crates/vre-cli/src/main.rs

echo "VRE structure created successfully."
