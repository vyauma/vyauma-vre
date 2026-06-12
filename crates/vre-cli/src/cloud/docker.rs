use std::fs;
use std::path::Path;

pub fn deploy_docker(input_path: &str) {
    let deploy_dir = Path::new("deploy/docker");
    if let Err(e) = fs::create_dir_all(deploy_dir) {
        println!("Failed to create docker deploy directory: {}", e);
        return;
    }

    // Copy the VYM file (simulate compilation / copying)
    let dest_file = deploy_dir.join("app.vym");
    if let Err(e) = fs::copy(input_path, &dest_file) {
        println!("Could not copy input file to deploy folder (might need compilation first): {}", e);
    }

    // Generate Dockerfile
    let dockerfile_path = deploy_dir.join("Dockerfile");
    let dockerfile_content = r#"# syntax=docker/dockerfile:1
FROM debian:bookworm-slim

# Install any required OS dependencies for VRE Core (e.g., OpenSSL)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Assume the precompiled VRE binary is placed alongside or downloaded
# For this scaffold, we assume `vre` is available locally or built from source
# COPY vre /usr/local/bin/vre

# Copy compiled VRE bytecode
COPY app.vym /app/app.vym

# Expose standard port for VRE HTTP servers
EXPOSE 8080

# Run VRE Engine
CMD ["vre", "app.vym"]
"#;
    fs::write(dockerfile_path, dockerfile_content).unwrap();

    // Generate .dockerignore
    let dockerignore_path = deploy_dir.join(".dockerignore");
    let dockerignore_content = r#".git
target/
deploy/
"#;
    fs::write(dockerignore_path, dockerignore_content).unwrap();

    println!("Docker deployment scaffold generated at deploy/docker/");
}
