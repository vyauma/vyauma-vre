use std::env;
use std::fs;
use std::path::Path;

use vre_core::config::VreConfig;
use vre_core::vm::value::Value;
use vre_core::bytecode::OpCode;
use vre_core::BytecodeLoader;
use vre_vol::consume_external_call;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Minimal CLI: `generate <path>` -> write sample bytecode file and exit.
    // Otherwise: optional path -> run program; flags: --cap N, --verbose, --format json|plain
    if args.len() >= 2 && args[1] == "generate" {
        if args.len() < 3 {
            eprintln!("usage: generate <path>");
            return;
        }
        if let Err(e) = write_sample_bytecode(&args[2]) {
            eprintln!("failed to write sample bytecode: {}", e);
        } else {
            println!("wrote sample bytecode to {}", &args[2]);
        }
        return;
    }

    // parse flags
    let mut file_path: Option<String> = None;
    let mut cap_to_grant: u8 = 42;
    let mut verbose = false;
    let mut format_json = false;
    let mut grant_bytecode_caps = false;
    let mut allow_loader_opt_in = false;
    let mut policy_allow_list: Vec<u8> = Vec::new();
    let mut policy_persist_path: Option<String> = None;
    let mut policy_load_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--cap" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<u8>() {
                        cap_to_grant = n;
                    }
                    i += 1;
                }
            }
            "--verbose" => verbose = true,
            "--format" => {
                if i + 1 < args.len() {
                    if args[i + 1].as_str() == "json" {
                        format_json = true;
                    }
                    i += 1;
                }
            }
            "--grant-bytecode-caps" => {
                grant_bytecode_caps = true;
            }
            "--allow-loader-opt-in" => {
                allow_loader_opt_in = true;
            }
            "--policy-allow" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<u8>() {
                        policy_allow_list.push(n);
                    }
                    i += 1;
                }
            }
            "--policy-persist-audit" => {
                if i + 1 < args.len() {
                    policy_persist_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--policy-load-allow" => {
                if i + 1 < args.len() {
                    policy_load_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            other => {
                // first non-flag arg is file path
                if file_path.is_none() {
                    file_path = Some(other.to_string());
                }
            }
        }
        i += 1;
    }

    // If a path is provided, attempt to load bytecode from file using BytecodeLoader.
    let (config, constants, instructions, caps) = if let Some(path_str) = file_path {
        let path = Path::new(&path_str);
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("failed to read {}: {}", path.display(), e);
                return;
            }
        };

        let (loaded, used_lenient) = match BytecodeLoader::load_with_opt_in(&bytes, allow_loader_opt_in) {
            Ok((l, used)) => (l, used),
            Err(e) => {
                eprintln!("bytecode load error: {}", e);
                return;
            }
        };

        if used_lenient {
            eprintln!("warning: strict loader validation failed; using lenient opt-in parse");
        }

        // If requested, collect capability ids via the loader's public API.
        let mut loader_caps: Vec<u8> = Vec::new();
        if grant_bytecode_caps {
            match BytecodeLoader::collect_caps(&bytes) {
                Ok(c) => loader_caps = c,
                Err(e) => {
                    eprintln!("bytecode cap collection error: {}", e);
                    return;
                }
            }
        }

        // Strict entry_point enforcement: must be within instructions bounds.
        if loaded.entry_point >= loaded.instructions.len() {
            eprintln!("invalid entry_point {}: out of bounds (instructions length {})", loaded.entry_point, loaded.instructions.len());
            return;
        }

        let instr = loaded.instructions[loaded.entry_point..].to_vec();
        let caps = if loader_caps.is_empty() { loaded.caps.clone() } else { loader_caps };
        (VreConfig::new(), loaded.constants, instr, caps)
    } else {
        // Fallback inline program
        (
            VreConfig::new(),
            vec![Value::Number(3.14)],
            vec![
                OpCode::Push as u8,
                0u8,
                OpCode::ExternalCall as u8,
                cap_to_grant,
                1u8,
                OpCode::Halt as u8,
            ],
            Vec::new(),
        )
    };

    let mut vm = vre_core::vm::VirtualMachine::new(config, constants, instructions, 0);
    vm.grant_capability(cap_to_grant);

    // Apply VOL policy when granting caps discovered by the loader.
    // If a policy allow-list path was provided, load it and merge with CLI
    if let Some(p) = policy_load_path {
        match vre_vol::policy::Policy::load_allow_list(std::path::Path::new(&p)) {
            Ok(mut list) => {
                // merge with existing CLI-provided allow list
                list.extend(policy_allow_list.iter().cloned());
                let policy = vre_vol::policy::Policy::new(list);
                if verbose { println!("loaded policy allow-list from {}", p); }
                // shadow previous policy var by reassigning
                
                // Apply caps using this policy below: replace local 'policy' via block
                if grant_bytecode_caps {
                    for c in caps.clone().into_iter() {
                        if policy.allows(c) {
                            vm.grant_capability(c);
                            policy.record(format!("granted cap {} via loader", c));
                        } else {
                            policy.record(format!("denied cap {} via loader", c));
                            eprintln!("policy: denied granting cap {} (use --policy-allow to allow)", c);
                        }
                    }
                }

                // Persist audit if requested
                if let Some(pp) = policy_persist_path {
                    if let Err(e) = policy.persist_audit(std::path::Path::new(&pp)) {
                        eprintln!("failed to persist policy audit: {}", e);
                    } else {
                        if verbose { println!("policy audit persisted to {}", pp); }
                    }
                }
            }
            Err(e) => {
                eprintln!("failed to load policy allow-list {}: {}", p, e);
            }
        }
    } else {
        let policy = vre_vol::policy::Policy::new(policy_allow_list);
        if grant_bytecode_caps {
            for c in caps.into_iter() {
                if policy.allows(c) {
                    vm.grant_capability(c);
                    policy.record(format!("granted cap {} via loader", c));
                } else {
                    policy.record(format!("denied cap {} via loader", c));
                    eprintln!("policy: denied granting cap {} (use --policy-allow to allow)", c);
                }
            }
        }

        // Optionally persist audit log
        if let Some(p) = policy_persist_path {
            if let Err(e) = policy.persist_audit(std::path::Path::new(&p)) {
                eprintln!("failed to persist policy audit: {}", e);
            } else {
                if verbose { println!("policy audit persisted to {}", p); }
            }
        }
    }
    

    // Execute until external call emitted
    vm.execute().expect("execution failed");

    // Provide a host handler that prints args and returns two results
    fn handler(cap: u8, args: &[Value]) -> vre_core::VreResult<Vec<Value>> {
        println!("Host handler invoked for cap={} args={:?}", cap, args);
        Ok(vec![Value::Number(123.0), Value::Bool(true)])
    }

    // Consume the external call via VOL helper
    consume_external_call(&mut vm, handler).expect("consume failed");

    // Resume VM and finish
    vm.execute().expect("resume failed");

    // Inspect results on the stack
    let mut results = Vec::new();
    while let Ok(v) = vm.pop_top() {
        results.push(v);
    }

    if verbose {
        println!("raw stack results (top..bottom): {:?}", results);
    }

    if format_json {
        // simple JSON array output (Number/Bool/Null only)
        let mut out = Vec::new();
        for v in results.iter().rev() {
            match v {
                Value::Number(n) => out.push(format!("{}", n)),
                Value::Bool(b) => out.push(format!("{}", b)),
                Value::Null => out.push("null".to_string()),
                Value::Ref(r) => out.push(format!("{{\"ref\":{}}}", r)),
            }
        }
        println!("[{}]", out.join(", "));
    } else {
        for v in results.iter().rev() {
            println!("Stack result: {:?}", v);
        }
    }
}

fn write_sample_bytecode(path: &str) -> std::io::Result<()> {
    // Build a minimal bytecode: magic, version 1.0.0, reserved, entry_point=0,
    // 1 constant (Number 3.14), instructions same as inline demo.
    use std::io::Write;

    let mut buf = Vec::new();

    // Magic
    buf.extend(&0x5659_4D41u32.to_be_bytes());
    // version major/minor/patch and reserved
    buf.push(1u8);
    buf.push(0u8);
    buf.push(0u8);
    buf.push(0u8);
    // entry_point (u32)
    buf.extend(&(0u32.to_be_bytes()));

    // constant count (u32)
    buf.extend(&(1u32.to_be_bytes()));
    // constant: tag 0x02 = Number, then f64
    buf.push(0x02u8);
    buf.extend(&3.14f64.to_be_bytes());

    // instructions
    let instr = vec![
        OpCode::Push as u8,
        0u8,
        OpCode::ExternalCall as u8,
        42u8,
        1u8,
        OpCode::Halt as u8,
    ];
    buf.extend(&(instr.len() as u32).to_be_bytes());
    buf.extend(&instr);

    let mut f = std::fs::File::create(path)?;
    f.write_all(&buf)?;
    Ok(())
}
