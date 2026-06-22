//! Network layer: typed models for the Digi-API and the async fetch helpers.
//!
//! Public endpoint: <https://digi-api.com/api/v1/digimon>
//!
//! Two shapes matter to us:
//! * The *list* endpoint returns a paginated `content` array of lightweight
//!   summaries plus a `pageable` block (used for search + filtering).
//! * The *detail* endpoint (`/digimon/{id}`) returns the full record including
//!   the branched `priorEvolutions` / `nextEvolutions` arrays.

use serde::{Deserialize, Serialize};

const BASE: &str = "https://digi-api.com/api/v1/digimon";

/// Result of an async network task, handed back to the app event loop.
///
/// We carry the `page`/`id` that the request was *for* so the reducer in
/// `App::on_net` can decide whether to replace or append, and whether a
/// returning detail/sprite still matches the current selection.
pub enum NetMessage {
    List {
        page: u32,
        result: Result<DigimonPage, String>,
    },
    Detail {
        id: u32,
        result: Result<Digimon, String>,
    },
    /// A decoded sprite image for Digimon `id`.
    Image {
        id: u32,
        result: Result<image::DynamicImage, String>,
    },
}

// ---------------------------------------------------------------------------
// List endpoint
// ---------------------------------------------------------------------------

/// One row in the paginated index. The list endpoint only exposes these four
/// fields; full data is loaded lazily via [`fetch_detail`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DigimonSummary {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub href: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Pageable {
    #[serde(rename = "totalPages", default)]
    pub total_pages: u32,
    #[serde(rename = "totalElements", default)]
    pub total_elements: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DigimonPage {
    #[serde(default)]
    pub content: Vec<DigimonSummary>,
    #[serde(default)]
    pub pageable: Pageable,
}

// ---------------------------------------------------------------------------
// Detail endpoint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Digimon {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub levels: Vec<LevelEntry>,
    #[serde(default)]
    pub types: Vec<TypeEntry>,
    #[serde(default)]
    pub attributes: Vec<AttributeEntry>,
    #[serde(default)]
    pub descriptions: Vec<Description>,
    #[serde(rename = "releaseDate", default)]
    pub release_date: String,
    #[serde(rename = "priorEvolutions", default)]
    pub prior_evolutions: Vec<Evolution>,
    #[serde(rename = "nextEvolutions", default)]
    pub next_evolutions: Vec<Evolution>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelEntry {
    #[serde(default)]
    pub level: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeEntry {
    #[serde(rename = "type", default)]
    pub type_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttributeEntry {
    #[serde(default)]
    pub attribute: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Description {
    #[serde(default)]
    pub origin: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub description: String,
}

/// A single edge in the evolution graph. Because a Digimon may digivolve into
/// (or from) several others, these arrive as arrays — the branching we render.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Evolution {
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub digimon: String,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub url: String,
}

impl Digimon {
    /// Pick the best human-readable description, preferring US English.
    pub fn english_description(&self) -> &str {
        self.descriptions
            .iter()
            .find(|d| d.language.eq_ignore_ascii_case("en_us"))
            .or_else(|| self.descriptions.iter().find(|d| d.language.starts_with("en")))
            .or_else(|| self.descriptions.first())
            .map(|d| d.description.as_str())
            .unwrap_or("No description on record for this Digimon.")
    }

    pub fn primary_level(&self) -> Option<&str> {
        self.levels.first().map(|l| l.level.as_str())
    }

    pub fn primary_attribute(&self) -> Option<&str> {
        self.attributes.first().map(|a| a.attribute.as_str())
    }

    pub fn type_list(&self) -> String {
        if self.types.is_empty() {
            "Unknown".into()
        } else {
            self.types
                .iter()
                .map(|t| t.type_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

/// Build a shared, connection-pooling HTTP client. Cheap to clone.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("digiductor/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build reqwest client")
}

/// Construct a list URL from the active query state. `level`/`attribute` use the
/// API's internal naming (see `LevelFilter::api_value`).
pub fn list_url(page: u32, name: &str, level: Option<&str>, attribute: Option<&str>) -> String {
    let mut url = format!("{BASE}?pageSize=50&page={page}");
    if !name.trim().is_empty() {
        url.push_str(&format!("&name={}", encode(name.trim())));
    }
    if let Some(level) = level {
        url.push_str(&format!("&level={}", encode(level)));
    }
    if let Some(attribute) = attribute {
        url.push_str(&format!("&attribute={}", encode(attribute)));
    }
    url
}

pub async fn fetch_list(client: &reqwest::Client, url: &str) -> anyhow::Result<DigimonPage> {
    let page = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<DigimonPage>()
        .await?;
    Ok(page)
}

pub async fn fetch_detail(client: &reqwest::Client, id: u32) -> anyhow::Result<Digimon> {
    let url = format!("{BASE}/{id}");
    let digimon = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Digimon>()
        .await?;
    Ok(digimon)
}

/// Download a sprite and decode it into an in-memory image. Decoding is CPU
/// work, so it runs on a blocking thread to keep the async runtime free.
pub async fn fetch_image(
    client: &reqwest::Client,
    url: &str,
) -> anyhow::Result<image::DynamicImage> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let image = tokio::task::spawn_blocking(move || image::load_from_memory(&bytes)).await??;
    Ok(image)
}

/// Minimal percent-encoding for query values (names, "Baby I", etc.).
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
