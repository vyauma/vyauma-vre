import re

with open('crates/vre-asm/src/assembler.rs', 'r', encoding='utf-8') as f:
    content = f.read()

typed_ops_replacement = """OpCode::AddI32 | OpCode::SubI32 | OpCode::MulI32 | OpCode::DivI32 | OpCode::ModI32 | OpCode::NegI32 |
                OpCode::AddI64 | OpCode::SubI64 | OpCode::MulI64 | OpCode::DivI64 | OpCode::ModI64 | OpCode::NegI64 |
                OpCode::AddF32 | OpCode::SubF32 | OpCode::MulF32 | OpCode::DivF32 | OpCode::ModF32 | OpCode::NegF32 |
                OpCode::AddF64 | OpCode::SubF64 | OpCode::MulF64 | OpCode::DivF64 | OpCode::ModF64 | OpCode::NegF64 |
                OpCode::EqualI32 | OpCode::NotEqualI32 | OpCode::LessI32 | OpCode::LessEqualI32 | OpCode::GreaterI32 | OpCode::GreaterEqualI32 |
                OpCode::EqualI64 | OpCode::NotEqualI64 | OpCode::LessI64 | OpCode::LessEqualI64 | OpCode::GreaterI64 | OpCode::GreaterEqualI64 |
                OpCode::EqualF32 | OpCode::NotEqualF32 | OpCode::LessF32 | OpCode::LessEqualF32 | OpCode::GreaterF32 | OpCode::GreaterEqualF32 |
                OpCode::EqualF64 | OpCode::NotEqualF64 | OpCode::LessF64 | OpCode::LessEqualF64 | OpCode::GreaterF64 | OpCode::GreaterEqualF64 |
                OpCode::EqualStr | OpCode::NotEqualStr |"""

old_ops = """OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
                OpCode::Mod | OpCode::Neg | OpCode::Equal | OpCode::NotEqual |
                OpCode::Less | OpCode::LessEqual | OpCode::Greater | OpCode::GreaterEqual |"""
content = content.replace(old_ops, typed_ops_replacement)

old_ops2 = """OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
          OpCode::Mod | OpCode::Neg | OpCode::Equal | OpCode::NotEqual |
          OpCode::Less | OpCode::LessEqual | OpCode::Greater | OpCode::GreaterEqual |"""
content = content.replace(old_ops2, typed_ops_replacement)

old_parse = """        "add" => Some(OpCode::Add),
        "sub" => Some(OpCode::Sub),
        "mul" => Some(OpCode::Mul),
        "div" => Some(OpCode::Div),
        "mod" => Some(OpCode::Mod),
        "neg" => Some(OpCode::Neg),
        "equal" | "eq" => Some(OpCode::Equal),
        "notequal" | "ne" => Some(OpCode::NotEqual),
        "less" | "lt" => Some(OpCode::Less),
        "lessequal" | "le" => Some(OpCode::LessEqual),
        "greater" | "gt" => Some(OpCode::Greater),
        "greaterequal" | "ge" => Some(OpCode::GreaterEqual),"""

new_parse = """        "addi32" => Some(OpCode::AddI32), "subi32" => Some(OpCode::SubI32), "muli32" => Some(OpCode::MulI32), "divi32" => Some(OpCode::DivI32), "modi32" => Some(OpCode::ModI32), "negi32" => Some(OpCode::NegI32),
        "addi64" => Some(OpCode::AddI64), "subi64" => Some(OpCode::SubI64), "muli64" => Some(OpCode::MulI64), "divi64" => Some(OpCode::DivI64), "modi64" => Some(OpCode::ModI64), "negi64" => Some(OpCode::NegI64),
        "addf32" => Some(OpCode::AddF32), "subf32" => Some(OpCode::SubF32), "mulf32" => Some(OpCode::MulF32), "divf32" => Some(OpCode::DivF32), "modf32" => Some(OpCode::ModF32), "negf32" => Some(OpCode::NegF32),
        "addf64" => Some(OpCode::AddF64), "subf64" => Some(OpCode::SubF64), "mulf64" => Some(OpCode::MulF64), "divf64" => Some(OpCode::DivF64), "modf64" => Some(OpCode::ModF64), "negf64" => Some(OpCode::NegF64),
        "equali32" => Some(OpCode::EqualI32), "notequali32" => Some(OpCode::NotEqualI32), "lessi32" => Some(OpCode::LessI32), "lessequali32" => Some(OpCode::LessEqualI32), "greateri32" => Some(OpCode::GreaterI32), "greaterequali32" => Some(OpCode::GreaterEqualI32),
        "equali64" => Some(OpCode::EqualI64), "notequali64" => Some(OpCode::NotEqualI64), "lessi64" => Some(OpCode::LessI64), "lessequali64" => Some(OpCode::LessEqualI64), "greateri64" => Some(OpCode::GreaterI64), "greaterequali64" => Some(OpCode::GreaterEqualI64),
        "equalf32" => Some(OpCode::EqualF32), "notequalf32" => Some(OpCode::NotEqualF32), "lessf32" => Some(OpCode::LessF32), "lessequalf32" => Some(OpCode::LessEqualF32), "greaterf32" => Some(OpCode::GreaterF32), "greaterequalf32" => Some(OpCode::GreaterEqualF32),
        "equalf64" => Some(OpCode::EqualF64), "notequalf64" => Some(OpCode::NotEqualF64), "lessf64" => Some(OpCode::LessF64), "lessequalf64" => Some(OpCode::LessEqualF64), "greaterf64" => Some(OpCode::GreaterF64), "greaterequalf64" => Some(OpCode::GreaterEqualF64),
        "equalstr" => Some(OpCode::EqualStr), "notequalstr" => Some(OpCode::NotEqualStr),"""

content = content.replace(old_parse, new_parse)

with open('crates/vre-asm/src/assembler.rs', 'w', encoding='utf-8') as f:
    f.write(content)
