pub mod docker;
pub mod k8s;
pub mod serverless;

pub fn deploy(input_path: &str, target: &str) {
    println!("Generating cloud deployment scaffold for {} targeting {}...", input_path, target);
    match target {
        "docker" => docker::deploy_docker(input_path),
        "k8s" => k8s::deploy_k8s(input_path),
        "serverless" => serverless::deploy_serverless(input_path),
        _ => {
            println!("Unknown target '{}'. Use 'docker', 'k8s', or 'serverless'.", target);
        }
    }
}
