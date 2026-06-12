use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Represents a node in the VRE cluster.
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
}

/// Manages cluster membership and discovery.
pub struct ClusterNode {
    pub local_info: NodeInfo,
    pub peers: Arc<Mutex<HashMap<String, NodeInfo>>>,
}

impl ClusterNode {
    pub fn new(id: &str, address: &str) -> Self {
        ClusterNode {
            local_info: NodeInfo {
                id: id.to_string(),
                address: address.to_string(),
            },
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new peer node in the cluster.
    pub fn add_peer(&self, node: NodeInfo) {
        let mut peers = self.peers.lock().unwrap();
        println!("ClusterNode {}: Discovered peer {} at {}", self.local_info.id, node.id, node.address);
        peers.insert(node.id.clone(), node);
    }

    /// List all known peers.
    pub fn list_peers(&self) -> Vec<NodeInfo> {
        let peers = self.peers.lock().unwrap();
        peers.values().cloned().collect()
    }
}
