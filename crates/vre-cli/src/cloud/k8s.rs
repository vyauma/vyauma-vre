use std::fs;
use std::path::Path;

pub fn deploy_k8s(input_path: &str) {
    let deploy_dir = Path::new("deploy/k8s");
    if let Err(e) = fs::create_dir_all(deploy_dir) {
        println!("Failed to create k8s deploy directory: {}", e);
        return;
    }

    // Generate deployment.yaml
    let deployment_path = deploy_dir.join("deployment.yaml");
    let deployment_content = format!(r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: vre-app-deployment
  labels:
    app: vre-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: vre-app
  template:
    metadata:
      labels:
        app: vre-app
    spec:
      containers:
      - name: vre-app
        image: your-docker-registry/vre-app:latest
        ports:
        - containerPort: 8080
        env:
        - name: VRE_ENV
          value: "production"
        resources:
          requests:
            memory: "64Mi"
            cpu: "100m"
          limits:
            memory: "128Mi"
            cpu: "250m"
"#);
    fs::write(deployment_path, deployment_content).unwrap();

    // Generate service.yaml
    let service_path = deploy_dir.join("service.yaml");
    let service_content = r#"apiVersion: v1
kind: Service
metadata:
  name: vre-app-service
spec:
  type: LoadBalancer
  selector:
    app: vre-app
  ports:
    - protocol: TCP
      port: 80
      targetPort: 8080
"#;
    fs::write(service_path, service_content).unwrap();

    println!("Kubernetes deployment scaffold generated at deploy/k8s/ for input {}", input_path);
}
