//! Two-tier cache for fetched Digimon detail records.
//!
//! * Primary tier is an in-memory `HashMap`, so a Digimon viewed once in a
//!   session renders instantly forever after — no network round-trip.
//! * Secondary tier is a JSON file in the OS cache dir, loaded at startup and
//!   rewritten on each insert. This survives restarts so a warm cache means the
//!   encyclopedia works largely offline.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::network::api::Digimon;

pub struct Cache {
    mem: HashMap<u32, Digimon>,
    path: PathBuf,
}

impl Cache {
    /// Load any previously persisted cache. A missing or corrupt file is not an
    /// error — we just start empty.
    pub fn load() -> Self {
        let path = cache_path();
        let mem = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Self { mem, path }
    }

    pub fn get(&self, id: u32) -> Option<Digimon> {
        self.mem.get(&id).cloned()
    }

    /// Store a record and persist the whole cache to disk (fire-and-forget; a
    /// failed write is non-fatal and silently ignored).
    pub fn insert(&mut self, digimon: Digimon) {
        self.mem.insert(digimon.id, digimon);
        self.persist();
    }

    pub fn len(&self) -> usize {
        self.mem.len()
    }

    fn persist(&self) {
        if let Ok(raw) = serde_json::to_string(&self.mem) {
            let _ = std::fs::write(&self.path, raw);
        }
    }
}

fn cache_path() -> PathBuf {
    let mut dir = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("digiductor");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("digimon_cache.json");
    dir
}
