//! The Digi-Vault: a persisted list of favourite Digimon.
//!
//! Stored as JSON in the OS data dir (e.g. `~/.local/share/digiductor/`), so
//! favourites survive cache clears. We keep the lightweight summary (id, name,
//! sprite URL) so the vault can be browsed fully offline without the API.

use std::path::PathBuf;

use crate::network::api::DigimonSummary;

pub struct Favorites {
    items: Vec<DigimonSummary>,
    path: PathBuf,
}

impl Favorites {
    pub fn load() -> Self {
        let path = favorites_path();
        let items = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Self { items, path }
    }

    pub fn contains(&self, id: u32) -> bool {
        self.items.iter().any(|d| d.id == id)
    }

    /// Add or remove `entry`. Returns `true` if it is now a favourite.
    pub fn toggle(&mut self, entry: DigimonSummary) -> bool {
        let now_fav = if let Some(pos) = self.items.iter().position(|d| d.id == entry.id) {
            self.items.remove(pos);
            false
        } else {
            self.items.push(entry);
            self.items.sort_by_key(|d| d.id);
            true
        };
        self.persist();
        now_fav
    }

    /// Favourites whose name contains `query` (case-insensitive).
    pub fn matching(&self, query: &str) -> Vec<DigimonSummary> {
        let q = query.trim().to_lowercase();
        self.items
            .iter()
            .filter(|d| q.is_empty() || d.name.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    fn persist(&self) {
        if let Ok(raw) = serde_json::to_string_pretty(&self.items) {
            let _ = std::fs::write(&self.path, raw);
        }
    }
}

fn favorites_path() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("digiductor");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("favorites.json");
    dir
}
