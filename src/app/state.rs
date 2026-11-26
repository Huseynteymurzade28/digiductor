//! Application state and the reducer that drives it.
//!
//! `App` is the single source of truth: the UI reads it (never mutates from the
//! render path beyond the `ListState`), key presses mutate it, and async network
//! results are folded in via [`App::on_net`]. Network work is *spawned* — the
//! handlers here only kick off tasks and update loading flags; results arrive
//! later as `Event::Net`.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use image::DynamicImage;
use ratatui::widgets::ListState;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use tokio::sync::mpsc::UnboundedSender;

use crate::cache::Cache;
use crate::event::Event;
use crate::favorites::Favorites;
use crate::network::api::{self, Digimon, DigimonSummary, NetMessage};
use crate::sprite::{self, SpriteStyle};
use crate::theme;

/// Fallback upper bound for random picks before the full index size is known.
const DEFAULT_UNIVERSE: u32 = 1400;
/// How many jumps `Backspace` can rewind.
const MAX_HISTORY: usize = 64;

/// Whether keystrokes drive navigation or feed the search box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Browse,
    Search,
}

/// How many evolution branches we display (and allow navigation over) per side.
/// Kept in sync with the renderer in `ui::evolution`.
pub const MAX_EVOLUTIONS_SHOWN: usize = 7;

/// Level filter. Labels are the English terms; `api_value` is what the Digi-API
/// actually understands (it uses the Japanese-tradition naming internally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelFilter {
    All,
    Fresh,
    InTraining,
    Rookie,
    Champion,
    Ultimate,
    Mega,
}

impl LevelFilter {
    pub fn cycle(self) -> Self {
        use LevelFilter::*;
        match self {
            All => Fresh,
            Fresh => InTraining,
            InTraining => Rookie,
            Rookie => Champion,
            Champion => Ultimate,
            Ultimate => Mega,
            Mega => All,
        }
    }

    pub fn label(self) -> &'static str {
        use LevelFilter::*;
        match self {
            All => "ALL",
            Fresh => "Fresh",
            InTraining => "In-Training",
            Rookie => "Rookie",
            Champion => "Champion",
            Ultimate => "Ultimate",
            Mega => "Mega",
        }
    }

    pub fn api_value(self) -> Option<&'static str> {
        use LevelFilter::*;
        match self {
            All => None,
            Fresh => Some("Baby I"),
            InTraining => Some("Baby II"),
            Rookie => Some("Child"),
            Champion => Some("Adult"),
            Ultimate => Some("Perfect"),
            Mega => Some("Ultimate"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeFilter {
    All,
    Vaccine,
    Data,
    Virus,
}

impl AttributeFilter {
    pub fn cycle(self) -> Self {
        use AttributeFilter::*;
        match self {
            All => Vaccine,
            Vaccine => Data,
            Data => Virus,
            Virus => All,
        }
    }

    pub fn label(self) -> &'static str {
        use AttributeFilter::*;
        match self {
            All => "ALL",
            Vaccine => "Vaccine",
            Data => "Data",
            Virus => "Virus",
        }
    }

    pub fn api_value(self) -> Option<&'static str> {
        use AttributeFilter::*;
        match self {
            All => None,
            Vaccine => Some("Vaccine"),
            Data => Some("Data"),
            Virus => Some("Virus"),
        }
    }
}

pub struct App {
    pub running: bool,

    // Plumbing.
    tx: UnboundedSender<Event>,
    client: reqwest::Client,
    pub cache: Cache,
    pub favorites: Favorites,

    // Index / list state.
    pub list: Vec<DigimonSummary>,
    pub list_state: ListState,
    pub page: u32,
    pub total_pages: u32,
    pub total_elements: u32,
    pub has_more: bool,

    // Detail pane.
    pub detail: Option<Digimon>,
    pub detail_id: Option<u32>,
    pub detail_scroll: u16,
    /// Digimon visited before each jump (evolution / random / vault), newest
    /// last, so `Backspace` can walk back along the path.
    pub history: Vec<u32>,

    // Evolution matrix navigation. When `evo_focus` is set, the arrow keys drive
    // a cursor (`evo_selected`) over the branch entries so the user can jump
    // along the digivolution line.
    pub evo_focus: bool,
    pub evo_selected: usize,

    // Sprite rendering. `picker` knows the terminal's graphics protocol;
    // `image_state` is the resize-able protocol for the current sprite.
    picker: Picker,
    pub image_state: Option<StatefulProtocol>,
    pub image_id: Option<u32>,
    pub loading_image: bool,
    /// Raw decoded artwork; styling is re-applied on demand.
    image_cache: HashMap<u32, DynamicImage>,
    pub sprite_style: SpriteStyle,

    // Query state.
    pub mode: InputMode,
    pub search: String,
    pub input: String,
    pub level_filter: LevelFilter,
    pub attribute_filter: AttributeFilter,
    /// Browsing the local favourites vault instead of the API index.
    pub vault_view: bool,
    /// Total Digimon in the unfiltered index, used for random picks.
    universe: u32,

    // Overlays.
    pub show_help: bool,

    // Async / feedback.
    pub loading_list: bool,
    pub loading_detail: bool,
    pub status: String,
    pub error: Option<String>,
    pub tick_count: u64,
}

impl App {
    pub fn new(tx: UnboundedSender<Event>, mut picker: Picker) -> Self {
        // Transparent sprite pixels blend into the panel instead of black.
        if let ratatui::style::Color::Rgb(r, g, b) = theme::BG {
            picker.set_background_color([r, g, b, 255]);
        }
        let cache = Cache::load();
        let cached = cache.len();
        Self {
            running: true,
            tx,
            client: api::client(),
            cache,
            favorites: Favorites::load(),
            list: Vec::new(),
            list_state: ListState::default(),
            page: 0,
            total_pages: 0,
            total_elements: 0,
            has_more: false,
            detail: None,
            detail_id: None,
            detail_scroll: 0,
            history: Vec::new(),
            evo_focus: false,
            evo_selected: 0,
            picker,
            image_state: None,
            image_id: None,
            loading_image: false,
            image_cache: HashMap::new(),
            sprite_style: SpriteStyle::Pixel,
            mode: InputMode::Browse,
            search: String::new(),
            input: String::new(),
            level_filter: LevelFilter::All,
            attribute_filter: AttributeFilter::All,
            vault_view: false,
            universe: DEFAULT_UNIVERSE,
            show_help: false,
            loading_list: true,
            loading_detail: false,
            status: format!("BOOTING DIGIDUCTOR :: {cached} records in local cache"),
            error: None,
            tick_count: 0,
        }
    }

    /// Kick off the initial index load.
    pub fn bootstrap(&mut self) {
        self.spawn_list(false);
    }

    // -- animation -----------------------------------------------------------

    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    pub fn spinner_active(&self) -> bool {
        self.loading_list || self.loading_detail
    }

    // -- input ---------------------------------------------------------------

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.mode {
            InputMode::Search => self.on_key_search(key),
            InputMode::Browse => self.on_key_browse(key),
        }
    }

    fn on_key_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.search = self.input.clone();
                self.mode = InputMode::Browse;
                self.reload();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
            }
            KeyCode::Esc => {
                self.input = self.search.clone();
                self.mode = InputMode::Browse;
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
    }

    fn on_key_browse(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Quit is global regardless of which pane has focus.
        if key.code == KeyCode::Char('q') || (ctrl && key.code == KeyCode::Char('c')) {
            self.running = false;
            return;
        }

        // The help overlay swallows the next key press and closes.
        if self.show_help {
            self.show_help = false;
            return;
        }

        // When the evolution matrix has focus, route keys to its cursor.
        if self.evo_focus {
            self.on_key_evolution(key);
            return;
        }

        match key.code {
            KeyCode::Esc => self.running = false,

            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::Home => self.select_index(0),

            // Detail description scrolling.
            KeyCode::Char(']') => self.detail_scroll = self.detail_scroll.saturating_add(1),
            KeyCode::Char('[') => self.detail_scroll = self.detail_scroll.saturating_sub(1),

            // Hop into the evolution matrix to walk the digivolution line.
            KeyCode::Char('e') | KeyCode::Tab => self.toggle_evo_focus(),

            // Search + filters.
            KeyCode::Char('/') => {
                self.input = self.search.clone();
                self.mode = InputMode::Search;
            }
            KeyCode::Char('l') => {
                self.level_filter = self.level_filter.cycle();
                self.reload();
            }
            KeyCode::Char('a') => {
                self.attribute_filter = self.attribute_filter.cycle();
                self.reload();
            }
            KeyCode::Char('x') => self.clear_filters(),
            KeyCode::Char('r') => self.reload(),

            // Extras.
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('f') => self.toggle_favorite(),
            KeyCode::Char('v') => self.toggle_vault(),
            KeyCode::Char('s') => self.random_digimon(),
            KeyCode::Char('p') => self.cycle_sprite_style(),
            KeyCode::Backspace | KeyCode::Char('b') => self.go_back(),
            _ => {}
        }
    }

    /// Key handling while the evolution matrix is focused.
    fn on_key_evolution(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc
            | KeyCode::Char('e')
            | KeyCode::Tab
            | KeyCode::Left
            | KeyCode::Char('h') => self.evo_focus = false,
            KeyCode::Down | KeyCode::Char('j') => self.move_evo(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_evo(-1),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.enter_evo(),
            _ => {}
        }
    }

    // -- evolution navigation -----------------------------------------------

    /// The navigable (id-bearing) evolution targets for the current Digimon, in
    /// display order: prior branches first, then next. Ordering and the cap match
    /// `ui::evolution` so the cursor lines up with what's drawn.
    pub fn evo_nav_list(&self) -> Vec<(u32, String)> {
        let Some(d) = &self.detail else {
            return Vec::new();
        };
        d.prior_evolutions
            .iter()
            .take(MAX_EVOLUTIONS_SHOWN)
            .chain(d.next_evolutions.iter().take(MAX_EVOLUTIONS_SHOWN))
            .filter_map(|e| e.id.map(|id| (id, e.image.clone())))
            .collect()
    }

    /// The id of the currently-highlighted evolution, if the matrix is focused.
    pub fn evo_selected_id(&self) -> Option<u32> {
        if !self.evo_focus {
            return None;
        }
        self.evo_nav_list()
            .get(self.evo_selected)
            .map(|(id, _)| *id)
    }

    fn toggle_evo_focus(&mut self) {
        if self.evo_focus {
            self.evo_focus = false;
        } else if !self.evo_nav_list().is_empty() {
            self.evo_focus = true;
            self.evo_selected = 0;
        }
    }

    fn move_evo(&mut self, delta: i32) {
        let n = self.evo_nav_list().len() as i32;
        if n == 0 {
            return;
        }
        self.evo_selected = (self.evo_selected as i32 + delta).rem_euclid(n) as usize;
    }

    /// Jump to the highlighted evolution, loading its record and sprite.
    fn enter_evo(&mut self) {
        if let Some((id, image)) = self.evo_nav_list().get(self.evo_selected).cloned() {
            self.jump_to(id, Some(image));
            self.status = format!("Digivolving → #{id}…");
        }
    }

    /// Show Digimon `id` as a deliberate jump (not list scrolling), recording
    /// the current one in the back-history.
    fn jump_to(&mut self, id: u32, image: Option<String>) {
        if let Some(current) = self.detail_id {
            if current != id {
                self.history.push(current);
                if self.history.len() > MAX_HISTORY {
                    self.history.remove(0);
                }
            }
        }
        self.show(id, image);
    }

    /// Load the record + sprite for `id` and mirror the selection in the index
    /// if it's loaded there. When the sprite URL is unknown it is taken from the
    /// detail record once that arrives.
    fn show(&mut self, id: u32, image: Option<String>) {
        match self.list.iter().position(|s| s.id == id) {
            Some(pos) => self.list_state.select(Some(pos)),
            None => self.list_state.select(None),
        }
        let image = image
            .or_else(|| {
                self.list
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.image.clone())
            })
            .unwrap_or_default();
        self.load_detail(id);
        self.load_image(id, image);
    }

    fn go_back(&mut self) {
        match self.history.pop() {
            Some(id) => {
                self.show(id, None);
                self.status = format!("◀ back to #{id}");
            }
            None => self.status = "Nothing to go back to.".into(),
        }
    }

    // -- extras --------------------------------------------------------------

    fn toggle_favorite(&mut self) {
        let Some(d) = &self.detail else {
            return;
        };
        let name = d.name.clone();
        let now_fav = self.favorites.toggle(d.summary());
        self.status = if now_fav {
            format!("★ {name} added to the Digi-Vault")
        } else {
            format!("☆ {name} removed from the Digi-Vault")
        };
        if self.vault_view {
            self.refresh_vault();
        }
    }

    fn toggle_vault(&mut self) {
        self.vault_view = !self.vault_view;
        if self.vault_view {
            self.list_state.select(None);
            self.refresh_vault();
        } else {
            self.reload();
        }
    }

    /// Rebuild the list from local favourites (name search still applies).
    fn refresh_vault(&mut self) {
        let keep = self.list_state.selected();
        self.list = self.favorites.matching(&self.search);
        self.total_elements = self.list.len() as u32;
        self.has_more = false;
        self.loading_list = false;
        if self.list.is_empty() {
            self.list_state.select(None);
            self.status = "Digi-Vault is empty — press 'f' on a Digimon to save it.".into();
        } else {
            self.status = format!("Digi-Vault · {} saved", self.list.len());
            let index = keep.unwrap_or(0).min(self.list.len() - 1);
            // Only reload the detail when the selection actually changes.
            if self.detail_id != Some(self.list[index].id) {
                self.select_index(index);
            } else {
                self.list_state.select(Some(index));
            }
        }
    }

    fn random_digimon(&mut self) {
        let id = random_in(1, self.universe.max(1));
        self.jump_to(id, None);
        self.status = format!("🎲 Random encounter! #{id}");
    }

    fn cycle_sprite_style(&mut self) {
        self.sprite_style = self.sprite_style.cycle();
        self.status = format!("Sprite style → {}", self.sprite_style.label());
        self.refresh_sprite();
    }

    /// (Re)build the displayed sprite from the raw image cache using the current
    /// style and the shown Digimon's attribute colour.
    fn refresh_sprite(&mut self) {
        let Some(id) = self.image_id else {
            return;
        };
        let Some(raw) = self.image_cache.get(&id) else {
            return;
        };
        let tint = self
            .detail
            .as_ref()
            .filter(|d| d.id == id)
            .and_then(|d| d.primary_attribute())
            .map(theme::attribute_color)
            .unwrap_or(theme::CYAN);
        let tint = match tint {
            ratatui::style::Color::Rgb(r, g, b) => [r, g, b],
            _ => [0, 132, 255],
        };
        let styled = sprite::stylize(raw, self.sprite_style, tint);
        self.image_state = Some(self.picker.new_resize_protocol(styled));
    }

    /// If the sprite for `id` is still unknown (random / history jumps), take
    /// its URL from the freshly loaded detail record.
    fn ensure_image_from_detail(&mut self, id: u32) {
        if self.image_id != Some(id) || self.loading_image || self.image_cache.contains_key(&id) {
            return;
        }
        if let Some(url) = self
            .detail
            .as_ref()
            .filter(|d| d.id == id)
            .and_then(|d| d.image_url())
            .map(str::to_string)
        {
            self.load_image(id, url);
        }
    }

    fn clear_filters(&mut self) {
        self.vault_view = false;
        self.search.clear();
        self.input.clear();
        self.level_filter = LevelFilter::All;
        self.attribute_filter = AttributeFilter::All;
        self.reload();
    }

    // -- list navigation -----------------------------------------------------

    fn move_selection(&mut self, delta: i32) {
        if self.list.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, self.list.len() as i32 - 1) as usize;
        self.select_index(next);
    }

    fn select_index(&mut self, index: usize) {
        if self.list.is_empty() {
            return;
        }
        let index = index.min(self.list.len() - 1);
        self.list_state.select(Some(index));
        let id = self.list[index].id;
        let sprite_url = self.list[index].image.clone();
        self.load_detail(id);
        self.load_image(id, sprite_url);

        // Lazy pagination: when nearing the end of what's loaded, pull the next
        // page so scrolling stays seamless.
        if index + 5 >= self.list.len() && self.has_more && !self.loading_list {
            self.spawn_list(true);
        }
    }

    // -- network dispatch ----------------------------------------------------

    fn reload(&mut self) {
        if self.vault_view {
            self.refresh_vault();
        } else {
            self.spawn_list(false);
        }
    }

    /// Spawn a list fetch. `append == false` starts a fresh query at page 0 and
    /// replaces the list; `append == true` loads the next page and extends it.
    fn spawn_list(&mut self, append: bool) {
        let page = if append { self.page + 1 } else { 0 };
        self.loading_list = true;
        self.error = None;
        if append {
            self.status = format!("Streaming page {}…", page + 1);
        } else {
            self.status = "Querying Digi-API index…".into();
        }

        let url = api::list_url(
            page,
            &self.search,
            self.level_filter.api_value(),
            self.attribute_filter.api_value(),
        );
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = api::fetch_list(&client, &url)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(Event::Net(NetMessage::List { page, result }));
        });
    }

    /// Load a detail record: serve instantly from cache, otherwise fetch.
    fn load_detail(&mut self, id: u32) {
        self.detail_id = Some(id);
        self.detail_scroll = 0;
        self.evo_selected = 0;

        if let Some(cached) = self.cache.get(id) {
            self.detail = Some(cached);
            self.loading_detail = false;
            self.status = "Loaded from local cache ⚡".into();
            // The sprite is (re)built by the `load_image` call that follows.
            return;
        }

        self.detail = None;
        self.loading_detail = true;
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = api::fetch_detail(&client, id)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(Event::Net(NetMessage::Detail { id, result }));
        });
    }

    /// Load the sprite for `id`: instant from the in-memory image cache,
    /// otherwise download + decode off-thread.
    fn load_image(&mut self, id: u32, url: String) {
        self.image_id = Some(id);
        self.image_state = None;

        if self.image_cache.contains_key(&id) {
            self.loading_image = false;
            self.refresh_sprite();
            return;
        }

        if url.trim().is_empty() {
            self.loading_image = false;
            self.ensure_image_from_detail(id);
            return;
        }

        self.loading_image = true;
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = api::fetch_image(&client, &url)
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(Event::Net(NetMessage::Image { id, result }));
        });
    }

    // -- network results -----------------------------------------------------

    pub fn on_net(&mut self, msg: NetMessage) {
        match msg {
            NetMessage::List { page, result } => self.on_list_result(page, result),
            NetMessage::Detail { id, result } => self.on_detail_result(id, result),
            NetMessage::Image { id, result } => self.on_image_result(id, result),
        }
    }

    fn on_image_result(&mut self, id: u32, result: Result<DynamicImage, String>) {
        match result {
            Ok(image) => {
                self.image_cache.insert(id, image);
                // Only display if this sprite is still the selected one.
                if self.image_id == Some(id) {
                    self.loading_image = false;
                    self.refresh_sprite();
                }
            }
            Err(_) => {
                if self.image_id == Some(id) {
                    self.loading_image = false; // leave a placeholder, non-fatal
                }
            }
        }
    }

    fn on_list_result(&mut self, page: u32, result: Result<api::DigimonPage, String>) {
        // The user switched to the vault while this was in flight.
        if self.vault_view {
            return;
        }
        self.loading_list = false;
        match result {
            Ok(data) => {
                if self.search.is_empty()
                    && self.level_filter == LevelFilter::All
                    && self.attribute_filter == AttributeFilter::All
                    && data.pageable.total_elements > 0
                {
                    self.universe = data.pageable.total_elements;
                }
                self.page = page;
                self.total_pages = data.pageable.total_pages;
                self.total_elements = data.pageable.total_elements;
                self.has_more = (page + 1) < data.pageable.total_pages;

                if page == 0 {
                    self.list = data.content;
                    if self.list.is_empty() {
                        self.detail = None;
                        self.detail_id = None;
                        self.list_state.select(None);
                        self.status = "No Digimon match the current filters.".into();
                    } else {
                        self.status = format!("{} records matched", self.total_elements);
                        self.select_index(0);
                    }
                } else {
                    self.list.extend(data.content);
                    self.status = format!("{} of {} loaded", self.list.len(), self.total_elements);
                }
            }
            Err(err) => {
                self.error = Some(format!("Index uplink failed: {err}"));
                self.status = "NETWORK ERROR — press 'r' to retry".into();
            }
        }
    }

    fn on_detail_result(&mut self, id: u32, result: Result<Digimon, String>) {
        match result {
            Ok(digimon) => {
                self.cache.insert(digimon.clone());
                // Only display if this is still the selected Digimon (the user
                // may have scrolled past while it was in flight).
                if self.detail_id == Some(id) {
                    self.detail = Some(digimon);
                    self.loading_detail = false;
                    self.status = format!("{} records matched", self.total_elements);
                    // Re-tint the sprite now that the attribute is known.
                    self.refresh_sprite();
                    self.ensure_image_from_detail(id);
                }
            }
            Err(err) => {
                if self.detail_id == Some(id) {
                    self.loading_detail = false;
                    self.error = Some(format!("Failed to decode record #{id}: {err}"));
                }
            }
        }
    }

    // -- view helpers --------------------------------------------------------

    /// Short summary of active filters for the status / filter bar.
    pub fn filter_summary(&self) -> String {
        let mut parts = Vec::new();
        if self.vault_view {
            parts.push("★vault".to_string());
        }
        if !self.search.is_empty() {
            parts.push(format!("name~\"{}\"", self.search));
        }
        if self.level_filter != LevelFilter::All {
            parts.push(format!("lvl={}", self.level_filter.label()));
        }
        if self.attribute_filter != AttributeFilter::All {
            parts.push(format!("attr={}", self.attribute_filter.label()));
        }
        if parts.is_empty() {
            "none".into()
        } else {
            parts.join(" · ")
        }
    }
}

/// A small, dependency-free random integer in `lo..=hi`, seeded from the clock.
/// Plenty for "surprise me" picks.
fn random_in(lo: u32, hi: u32) -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut x = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    // xorshift64*
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
    lo + (r % (hi - lo + 1) as u64) as u32
}
