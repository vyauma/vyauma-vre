use std::fs;
use std::path::Path;

pub fn init_project(project_name: &str) {
    let project_dir = Path::new(project_name);
    
    if project_dir.exists() {
        println!("Error: Directory '{}' already exists.", project_name);
        std::process::exit(1);
    }

    if let Err(e) = fs::create_dir_all(project_dir.join("src")) {
        println!("Failed to create project directories: {}", e);
        std::process::exit(1);
    }

    // Create vyauma.toml
    let toml_path = project_dir.join("vyauma.toml");
    let toml_content = format!(r#"[package]
name = "{}"
version = "0.1.0"
authors = ["Your Name <you@example.com>"]

[dependencies]
# std = "1.0.0"
"#, project_name);

    fs::write(toml_path, toml_content).unwrap();

    // Create src/main.vya
    let main_path = project_dir.join("src").join("main.vya");
    let main_content = r#"// Vyauma Language - Main Entry Point
// Idiomatic Static Typing and Concurrency

fn main() {
    print("Welcome to the Vyauma Language!");
    
    let sum: Int64 = calculate(10, 20);
    print(sum);
    
    // Concurrency example using actor-like spawn/yield syntax
    spawn(background_task);
}

fn calculate(a: Int64, b: Int64) -> Int64 {
    return a + b;
}

fn background_task() {
    print("Running in background...");
    yield;
    print("Finished background task.");
}
"#;
    
    fs::write(main_path, main_content).unwrap();

    println!("Successfully initialized Vyauma project '{}'.", project_name);
    println!("Run it with: vre {}/src/main.vya", project_name);
}
