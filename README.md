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
- bytecode-driven virtual runtime
- platform-agnostic execution engine
- foundation for Vyauma language and ecosystem

It is NOT:
- a framework
- a dependency-heavy runtime
- an experimental playground

---

## Core Principles

1. **Minimal Viable System (MVS)**
   - Rust standard library only
   - No external crates
   - No async
   - No concurrency

2. **Strict Layer Separation**
   - Bytecode is raw and semantic-free
   - VM owns semantics and execution
   - Capabilities are explicit and registered

3. **Deterministic Execution**
   - Single-threaded
   - Predictable memory model
   - No hidden side effects

4. **Blueprint-Driven Development**
   - Folder and file structure is locked
   - No restructuring without approval
   - One file implemented at a time

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
- No external dependencies
- No refactoring unless requested
- Correctness > performance > convenience

---

## License

Apache License 2.0
