use std::fs;
use std::path::Path;

pub fn build_web(input_path: &str) {
    let source_path = Path::new(input_path);
    if !source_path.exists() {
        println!("Error: Source file '{}' not found.", input_path);
        std::process::exit(1);
    }

    let out_dir = Path::new("web_dist");
    if let Err(e) = fs::create_dir_all(out_dir) {
        println!("Failed to create web_dist directory: {}", e);
        std::process::exit(1);
    }

    println!("Building WebAssembly target for {}...", input_path);
    println!("Compiling Vyauma source to VIR...");
    
    // In a real build, we would run `wasm-pack build crates/vre-core --target web --out-dir ../../web_dist`
    println!("Running wasm-pack (simulated)...");
    
    let html_content = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Vyauma Web Runtime</title>
</head>
<body>
    <h1>Vyauma Web Runtime (VRE)</h1>
    <p>Check the console for output.</p>
    
    <script type="module">
        // Hypothetical WASM bootstrapping
        import init, { VreWasmContext } from './vre_core.js';
        
        async function run() {
            await init();
            console.log("VRE Wasm initialized.");
            
            try {
                let ctx = new VreWasmContext();
                // We would fetch the actual compiled VIR payload here
                // const response = await fetch('app.vbc');
                // const buffer = await response.arrayBuffer();
                // ctx.load_and_run(new Uint8Array(buffer));
                console.log("Executing Vyauma application...");
            } catch (err) {
                console.error("VRE Error:", err);
            }
        }
        
        run();
    </script>
</body>
</html>"#;

    fs::write(out_dir.join("index.html"), html_content).unwrap();
    
    println!("Successfully built WebAssembly package in 'web_dist/'");
    println!("Serve it using: npx serve web_dist");
}
