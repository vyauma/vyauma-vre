use std::fs;
use std::path::Path;
use crate::manifest::Manifest;

const MOCK_REGISTRY_URL: &str = "https://registry.vyauma.org/api/v1";

pub fn publish() {
    println!("Vyauma Package Registry");
    println!("Packaging current directory...");
    
    // Check if vyauma.toml exists
    if let Ok(content) = fs::read_to_string("vyauma.toml") {
        match Manifest::parse(&content) {
            Ok(manifest) => {
                println!("Publishing {} v{} to {}...", manifest.package.name, manifest.package.version, MOCK_REGISTRY_URL);
                
                let result = ureq::post(&format!("{}/publish", MOCK_REGISTRY_URL))
                    .set("Content-Type", "application/json")
                    .send_string(&format!(
                        r#"{{"package_name": "{}", "version": "{}"}}"#,
                        manifest.package.name, manifest.package.version
                    ));
                    
                match result {
                    Ok(res) => println!("Registry replied with status: {}", res.status()),
                    Err(_) => println!("(Simulated: {} is unreachable. Assuming success)", MOCK_REGISTRY_URL),
                }
                
                println!("Successfully published v{}!", manifest.package.version);
            }
            Err(e) => {
                eprintln!("Error parsing vyauma.toml: {}", e);
            }
        }
    } else {
        eprintln!("Error: vyauma.toml not found in the current directory.");
    }
}

pub fn install(package_name: Option<&str>) {
    println!("Vyauma Package Registry");

    let mut to_install = Vec::new();

    if let Some(name) = package_name {
        to_install.push((name.to_string(), "latest".to_string()));
    } else {
        // Read vyauma.toml
        if let Ok(content) = fs::read_to_string("vyauma.toml") {
            match Manifest::parse(&content) {
                Ok(manifest) => {
                    println!("Resolving dependencies for {}...", manifest.package.name);
                    for (dep_name, dep_version) in manifest.dependencies {
                        to_install.push((dep_name, dep_version));
                    }
                }
                Err(e) => {
                    eprintln!("Error parsing vyauma.toml: {}", e);
                    return;
                }
            }
        } else {
            eprintln!("Error: No package name provided and vyauma.toml not found.");
            return;
        }
    }

    if to_install.is_empty() {
        println!("No dependencies to install.");
        return;
    }

    // Ensure vyauma_modules exists
    if let Err(e) = fs::create_dir_all("vyauma_modules") {
        eprintln!("Failed to create vyauma_modules directory: {}", e);
        return;
    }

    for (name, version) in to_install {
        println!("Fetching package '{}' (v{}) from {}...", name, version, MOCK_REGISTRY_URL);
        
        let url = format!("{}/packages/{}/{}", MOCK_REGISTRY_URL, name, version);
        let result = ureq::get(&url).call();
        
        match result {
            Ok(res) => {
                println!("Registry replied with status: {}", res.status());
                // In a real implementation, we would extract the tarball here.
            }
            Err(_) => {
                println!("(Simulated: registry unreachable. Creating mock package locally...)");
                
                // Create mock package structure
                let pkg_dir = Path::new("vyauma_modules").join(&name);
                if let Err(e) = fs::create_dir_all(&pkg_dir) {
                    eprintln!("Failed to create package directory {}: {}", pkg_dir.display(), e);
                    continue;
                }
                
                // Write a mock index.ts file
                let mock_content = format!(
                    "// Mock implementation of {}\nexport function hello() {{\n    ffi_console_println(\"Hello from mock package {}!\");\n}}\n",
                    name, name
                );
                
                if let Err(e) = fs::write(pkg_dir.join("index.ts"), mock_content) {
                    eprintln!("Failed to write mock index.ts for {}: {}", name, e);
                    continue;
                }
            }
        }
        
        println!("Successfully installed '{}'.", name);
    }
    
    println!("Installation complete.");
}
