# Vyauma Bytecode Specification — v0.1

This document specifies the binary format of Vyauma bytecode files.
It is the authoritative reference for bytecode producers (compilers, assemblers)
and consumers (the VRE loader).

---

## 1. File Format

All multi-byte integers are **big-endian**.

```
┌─────────────────────────────────────────────────────┐
│  Header (16 bytes, fixed)                           │
│    [4 bytes] Magic: "VYMA" (0x5659_4D41)            │
│    [1 byte]  Version major                          │
│    [1 byte]  Version minor                          │
│    [1 byte]  Version patch                          │
│    [1 byte]  Reserved (must be 0x00)                │
│    [4 bytes] Entry point (byte offset in code)      │
├─────────────────────────────────────────────────────┤
│  Constant Pool                                      │
│    [4 bytes] Constant count (N)                     │
│    [N × variable] Constant records (see §2)         │
├─────────────────────────────────────────────────────┤
│  Code Section                                       │
│    [4 bytes] Instruction byte length (L)            │
│    [L bytes] Raw instruction bytes                  │
└─────────────────────────────────────────────────────┘
```

### 1.1 Version

| Field | v0.1 value |
|-------|-----------|
| major | 1         |
| minor | 0         |
| patch | 1         |

The loader validates **major** only. Minor and patch are informational.

### 1.2 Entry Point

The entry point is a byte offset into the code section, where execution begins.
An entry point of `0` starts execution at the first instruction.

---

## 2. Constant Pool

Each constant record starts with a 1-byte **type tag**:

| Tag    | Type    | Payload               |
|--------|---------|-----------------------|
| `0x00` | Null    | _(none)_              |
| `0x01` | Bool    | 1 byte (0=false, else true) |
| `0x02` | Number  | 8 bytes IEEE 754 f64, big-endian |
| `0xFF` | Ref     | 4 bytes u32 (opaque reference ID) |

Any other tag is a **malformed bytecode** error.

---

## 3. Instruction Encoding

Each instruction is:

```
[1 byte] Opcode
[N bytes] Operand (see table below)
```

Operands are consumed **inline** from the instruction stream (not a separate section).

---

## 4. Opcode Table

### 4.1 Stack Operations

| Opcode | Byte   | Operand | Stack Effect | Description |
|--------|--------|---------|--------------|-------------|
| `Push` | `0x01` | u16 — constant pool index | `( -- value )` | Push constant onto stack |
| `Pop`  | `0x02` | _(none)_ | `( value -- )` | Discard top of stack |
| `Dup`  | `0x03` | _(none)_ | `( value -- value value )` | Duplicate top of stack |

### 4.2 Local Variable Operations

| Opcode | Byte | Operand | Stack Effect | Description |
|--------|------|---------|--------------|-------------|
| `LoadLocal`  | `0x10` | u16 — local index | `( -- value )` | Push local variable |
| `StoreLocal` | `0x11` | u16 — local index | `( value -- )` | Pop into local variable |

Locals are per-call-frame. Accessing an index ≥ frame's local count is an error.

### 4.3 Arithmetic Operations

All arithmetic ops consume two `Number` values and push one `Number`.
Passing a non-`Number` type is a `TypeMismatch` error.

| Opcode | Byte   | Description |
|--------|--------|-------------|
| `Add`  | `0x20` | `a + b` |
| `Sub`  | `0x21` | `a - b` |
| `Mul`  | `0x22` | `a * b` |
| `Div`  | `0x23` | `a / b` (DivisionByZero if b = 0.0) |
| `Mod`  | `0x24` | `a % b` (DivisionByZero if b = 0.0) |
| `Neg`  | `0x25` | `-a` (unary, pops one value) |

Stack convention: `a` was pushed before `b`.
`Sub` computes `(second-from-top) - (top)`.

### 4.4 Comparison Operations

Consume two values, push one `Bool`.

| Opcode         | Byte   | Description |
|----------------|--------|-------------|
| `Equal`        | `0x30` | `a == b` (structural equality) |
| `NotEqual`     | `0x31` | `a != b` |
| `Less`         | `0x32` | `a < b` (Numbers only) |
| `LessEqual`    | `0x33` | `a <= b` (Numbers only) |
| `Greater`      | `0x34` | `a > b` (Numbers only) |
| `GreaterEqual` | `0x35` | `a >= b` (Numbers only) |

`Equal` and `NotEqual` work on any value type pair.
Ordered comparisons (`Less`, `LessEqual`, `Greater`, `GreaterEqual`) require both values to be `Number`.

### 4.5 Control Flow

| Opcode   | Byte   | Operand | Description |
|----------|--------|---------|-------------|
| `Jump`   | `0x40` | u32 — target byte offset | Unconditional jump |
| `JumpIf` | `0x41` | u32 — target byte offset | Jump if top of stack is `true` (`Bool`) |
| `Call`   | `0x42` | u32 target + u16 local count | Push call frame, jump to target |
| `Return` | `0x43` | _(none)_ | Pop call frame, resume at return IP |

**`Call` encoding:**
```
[0x42] [u32: target offset] [u16: local variable count for new frame]
```

**`Return` at top level** (no active call frame) is treated as `Halt`.

### 4.6 System

| Opcode | Byte   | Description |
|--------|--------|-------------|
| `Nop`  | `0xF0` | No operation |
| `Halt` | `0xFF` | Stop execution |

---

## 5. Runtime Limits (v0.1 defaults)

| Limit | Default |
|-------|---------|
| Max stack depth | 1024 values |
| Max local variables per frame | 256 |
| Max call depth | 256 frames |

These are configurable via `VreConfig`.

---

## 6. Error Conditions

| Condition | Error |
|-----------|-------|
| File < 16 bytes | `BytecodeTooShort` |
| Wrong magic | `InvalidMagicNumber` |
| Wrong major version | `InvalidBytecodeVersion` |
| Unknown opcode byte | `InvalidOpcode(byte)` |
| Unknown constant tag | `MalformedBytecode` |
| Stack push beyond limit | `StackOverflow` |
| Pop/peek on empty stack | `StackUnderflow` |
| Local index out of bounds | `InvalidLocalAccess(index)` |
| Constant index out of bounds | `InvalidConstantAccess(index)` |
| Divide/mod by zero | `DivisionByZero` |
| Jump target ≥ code length | `InvalidJumpTarget(offset)` |
| Call depth exceeded | `StackOverflow` |
| Non-Number on numeric op | `TypeMismatch` |
| Non-Bool on JumpIf | `TypeMismatch` |

---

## 7. Example: Add Two Numbers

Program: push 2.0, push 3.0, add, halt.

**Constant pool:**
```
count = 2
  [0x02] [0x40 00 00 00 00 00 00 00]  → Number(2.0)
  [0x02] [0x40 08 00 00 00 00 00 00]  → Number(3.0)
```

**Code:**
```
0x01 0x00 0x00   Push #0 (2.0)
0x01 0x00 0x01   Push #1 (3.0)
0x20             Add
0xFF             Halt
```

Result: stack holds `Number(5.0)`.
