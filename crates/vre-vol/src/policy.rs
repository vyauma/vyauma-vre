use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::path::Path;
use std::fs::File;
use std::io::Write;

/// Simple VOL policy: allow-list of capability ids and an audit log.
#[derive(Clone)]
pub struct Policy {
    allow_list: Arc<Vec<u8>>,
    audit: Arc<Mutex<Vec<String>>>,
    // optional time-limited grants: cap -> expiry instant
    ttl_grants: Arc<Mutex<Vec<(u8, Instant)>>>,
}

impl Policy {
    /// Create a new policy with an explicit allow-list.
    pub fn new(allow_list: Vec<u8>) -> Self {
        Policy { allow_list: Arc::new(allow_list), audit: Arc::new(Mutex::new(Vec::new())), ttl_grants: Arc::new(Mutex::new(Vec::new())) }
    }

    

    /// Record an audit entry describing a decision.
    pub fn record(&self, entry: String) {
        if let Ok(mut a) = self.audit.lock() {
            a.push(entry);
        }
    }

    /// Retrieve the audit log snapshot.
    pub fn audit_log(&self) -> Vec<String> {
        match self.audit.lock() {
            Ok(a) => a.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Add a time-limited grant for `cap` lasting `dur` from now.
    pub fn grant_with_ttl(&self, cap: u8, dur: Duration) {
        if let Ok(mut g) = self.ttl_grants.lock() {
            g.push((cap, Instant::now() + dur));
        }
        self.record(format!("granted cap {} with ttl {:?}", cap, dur));
    }

    /// Check ttl grants and see if cap is currently granted by TTL.
    fn is_granted_by_ttl(&self, cap: u8) -> bool {
        if let Ok(g) = self.ttl_grants.lock() {
            let now = Instant::now();
            for (c, exp) in g.iter() {
                if *c == cap && *exp > now { return true; }
            }
        }
        false
    }

    /// Persist the audit log to a file (append mode).
    pub fn persist_audit(&self, path: &Path) -> std::io::Result<()> {
        let log = self.audit_log();
        let mut f = File::create(path)?;
        for l in log.into_iter() {
            writeln!(f, "{}", l)?;
        }
        Ok(())
    }

    /// Load an allow-list from a newline- or comma-separated file of integer ids.
    pub fn load_allow_list(path: &Path) -> std::io::Result<Vec<u8>> {
        use std::io::Read;
        let mut s = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut s)?;
        let mut out = Vec::new();
        for part in s.split(|c| c == '\n' || c == ',' || c == '\r') {
            let t = part.trim();
            if t.is_empty() { continue; }
            if let Ok(n) = t.parse::<u8>() { out.push(n); }
        }
        Ok(out)
    }

    /// Public helper: allows if in allow-list or active ttl grant.
    pub fn allows(&self, cap: u8) -> bool {
        if self.is_granted_by_ttl(cap) { return true; }
        self.allow_list.iter().any(|&c| c == cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_allows_and_records() {
        let p = Policy::new(vec![1, 2, 42]);
        assert!(p.allows(42));
        assert!(!p.allows(3));
        p.record("granted cap 42".to_string());
        p.record("denied cap 3".to_string());
        let log = p.audit_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0], "granted cap 42");
    }
}
