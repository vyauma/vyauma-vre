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
| `LoadLocal`  | `0x10` | u16 — local index | `( -- value )` | Push generic local variable |
| `StoreLocal` | `0x11` | u16 — local index | `( value -- )` | Pop into local variable |
| `LoadLocalI32`|`0x12` | u16 — local index | `( -- value )` | Push I32 local variable |
| `LoadLocalI64`|`0x13` | u16 — local index | `( -- value )` | Push I64 local variable |
| `LoadLocalF32`|`0x14` | u16 — local index | `( -- value )` | Push F32 local variable |
| `LoadLocalF64`|`0x15` | u16 — local index | `( -- value )` | Push F64 local variable |
| `LoadLocalStr`|`0x16` | u16 — local index | `( -- value )` | Push Str local variable |

Locals are per-call-frame. Accessing an index ≥ frame's local count is an error.

### 4.3 Arithmetic Operations

All arithmetic ops are statically typed based on primitive types (I32, I64, F32, F64).
They consume two values of the matched type and push one value. Passing the wrong type causes a `TypeMismatch` error.

| Opcode Range | Category | Bytes |
|--------------|----------|-------|
| `AddI32` .. `NegI32` | Int32 | `0x20` .. `0x25` |
| `AddI64` .. `NegI64` | Int64 | `0x26` .. `0x2B` |
| `AddF32` .. `NegF32` | Float32 | `0x2C` .. `0x31` |
| `AddF64` .. `NegF64` | Float64 | `0x32` .. `0x37` |

Stack convention: `a` was pushed before `b`.
`Sub` computes `(second-from-top) - (top)`.

### 4.4 Comparison Operations

Consume two values, push one `Bool`. Operands must strictly match the typed operation.

| Opcode Range | Category | Bytes |
|--------------|----------|-------|
| `EqualI32` .. `GreaterEqualI32` | Int32 | `0x38` .. `0x3D` |
| `EqualI64` .. `GreaterEqualI64` | Int64 | `0x3E` .. `0x43` |
| `EqualF32` .. `GreaterEqualF32` | Float32 | `0x44` .. `0x49` |
| `EqualF64` .. `GreaterEqualF64` | Float64 | `0x4A` .. `0x4F` |
| `EqualStr`, `NotEqualStr` | String | `0x50` .. `0x51` |
| `AndBool`, `OrBool` | Logical | `0x52` .. `0x53` |

### 4.5 Control Flow

| Opcode   | Byte   | Operand | Description |
|----------|--------|---------|-------------|
| `Jump`   | `0x60` | u32 — target offset | Unconditional jump |
| `JumpIf` | `0x61` | u32 — target offset | Jump if top of stack is `true` (`Bool`) |
| `Call`   | `0x62` | u32 target + u16 locals | Push call frame, jump to target |
| `Return` | `0x63` | _(none)_ | Pop call frame, resume at return IP |
| `Spawn`  | `0x64` | u32 target | Spawn coroutine/task |
| `Yield`  | `0x65` | _(none)_ | Yield coroutine execution |
| `Await`  | `0x66` | _(none)_ | Await async task |

**`Return` at top level** (no active call frame) is treated as `Halt`.

### 4.6 Heap, Objects and FFI

| Opcode | Byte   | Description |
|--------|--------|-------------|
| `NewArray` | `0x70` | Allocate new array |
| `LoadElement` | `0x71` | Load array element |
| `StoreElement` | `0x72` | Store array element |
| `NewStruct` | `0x73` | Allocate new struct |
| `LoadProperty` | `0x74` | Load struct property |
| `StoreProperty` | `0x75` | Store struct property |
| `CallNative` | `0x76` | FFI native function call |

### 4.7 Exception Handling

| Opcode | Byte   | Description |
|--------|--------|-------------|
| `TryStart` | `0x80` | Push try-catch block |
| `TryEnd`   | `0x81` | Pop try-catch block |
| `Throw`    | `0x82` | Throw exception |

### 4.8 System

| Opcode | Byte   | Description |
|--------|--------|-------------|
| `Nop`  | `0xF0` | No operation |
| `Syscall` | `0xF1` | System capability call |
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

## 7. Example: Add Two Float64 Numbers

Program: push 2.0, push 3.0, addf64, halt.

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
0x32             AddF64
0xFF             Halt
```

Result: stack holds `Number(5.0)`.
