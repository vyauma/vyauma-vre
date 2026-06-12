use std::fs;
use std::path::Path;

pub fn deploy_serverless(input_path: &str) {
    let deploy_dir = Path::new("deploy/serverless");
    if let Err(e) = fs::create_dir_all(deploy_dir) {
        println!("Failed to create serverless deploy directory: {}", e);
        return;
    }

    // Copy the VYM file
    let dest_file = deploy_dir.join("app.vym");
    if let Err(e) = fs::copy(input_path, &dest_file) {
        println!("Could not copy input file to deploy folder (might need compilation first): {}", e);
    }

    // Generate serverless.yml (for Serverless Framework targeting AWS Lambda)
    let sls_path = deploy_dir.join("serverless.yml");
    let sls_content = r#"service: vre-serverless-app

provider:
  name: aws
  runtime: provided.al2
  architecture: arm64
  memorySize: 256
  timeout: 10
  environment:
    VRE_ENV: production

package:
  patterns:
    - '!**/*'
    - 'bootstrap'
    - 'app.vym'

functions:
  api:
    handler: handler
    events:
      - httpApi:
          path: /
          method: '*'
      - httpApi:
          path: /{proxy+}
          method: '*'
"#;
    fs::write(sls_path, sls_content).unwrap();

    // Generate AWS Lambda custom runtime bootstrap
    let bootstrap_path = deploy_dir.join("bootstrap");
    let bootstrap_content = r#"#!/bin/sh
# AWS Lambda custom runtime bootstrap for Vyauma Runtime Engine (VRE)
set -euo pipefail

# Initialization - execute VRE in AWS Lambda mode
# Assumes `vre` binary is included in the package or Lambda Layer
export VRE_LAMBDA_API="http://${AWS_LAMBDA_RUNTIME_API}/2018-06-01/runtime/invocation/"

exec vre app.vym --lambda
"#;
    fs::write(bootstrap_path, bootstrap_content).unwrap();

    // Make bootstrap executable if running on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(mut perms) = fs::metadata(&bootstrap_path).map(|m| m.permissions()) {
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&bootstrap_path, perms);
        }
    }

    println!("Serverless deployment scaffold generated at deploy/serverless/");
}
