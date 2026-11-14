//! What the person chose, kept between runs (never browsing data).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum SearchEngine {
    DuckDuckGo,
    Brave,
    Startpage,
    Mojeek,
}

impl SearchEngine {
    pub const ALL: [SearchEngine; 4] = [
        SearchEngine::DuckDuckGo,
        SearchEngine::Brave,
        SearchEngine::Startpage,
        SearchEngine::Mojeek,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SearchEngine::DuckDuckGo => "DuckDuckGo",
            SearchEngine::Brave => "Brave Search",
            SearchEngine::Startpage => "Startpage",
            SearchEngine::Mojeek => "Mojeek",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            SearchEngine::DuckDuckGo => "No search history, no profiling",
            SearchEngine::Brave => "Independent index, no tracking",
            SearchEngine::Startpage => "Private results from a large index",
            SearchEngine::Mojeek => "Own crawler, no tracking",
        }
    }

    pub fn query_url(self, query: &str) -> String {
        let q = url_encode(query);
        match self {
            SearchEngine::DuckDuckGo => format!("https://duckduckgo.com/?q={q}"),
            SearchEngine::Brave => format!("https://search.brave.com/search?q={q}"),
            SearchEngine::Startpage => format!("https://www.startpage.com/do/search?q={q}"),
            SearchEngine::Mojeek => format!("https://www.mojeek.com/search?q={q}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Tracking {
    Basic,
    Balanced,
    Strict,
}

impl Tracking {
    pub const ALL: [Tracking; 3] = [Tracking::Basic, Tracking::Balanced, Tracking::Strict];
    pub fn name(self) -> &'static str {
        match self {
            Tracking::Basic => "Basic",
            Tracking::Balanced => "Balanced",
            Tracking::Strict => "Strict",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            Tracking::Basic => "Stops harmful trackers only",
            Tracking::Balanced => "Also stops trackers from sites you haven't visited",
            Tracking::Strict => "Stops most trackers and third-party cookies",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum PermMode {
    Block,
    Ask,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct Totals {
    pub trackers: u64,
    pub upgrades: u64,
    pub fingerprinting: u64,
    pub permissions: u64,
    pub popups: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    // Shields
    pub block_trackers: bool,
    pub tracking: Tracking,
    pub fingerprint_guard: bool,
    pub send_gpc: bool,
    pub trim_referrers: bool,
    pub block_webrtc_leak: bool,
    pub block_popups: bool,
    pub blend_in_user_agent: bool,
    // Connections
    pub https_only: bool,
    pub smartscreen: bool,
    // Data
    pub forget_on_close: bool,
    pub save_passwords: bool,
    pub autofill: bool,
    pub block_javascript: bool,
    // Search
    pub search: SearchEngine,
    // Permissions
    pub perm_camera: PermMode,
    pub perm_microphone: PermMode,
    pub perm_location: PermMode,
    pub perm_notifications: PermMode,
    pub perm_clipboard: PermMode,
    pub perm_other: PermMode,
    // Lists
    pub shields_off_sites: Vec<String>,
    pub bookmarks: Vec<Bookmark>,
    pub totals: Totals,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            block_trackers: true,
            tracking: Tracking::Strict,
            fingerprint_guard: true,
            send_gpc: true,
            trim_referrers: true,
            block_webrtc_leak: true,
            block_popups: true,
            blend_in_user_agent: true,
            https_only: true,
            smartscreen: false,
            forget_on_close: true,
            save_passwords: false,
            autofill: false,
            block_javascript: false,
            search: SearchEngine::DuckDuckGo,
            perm_camera: PermMode::Block,
            perm_microphone: PermMode::Block,
            perm_location: PermMode::Block,
            perm_notifications: PermMode::Block,
            perm_clipboard: PermMode::Block,
            perm_other: PermMode::Block,
            shields_off_sites: Vec::new(),
            bookmarks: default_bookmarks(),
            totals: Totals::default(),
        }
    }
}

fn default_bookmarks() -> Vec<Bookmark> {
    [
        ("DuckDuckGo", "https://duckduckgo.com/"),
        ("Wikipedia", "https://www.wikipedia.org/"),
        ("Cover Your Tracks", "https://coveryourtracks.eff.org/"),
        ("Privacy Guides", "https://www.privacyguides.org/"),
        ("BrowserLeaks", "https://browserleaks.com/"),
        ("Have I Been Pwned", "https://haveibeenpwned.com/"),
    ]
    .into_iter()
    .map(|(t, u)| Bookmark { title: t.into(), url: u.into() })
    .collect()
}

/// Where LabSphere keeps its choices (roaming app data).
pub fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LabSphere")
}

/// Where the page engine keeps its working files (local app data).
pub fn data_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("LabSphere")
}

impl Settings {
    fn path() -> PathBuf {
        config_dir().join("settings.json")
    }

    pub fn load() -> Settings {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let dir = config_dir();
        if std::fs::create_dir_all(&dir).is_ok() {
            if let Ok(text) = serde_json::to_string_pretty(self) {
                let tmp = dir.join("settings.json.tmp");
                if std::fs::write(&tmp, text).is_ok() {
                    let _ = std::fs::rename(&tmp, Self::path());
                }
            }
        }
    }

    pub fn is_bookmarked(&self, url: &str) -> bool {
        self.bookmarks.iter().any(|b| same_page(&b.url, url))
    }
}

fn same_page(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
