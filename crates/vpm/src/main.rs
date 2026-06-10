use std::env;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct VyaumaManifest {
    name: String,
    version: String,
    description: Option<String>,
    dependencies: std::collections::HashMap<String, String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "init" => init_package(),
        "install" => install_packages(),
        "update" => update_packages(),
        "publish" => publish_package(),
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("Vyauma Package Manager (vpm)");
    println!("Usage:");
    println!("  vpm init      - Initialize a new package (creates vyauma.toml)");
    println!("  vpm install   - Install dependencies from vyauma.toml");
    println!("  vpm update    - Update dependencies");
    println!("  vpm publish   - Publish package to Vyauma registry");
}

fn init_package() {
    let path = Path::new("vyauma.toml");
    if path.exists() {
        println!("Error: vyauma.toml already exists in this directory.");
        return;
    }

    let manifest = VyaumaManifest {
        name: "my_package".to_string(),
        version: "0.1.0".to_string(),
        description: Some("A Vyauma package".to_string()),
        dependencies: std::collections::HashMap::new(),
    };

    let toml_string = toml::to_string(&manifest).expect("Failed to serialize manifest");
    fs::write(path, toml_string).expect("Failed to write vyauma.toml");
    println!("Initialized new Vyauma package in {}", env::current_dir().unwrap().display());
}

fn install_packages() {
    let path = Path::new("vyauma.toml");
    if !path.exists() {
        println!("Error: vyauma.toml not found. Run `vpm init` first.");
        return;
    }

    let content = fs::read_to_string(path).expect("Failed to read vyauma.toml");
    let manifest: VyaumaManifest = toml::from_str(&content).expect("Invalid vyauma.toml format");

    println!("Resolving dependencies for {} v{}...", manifest.name, manifest.version);
    
    let modules_dir = Path::new("vyauma_modules");
    if !modules_dir.exists() {
        fs::create_dir(modules_dir).expect("Failed to create vyauma_modules directory");
    }

    if manifest.dependencies.is_empty() {
        println!("No dependencies to install.");
        return;
    }

    for (dep_name, dep_version) in manifest.dependencies {
        println!("Fetching {}@{}...", dep_name, dep_version);
        let dep_dir = modules_dir.join(&dep_name);
        if !dep_dir.exists() {
            fs::create_dir_all(&dep_dir).expect("Failed to create dependency directory");
        }
        
        // Write a mock index file for the package so that the compiler can import it
        let mock_file_path = dep_dir.join("index.vym");
        let mock_code = format!("// Mock Vyauma Package '{}' v{}\nexport fn ping() {{ return \"pong\"; }}\n", dep_name, dep_version);
        fs::write(mock_file_path, mock_code).expect("Failed to write mock package file");
        
        println!("Installed {} v{}", dep_name, dep_version);
    }

    println!("Installation complete.");
}

fn update_packages() {
    println!("Checking for updates...");
    // Simulate update logic
    println!("All packages are up to date.");
}

fn publish_package() {
    let path = Path::new("vyauma.toml");
    if !path.exists() {
        println!("Error: vyauma.toml not found.");
        return;
    }

    println!("Packaging module...");
    println!("Simulating upload to Vyauma Registry...");
    println!("Successfully published package.");
}
