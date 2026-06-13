use axum::{
    routing::{get, post},
    Router, Json, extract::{Path, Query}, http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
    description: String,
    checksum: String,
}

#[derive(Serialize, Deserialize)]
struct PublishPayload {
    metadata: PackageInfo,
    archive_base64: String, // Mock payload for tar.gz
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

struct AppState {
    // Map of package name -> List of versions
    packages: HashMap<String, Vec<PackageInfo>>,
}

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(AppState {
        packages: HashMap::new(),
    }));

    let app = Router::new()
        .route("/publish", post(publish_package))
        .route("/packages/:name", get(get_package))
        .route("/search", get(search_packages))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9090").await.unwrap();
    println!("Vyauma Registry listening on http://127.0.0.1:9090");
    axum::serve(listener, app).await.unwrap();
}

async fn publish_package(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Json(payload): Json<PublishPayload>,
) -> StatusCode {
    let mut state = state.lock().await;
    let pkg = payload.metadata;
    
    // Ensure uniqueness
    let versions = state.packages.entry(pkg.name.clone()).or_insert_with(Vec::new);
    if versions.iter().any(|v| v.version == pkg.version) {
        return StatusCode::CONFLICT;
    }
    
    versions.push(pkg);
    StatusCode::OK
}

async fn get_package(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<Vec<PackageInfo>>, StatusCode> {
    let state = state.lock().await;
    if let Some(versions) = state.packages.get(&name) {
        Ok(Json(versions.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn search_packages(
    axum::extract::State(state): axum::extract::State<SharedState>,
    Query(query): Query<SearchQuery>,
) -> Json<Vec<PackageInfo>> {
    let state = state.lock().await;
    let q = query.q.to_lowercase();
    
    let mut results = Vec::new();
    for versions in state.packages.values() {
        if let Some(latest) = versions.last() {
            if latest.name.to_lowercase().contains(&q) || latest.description.to_lowercase().contains(&q) {
                results.push(latest.clone());
            }
        }
    }
    
    Json(results)
}
