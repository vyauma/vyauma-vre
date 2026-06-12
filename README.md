# Vyauma Runtime Engine (VRE)

VRE is the reference runtime engine for the Vyauma ecosystem.
It is designed to be minimal, deterministic, dependency-free, and long-lived.

This repository intentionally prioritizes:
- correctness over cleverness
- clarity over abstraction
- stability over velocity

---

## What VRE Is

VRE is a:
- bytecode-driven virtual runtime (with statically typed instructions)
- platform-agnostic execution engine
- foundation for Vyauma language and ecosystem

It is NOT:
- a framework
- a dependency-heavy runtime
- an experimental playground

---

## Core Principles

1. **Minimal Viable System (MVS)**
   - Strict standard library usage where possible
   - Minimal external dependencies (e.g., `mio` for async IO, `region` for JIT memory, `serde` for JSON)
   - No heavyweight frameworks

2. **Strict Layer Separation**
   - Bytecode is raw, semantic-free, and heavily typed (e.g., `AddI32`, `AddF64`)
   - VM owns execution without dynamic type dispatch overhead
   - Capabilities are explicit and registered

3. **Deterministic Execution**
   - Single-threaded
   - Predictable memory model
   - No hidden side effects

4. **Blueprint-Driven Development**
   - Folder and file structure is locked
   - No restructuring without approval
   - One file implemented at a time

## Recent Highlights

- **Core Engine Complete (Phases 1-30):** The VRE runtime is now fully implemented with comprehensive JIT, Async execution, and strict Capability Sandboxing enforcement.
- **Multi-Language Support:** Cross-language compilation and FFI execution capabilities natively supporting TypeScript, JavaScript, PHP, and Python codebases.
- **Ecosystem Tooling:** Shipped full developer tooling including a Package Registry (`vre install`, `vre publish`), Language Server Protocol (LSP), and Debug Adapter Protocol (DAP).
- **Platform Agnostic & Extensible:** Added foundational support for Cloud, Mobile, Embedded architectures, and WebAssembly, alongside expanding the native Standard Library with Document Database, Regex, and Cryptography utilities.

---

## Repository Structure

See `docs/architecture.md` for a detailed explanation.

All contributors MUST respect the existing structure.
New files or changes to structure require explicit approval.

---

## Development Rules (Mandatory)

- One file per change
- No speculative features
- No premature optimization
- Refactoring must be purposeful
- Correctness > performance > convenience

---

## File Extensions

- **`.vym`**: Vyauma source code
- **`.vasm`**: Human-readable Vyauma assembly language
- **`.vyma`**: Compiled binary bytecode

---

## Usage & Toolchain

The repository provides several tools to interact with the engine:

### 1. The Main CLI (`vre`)
Run a Vyauma source file or compiled bytecode directly:
```bash
# Compile and run a source file
cargo run --bin vre -- script.vym

# Run pre-compiled bytecode
cargo run --bin vre -- script.vyma
```

### 2. The Interactive Debugger (`vre-debug`)
Step through bytecode, inspect stack, locals, and memory:
```bash
cargo run --bin vre-debug -- script.vym
# Useful commands:
# s (step), c (continue), b <addr> (break), bl (break list), st (stack), l (locals)
```

### 3. The Assembler (`vre-asm`)
Compile raw `.vasm` assembly into `.vyma` binary format:
```bash
cargo run --bin vre-asm -- input.vasm output.vyma
```

---

## License

Apache License 2.0
