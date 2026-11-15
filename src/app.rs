//! The LabSphere window: tabs, toolbar, Shields panel and the page area.

use crate::engine::{Engine, EngineEvent, Notifier, Page, PageOptions, Shortcut};
use crate::settings::{Bookmark, Settings};
use crate::shields::{self, Category, Privacy};
use crate::ui::{self, Icon};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Key, Layout, Modifiers, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Internal {
    Home,
    Settings,
    History,
}

impl Internal {
    pub fn url(self) -> &'static str {
        match self {
            Internal::Home => "labsphere://home",
            Internal::Settings => "labsphere://settings",
            Internal::History => "labsphere://history",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Internal::Home => "New tab",
            Internal::Settings => "Settings",
            Internal::History => "History",
        }
    }
}

pub enum InfoBar {
    HttpsFailed { http_url: String },
    Popup { url: String },
    Permission { what: &'static str, host: String },
    Note { text: String, tone: Color32 },
}

pub struct Tab {
    pub id: u64,
    pub private: bool,
    pub page: Option<Page>,
    pub pending_url: Option<String>,
    pub internal: Option<Internal>,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub address: String,
    pub blocked: Vec<(String, Category, u32)>,
    pub fingerprint: Vec<String>,
    pub video_ads: u32,
    pub denied: Vec<&'static str>,
    pub popups: Vec<String>,
    pub upgraded: bool,
    pub upgrade_host: Option<String>,
    pub info: Option<InfoBar>,
    pub zoom: f64,
    pub can_back: bool,
    pub can_forward: bool,
    pub status: String,
}

impl Tab {
    fn new(id: u64, private: bool, internal: Option<Internal>, pending_url: Option<String>) -> Tab {
        let url = pending_url.clone().unwrap_or_default();
        Tab {
            id,
            private,
            page: None,
            pending_url,
            internal,
            address: url.clone(),
            url,
            title: String::new(),
            loading: false,
            blocked: Vec::new(),
            fingerprint: Vec::new(),
            video_ads: 0,
            denied: Vec::new(),
            popups: Vec::new(),
            upgraded: false,
            upgrade_host: None,
            info: None,
            zoom: 1.0,
            can_back: false,
            can_forward: false,
            status: String::new(),
        }
    }

    pub fn has_web(&self) -> bool {
        self.page.is_some() || self.pending_url.is_some()
    }

    pub fn showing_web(&self) -> bool {
        self.internal.is_none() && self.has_web()
    }

    pub fn host(&self) -> Option<String> {
        if self.showing_web() { shields::host_of(&self.url) } else { None }
    }

    pub fn blocked_total(&self) -> u32 {
        self.blocked.iter().map(|b| b.2).sum()
    }

    pub fn display_title(&self) -> String {
        if let Some(i) = self.internal {
            return if self.private && i == Internal::Home { "Private tab".into() } else { i.title().into() };
        }
        if !self.title.trim().is_empty() {
            return self.title.trim().to_string();
        }
        self.host().unwrap_or_else(|| "Loading…".into())
    }

    pub fn shown_url(&self) -> String {
        match self.internal {
            Some(Internal::Home) => String::new(),
            Some(i) => i.url().to_string(),
            None => self.url.clone(),
        }
    }
}

#[derive(Default, Clone)]
pub struct SessionStats {
    pub trackers: u64,
    pub video_ads: u64,
    pub upgrades: u64,
    pub fingerprinting: u64,
    pub permissions: u64,
    pub popups: u64,
}

pub struct Visit {
    pub title: String,
    pub url: String,
    pub when: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drawer {
    Shields,
    Menu,
}

struct Shot {
    path: PathBuf,
    frames: u32,
    asked: bool,
}

pub struct SelfTest {
    pub out: PathBuf,
    pub url: String,
    pub started: Option<Instant>,
    pub injected: bool,
    pub log: Vec<String>,
    pub basic: bool,
}

pub struct LabSphere {
    pub settings: Settings,
    pub privacy: Arc<Privacy>,
    notifier: Notifier,
    rx: Receiver<EngineEvent>,
    engine: Option<Engine>,
    pub engine_error: Option<String>,
    pub engine_version: Option<String>,
    data_dir: PathBuf,
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_id: u64,
    pub drawer: Option<Drawer>,
    pub confirm_burn: bool,
    pub focus_address: bool,
    address_focused: bool,
    pub history: Vec<Visit>,
    pub session: SessionStats,
    pub fullscreen: bool,
    pub restart_needed: bool,
    dirty: bool,
    last_save: Instant,
    shot: Option<Shot>,
    pub selftest: Option<SelfTest>,
    last_title: String,
    pub home_query: String,
    burn_pending: bool,
    pub start_page_engine: bool,
    pub launched: Instant,
    persist: bool,
    focus_grace: u32,
}

impl LabSphere {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        screenshot: Option<PathBuf>,
        open_url: Option<String>,
        selftest: Option<SelfTest>,
        cleanup: Arc<Mutex<Option<PathBuf>>>,
    ) -> LabSphere {
        ui::setup_style(&cc.egui_ctx);
        let mut settings = Settings::load();
        if selftest.as_ref().map(|s| s.basic).unwrap_or(false) {
            settings.tracking = crate::settings::Tracking::Basic;
        }
        let privacy = Arc::new(Privacy::new(&settings));
        let (tx, rx) = channel();
        let notifier = Notifier { tx, ctx: cc.egui_ctx.clone() };
        let quiet = screenshot.is_some();

        let root = crate::settings::data_root();
        let data_dir = if settings.forget_on_close {
            root.join("Sessions").join(format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ))
        } else {
            root.join("Profile")
        };
        if !quiet {
            purge_old_sessions(&root.join("Sessions"));
            if settings.forget_on_close {
                *cleanup.lock().unwrap() = Some(data_dir.clone());
            }
        }

        let mut app = LabSphere {
            settings,
            privacy,
            notifier,
            rx,
            engine: None,
            engine_error: None,
            engine_version: crate::engine::engine_version(),
            data_dir,
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            drawer: None,
            confirm_burn: false,
            focus_address: false,
            address_focused: false,
            history: Vec::new(),
            session: SessionStats::default(),
            fullscreen: false,
            restart_needed: false,
            dirty: false,
            last_save: Instant::now(),
            shot: screenshot.map(|path| Shot { path, frames: 0, asked: false }),
            selftest,
            last_title: String::new(),
            home_query: String::new(),
            burn_pending: false,
            start_page_engine: !quiet,
            launched: Instant::now(),
            persist: false,
            focus_grace: 0,
        };
        app.persist = app.shot.is_none() && app.selftest.is_none();
        if app.engine_version.is_none() && cfg!(windows) && !quiet {
            app.engine_error = Some(
                "The Chromium page engine (Microsoft Edge WebView2 Runtime) isn't installed on this computer. \
                 Install it from Microsoft and restart LabSphere."
                    .into(),
            );
        }
        let start_drawer = match open_url.as_deref() {
            Some("labsphere://menu") => Some(Drawer::Menu),
            Some("labsphere://shields") => Some(Drawer::Shields),
            _ => None,
        };
        let first = if let Some(st) = &app.selftest {
            Some(Target::Web(st.url.clone()))
        } else if open_url.as_deref().map(|u| u.eq_ignore_ascii_case("labsphere://private")).unwrap_or(false) {
            app.new_tab_with(true, Some(Internal::Home), None);
            return app;
        } else {
            open_url.and_then(|u| resolve_input(&u, &app.settings))
        };
        match first {
            Some(Target::Web(url)) => app.new_tab_with(false, None, Some(url)),
            Some(Target::Internal(p)) => app.new_tab_with(false, Some(p), None),
            None => app.new_tab_with(false, Some(Internal::Home), None),
        }
        app.drawer = start_drawer;
        app
    }

    // ------------------------------------------------------------ tabs

    fn new_tab_with(&mut self, private: bool, internal: Option<Internal>, url: Option<String>) {
        let id = self.next_id;
        self.next_id += 1;
        let tab = Tab::new(id, private, internal, url);
        let at = if self.tabs.is_empty() { 0 } else { self.active + 1 };
        self.tabs.insert(at, tab);
        self.active = at;
        if self.drawer == Some(Drawer::Menu) {
            self.drawer = None;
        }
    }

    pub fn new_tab(&mut self, private: bool) {
        self.new_tab_with(private, Some(Internal::Home), None);
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(idx);
        drop(tab);
        if self.tabs.is_empty() {
            self.new_tab(false);
            return;
        }
        if self.active > idx || self.active >= self.tabs.len() {
            self.active = self.active.saturating_sub(1).min(self.tabs.len() - 1);
        }
        self.fullscreen = false;
    }

    fn tab_index(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    pub fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    pub fn open_internal(&mut self, page: Internal) {
        if page == Internal::Home {
            self.tab_mut().internal = Some(Internal::Home);
            return;
        }
        if let Some(i) = self.tabs.iter().position(|t| t.internal == Some(page) && !t.has_web()) {
            self.active = i;
        } else if self.tab().internal == Some(Internal::Home) && !self.tab().has_web() {
            self.tab_mut().internal = Some(page);
        } else {
            self.new_tab_with(false, Some(page), None);
        }
        self.drawer = None;
    }

    /// Go somewhere in the current tab from typed text.
    pub fn go(&mut self, input: &str) {
        match resolve_input(input, &self.settings) {
            Some(Target::Internal(p)) => self.open_internal(p),
            Some(Target::Web(url)) => self.load_in_active(url),
            None => {}
        }
    }

    pub fn load_in_active(&mut self, url: String) {
        let t = self.tab_mut();
        t.internal = None;
        t.info = None;
        t.address = url.clone();
        t.url = url.clone();
        if let Some(p) = &t.page {
            p.navigate(&url);
        } else {
            t.pending_url = Some(url);
        }
    }

    pub fn open_in_new_tab(&mut self, url: String, private: bool) {
        self.new_tab_with(private, None, Some(url));
    }

    fn shortcut(&mut self, s: Shortcut) {
        // Take the keyboard back from the page first.
        if let Some(p) = &self.tab().page {
            p.focus_window();
        }
        match s {
            Shortcut::NewTab => {
                self.new_tab(false);
                self.focus_address = true;
            }
            Shortcut::NewPrivateTab => {
                self.new_tab(true);
                self.focus_address = true;
            }
            Shortcut::CloseTab => self.close_tab(self.active),
            Shortcut::FocusAddress => {
                if let Some(p) = &self.tab().page {
                    p.focus_window();
                }
                self.focus_address = true;
            }
            Shortcut::NextTab => self.active = (self.active + 1) % self.tabs.len(),
            Shortcut::PrevTab => self.active = (self.active + self.tabs.len() - 1) % self.tabs.len(),
            Shortcut::Reload => self.reload(),
        }
    }

    pub fn back(&mut self) {
        let t = self.tab_mut();
        if t.internal.is_some() && t.has_web() {
            t.internal = None;
        } else if let Some(p) = &t.page {
            p.back();
        }
    }

    pub fn forward(&mut self) {
        if let Some(p) = &self.tab().page {
            p.forward();
        }
    }

    pub fn reload(&mut self) {
        let t = self.tab_mut();
        if t.internal.is_none() {
            if let Some(p) = &t.page {
                if t.loading {
                    p.stop();
                    t.loading = false;
                } else {
                    p.reload();
                }
            }
        }
    }

    pub fn set_zoom(&mut self, z: f64) {
        let t = self.tab_mut();
        t.zoom = z.clamp(0.3, 3.0);
        if let Some(p) = &t.page {
            p.zoom(t.zoom);
        }
    }

    pub fn toggle_bookmark(&mut self) {
        let t = self.tab();
        if !t.showing_web() || t.url.is_empty() {
            return;
        }
        let url = t.url.clone();
        let title = t.display_title();
        if self.settings.is_bookmarked(&url) {
            self.settings.bookmarks.retain(|b| b.url.trim_end_matches('/') != url.trim_end_matches('/'));
        } else {
            self.settings.bookmarks.push(Bookmark { title, url });
        }
        self.mark_dirty();
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn page_options(&self) -> PageOptions {
        PageOptions {
            fingerprint_guard: self.settings.fingerprint_guard,
            video_ads: self.settings.block_video_ads,
            javascript: !self.settings.block_javascript,
            save_passwords: self.settings.save_passwords,
            autofill: self.settings.autofill,
            smartscreen: self.settings.smartscreen,
            tracking_level: self.settings.tracking,
        }
    }

    /// The person changed a setting: push it everywhere it applies.
    pub fn settings_changed(&mut self) {
        self.privacy.apply(&self.settings);
        let opts = self.page_options();
        for t in &self.tabs {
            if let Some(p) = &t.page {
                p.apply_options(&opts);
            }
        }
        self.mark_dirty();
    }

    pub fn set_javascript_blocked(&mut self, blocked: bool) {
        self.settings.block_javascript = blocked;
        self.settings_changed();
        for t in &self.tabs {
            if let Some(p) = &t.page {
                p.set_javascript(!blocked);
                p.reload();
            }
        }
    }

    pub fn toggle_site_shields(&mut self) {
        let Some(host) = self.tab().host() else { return };
        let base = shields::base_domain(&host);
        if self.settings.shields_off_sites.contains(&base) {
            self.settings.shields_off_sites.retain(|s| s != &base);
        } else {
            self.settings.shields_off_sites.push(base.clone());
        }
        self.settings_changed();
        let video_site = matches!(base.as_str(), "youtube.com" | "youtube-nocookie.com");
        if video_site && self.settings.block_video_ads {
            // The video-ad remover is fitted when a page view is made, so make a
            // fresh one for this tab with the new choice.
            let t = self.tab_mut();
            if t.page.take().is_some() {
                t.pending_url = Some(t.url.clone());
            }
        } else if let Some(p) = &self.tab().page {
            p.reload();
        }
    }

    pub fn shields_up_here(&self) -> bool {
        match self.tab().host() {
            Some(h) => self.settings.block_trackers && self.privacy.shields_up_for(&h),
            None => self.settings.block_trackers,
        }
    }

    pub fn forget_site_cookies(&mut self) -> usize {
        let t = self.tab();
        match &t.page {
            Some(p) if t.showing_web() => p.clear_site_cookies(&t.url),
            _ => 0,
        }
    }

    /// Erase cookies, cache, site data, history and tabs.
    pub fn burn(&mut self) {
        self.burn_pending = true;
        self.confirm_burn = false;
        self.drawer = None;
    }

    fn finish_burn(&mut self, frame: &eframe::Frame) {
        self.burn_pending = false;
        let mut cleared = false;
        for t in &self.tabs {
            if let Some(p) = &t.page {
                p.clear_profile_data();
                cleared = true;
            }
        }
        if !cleared && self.engine.is_some() {
            // No page open: use a short-lived one to reach the stored profile.
            let opts = self.page_options();
            if let Some(engine) = &mut self.engine {
                if let Ok(p) = engine.create(frame, 0, false, "about:blank", [0, 0, 1, 1], &opts) {
                    p.clear_profile_data();
                }
            }
        }
        self.tabs.clear();
        self.history.clear();
        self.session = SessionStats::default();
        self.new_tab(false);
        self.tab_mut().info = Some(InfoBar::Note {
            text: "Erased: cookies, site data, cache, history and open tabs. You're starting fresh.".into(),
            tone: ui::GOOD,
        });
    }

    // ------------------------------------------------------------ engine events

    fn process_events(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                EngineEvent::NavStarting { tab, url } => {
                    let editing = self.editing(tab);
                    let mut visit = None;
                    if let Some(t) = self.find(tab) {
                        let host = shields::host_of(&url);
                        t.upgraded = t.upgrade_host.is_some() && t.upgrade_host == host;
                        if !t.upgraded {
                            t.upgrade_host = None;
                        }
                        t.url = url.clone();
                        t.loading = true;
                        t.blocked.clear();
                        t.fingerprint.clear();
                        t.video_ads = 0;
                        t.denied.clear();
                        t.popups.clear();
                        t.info = None;
                        if !editing {
                            t.address = url.clone();
                        }
                        if !t.private && !url.starts_with("about:") {
                            visit = Some(url);
                        }
                    }
                    if let Some(url) = visit {
                        if self.history.last().map(|v| v.url != url).unwrap_or(true) {
                            self.history.push(Visit { title: String::new(), url, when: Instant::now() });
                            if self.history.len() > 500 {
                                self.history.remove(0);
                            }
                        }
                    }
                }
                EngineEvent::Upgraded { tab, https_url } => {
                    let mut first_time = false;
                    if let Some(t) = self.find(tab) {
                        let host = shields::host_of(&https_url);
                        first_time = t.upgrade_host != host;
                        t.upgrade_host = host;
                        if let Some(p) = &t.page {
                            p.navigate(&https_url);
                        }
                    }
                    if first_time {
                        self.session.upgrades += 1;
                        self.settings.totals.upgrades += 1;
                        self.dirty = true;
                    }
                }
                EngineEvent::UpgradeFailed { tab, http_url } => {
                    if let Some(t) = self.find(tab) {
                        t.loading = false;
                        t.upgraded = false;
                        t.info = Some(InfoBar::HttpsFailed { http_url });
                    }
                }
                EngineEvent::Title { tab, title } => {
                    let mut entry = None;
                    if let Some(t) = self.find(tab) {
                        t.title = title.clone();
                        if !t.private {
                            entry = Some(t.url.clone());
                        }
                    }
                    if let Some(url) = entry {
                        if let Some(v) = self.history.iter_mut().rev().find(|v| v.url == url) {
                            v.title = title;
                        }
                    }
                }
                EngineEvent::Loading { tab, started, url } => {
                    let editing = self.editing(tab);
                    if let Some(t) = self.find(tab) {
                        t.loading = started;
                        if !url.is_empty() && url != "about:blank" {
                            t.url = url.clone();
                            if !editing {
                                t.address = url;
                            }
                        }
                        if let Some(p) = &t.page {
                            t.can_back = p.can_back();
                            t.can_forward = p.can_forward();
                        }
                    }
                }
                EngineEvent::UrlChanged { tab, url } => {
                    let editing = self.editing(tab);
                    if let Some(t) = self.find(tab) {
                        if !url.is_empty() && url != "about:blank" {
                            t.url = url.clone();
                            if !editing {
                                t.address = url;
                            }
                        }
                        if let Some(p) = &t.page {
                            t.can_back = p.can_back();
                            t.can_forward = p.can_forward();
                        }
                    }
                }
                EngineEvent::Blocked { tab, host, category } => {
                    if let Some(t) = self.find(tab) {
                        match t.blocked.iter_mut().find(|b| b.0 == host) {
                            Some(b) => b.2 += 1,
                            None => t.blocked.push((host, category, 1)),
                        }
                    }
                    self.session.trackers += 1;
                    self.settings.totals.trackers += 1;
                    self.dirty = true;
                }
                EngineEvent::VideoAd { tab, kind } => {
                    if let Some(t) = self.find(tab) {
                        t.video_ads += 1;
                    }
                    if kind != "overlay" {
                        self.session.video_ads += 1;
                        self.settings.totals.video_ads += 1;
                        self.dirty = true;
                    }
                }
                EngineEvent::Fingerprint { tab, kind } => {
                    let mut counted = false;
                    if let Some(t) = self.find(tab) {
                        if !t.fingerprint.contains(&kind) {
                            t.fingerprint.push(kind);
                            counted = true;
                        }
                    }
                    if counted {
                        self.session.fingerprinting += 1;
                        self.settings.totals.fingerprinting += 1;
                        self.dirty = true;
                    }
                }
                EngineEvent::PermissionDenied { tab, what } => {
                    if let Some(t) = self.find(tab) {
                        if !t.denied.contains(&what) {
                            t.denied.push(what);
                        }
                        let host = shields::host_of(&t.url).unwrap_or_default();
                        t.info = Some(InfoBar::Permission { what, host });
                    }
                    self.session.permissions += 1;
                    self.settings.totals.permissions += 1;
                    self.dirty = true;
                }
                EngineEvent::PopupBlocked { tab, url } => {
                    if let Some(t) = self.find(tab) {
                        t.popups.push(url.clone());
                        t.info = Some(InfoBar::Popup { url });
                    }
                    self.session.popups += 1;
                    self.settings.totals.popups += 1;
                    self.dirty = true;
                }
                EngineEvent::OpenTab { from, url } => {
                    let private = self.tab_index(from).map(|i| self.tabs[i].private).unwrap_or(false);
                    if let Some(i) = self.tab_index(from) {
                        self.active = i;
                    }
                    self.open_in_new_tab(url, private);
                }
                EngineEvent::Shortcut(s) => self.shortcut(s),
                EngineEvent::StatusText { tab, text } => {
                    if let Some(t) = self.find(tab) {
                        t.status = text;
                    }
                }
                EngineEvent::FullScreen { tab, on } => {
                    if self.tab_index(tab) == Some(self.active) {
                        self.fullscreen = on;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(on));
                    }
                }
                EngineEvent::Closed { tab } => {
                    if let Some(i) = self.tab_index(tab) {
                        self.close_tab(i);
                    }
                }
            }
        }
    }

    fn find(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    fn editing(&self, id: u64) -> bool {
        self.address_focused && self.tabs.get(self.active).map(|t| t.id == id).unwrap_or(false)
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        let ctrl = Modifiers::COMMAND;
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        let mut actions: Vec<Shortcut> = Vec::new();
        let mut other: Vec<&str> = Vec::new();
        ctx.input_mut(|i| {
            if i.consume_key(cs, Key::N) {
                actions.push(Shortcut::NewPrivateTab);
            }
            if i.consume_key(ctrl, Key::T) {
                actions.push(Shortcut::NewTab);
            }
            if i.consume_key(ctrl, Key::W) {
                actions.push(Shortcut::CloseTab);
            }
            if i.consume_key(ctrl, Key::L) {
                actions.push(Shortcut::FocusAddress);
            }
            if i.consume_key(cs, Key::Tab) {
                actions.push(Shortcut::PrevTab);
            }
            if i.consume_key(ctrl, Key::Tab) {
                actions.push(Shortcut::NextTab);
            }
            if i.consume_key(ctrl, Key::R) || i.consume_key(Modifiers::NONE, Key::F5) {
                actions.push(Shortcut::Reload);
            }
            if i.consume_key(ctrl, Key::D) {
                other.push("bookmark");
            }
            if i.consume_key(ctrl, Key::H) {
                other.push("history");
            }
            if i.consume_key(Modifiers::ALT, Key::ArrowLeft) {
                other.push("back");
            }
            if i.consume_key(Modifiers::ALT, Key::ArrowRight) {
                other.push("forward");
            }
            if i.consume_key(ctrl, Key::Plus) || i.consume_key(ctrl, Key::Equals) {
                other.push("zoom+");
            }
            if i.consume_key(ctrl, Key::Minus) {
                other.push("zoom-");
            }
            if i.consume_key(ctrl, Key::Num0) {
                other.push("zoom0");
            }
        });
        for a in actions {
            self.shortcut(a);
        }
        for o in other {
            match o {
                "bookmark" => self.toggle_bookmark(),
                "history" => self.open_internal(Internal::History),
                "back" => self.back(),
                "forward" => self.forward(),
                "zoom+" => self.set_zoom(self.tab().zoom + 0.1),
                "zoom-" => self.set_zoom(self.tab().zoom - 0.1),
                _ => self.set_zoom(1.0),
            }
        }
    }

    // ------------------------------------------------------------ page views

    fn sync_pages(&mut self, ctx: &egui::Context, frame: &eframe::Frame, content: Rect) {
        if self.burn_pending {
            self.finish_burn(frame);
        }
        let ppp = ctx.pixels_per_point();
        let bounds = [
            (content.min.x * ppp).round() as i32,
            (content.min.y * ppp).round() as i32,
            (content.width() * ppp).round().max(1.0) as i32,
            (content.height() * ppp).round().max(1.0) as i32,
        ];
        let usable = content.width() > 20.0 && content.height() > 20.0;
        if self.start_page_engine && self.engine_error.is_none() {
            let needs = self.tabs.iter().any(|t| t.page.is_none() && t.pending_url.is_some());
            if needs && self.engine.is_none() {
                let mut e = Engine::new(
                    self.data_dir.clone(),
                    self.privacy.clone(),
                    self.notifier.clone(),
                    self.settings.block_webrtc_leak,
                );
                if let Some(err) = e.failed.take() {
                    self.engine_error = Some(err);
                } else {
                    self.engine = Some(e);
                }
            }
            let opts = self.page_options();
            if let Some(engine) = &mut self.engine {
                for (i, t) in self.tabs.iter_mut().enumerate() {
                    if t.page.is_none() {
                        if let Some(url) = t.pending_url.clone() {
                            let visible = i == self.active && t.internal.is_none() && usable;
                            let b = if visible { bounds } else { [0, 0, 1, 1] };
                            match engine.create(frame, t.id, t.private, &url, b, &opts) {
                                Ok(mut p) => {
                                    p.place(b, visible);
                                    if (t.zoom - 1.0).abs() > 0.01 {
                                        p.zoom(t.zoom);
                                    }
                                    t.page = Some(p);
                                    t.pending_url = None;
                                    t.loading = true;
                                }
                                Err(e) => {
                                    self.engine_error = Some(format!("The page engine couldn't open this page: {e}"));
                                    t.pending_url = None;
                                }
                            }
                        }
                    }
                }
            }
        }
        for (i, t) in self.tabs.iter_mut().enumerate() {
            let visible = i == self.active && t.internal.is_none() && usable;
            if let Some(p) = &mut t.page {
                p.place(bounds, visible);
            }
        }
    }

    // ------------------------------------------------------------ drawing

    fn tab_strip(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let new_w = 34.0;
        let pad = 8.0;
        let n = self.tabs.len().max(1) as f32;
        let avail = full.width() - new_w - pad * 3.0;
        let tab_w = (avail / n).clamp(72.0, 230.0);
        let top = full.top() + 6.0;
        let mut x = full.left() + pad;
        let mut activate = None;
        let mut close = None;
        let time = ui.input(|i| i.time);
        let mut any_loading = false;
        let painter = ui.painter_at(full);
        for (i, t) in self.tabs.iter().enumerate() {
            let rect = Rect::from_min_size(pos2(x, top), vec2(tab_w, full.bottom() - top));
            let resp = ui.interact(rect, ui.id().with(("tab", t.id)), Sense::click());
            let active = i == self.active;
            let cr = CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 };
            if active {
                painter.rect_filled(rect, cr, ui::BAR);
                if t.private {
                    painter.rect_filled(
                        Rect::from_min_size(rect.min + vec2(10.0, 0.0), vec2(rect.width() - 20.0, 2.0)),
                        CornerRadius::same(1),
                        ui::PRIVATE,
                    );
                }
            } else if resp.hovered() {
                painter.rect_filled(rect.shrink2(vec2(2.0, 3.0)), CornerRadius::same(8), Color32::from_rgb(24, 28, 44));
            } else if i + 1 != self.active && i + 1 < self.tabs.len() {
                painter.line_segment(
                    [pos2(rect.right(), rect.top() + 9.0), pos2(rect.right(), rect.bottom() - 9.0)],
                    Stroke::new(1.0_f32, Color32::from_rgb(38, 44, 66)),
                );
            }
            // icon
            let ic = Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), Vec2::splat(18.0));
            if t.loading && t.showing_web() {
                any_loading = true;
                let a0 = (time * 6.0) as f32;
                let pts: Vec<_> = (0..=18)
                    .map(|k| {
                        let a = a0 + k as f32 / 18.0 * 4.4;
                        ic.center() + vec2(a.cos(), a.sin()) * 7.0
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(2.0_f32, ui::TEAL)));
            } else {
                match t.internal {
                    Some(Internal::Home) if t.private => ui::draw_icon(&painter, ic, Icon::Private, ui::PRIVATE),
                    Some(Internal::Home) => ui::draw_logo(&painter, ic.center(), 6.0),
                    Some(Internal::Settings) => ui::draw_icon(&painter, ic, Icon::Gear, ui::MUTED),
                    Some(Internal::History) => ui::draw_icon(&painter, ic, Icon::Clock, ui::MUTED),
                    None => {
                        let host = t.host().unwrap_or_default();
                        if t.private {
                            ui::draw_icon(&painter, ic, Icon::Private, ui::PRIVATE);
                        } else {
                            ui::monogram(&painter, ic.center(), 8.5, host.trim_start_matches("www."), &host);
                        }
                    }
                }
            }
            // title
            let show_close = active || resp.hovered();
            let title_w = rect.width() - 36.0 - if show_close { 28.0 } else { 8.0 };
            if title_w > 10.0 {
                let mut job = egui::text::LayoutJob::single_section(
                    t.display_title(),
                    egui::TextFormat {
                        font_id: FontId::proportional(13.0),
                        color: if active { ui::TEXT } else { ui::MUTED },
                        ..Default::default()
                    },
                );
                job.wrap = egui::text::TextWrapping {
                    max_width: title_w,
                    max_rows: 1,
                    break_anywhere: true,
                    overflow_character: Some('…'),
                };
                let galley = painter.layout_job(job);
                let pos = pos2(rect.left() + 34.0, rect.center().y - galley.size().y / 2.0);
                painter.galley(pos, galley, ui::TEXT);
            }
            if show_close {
                let cr = Rect::from_center_size(pos2(rect.right() - 16.0, rect.center().y), Vec2::splat(20.0));
                let cresp = ui.interact(cr, ui.id().with(("close", t.id)), Sense::click());
                if cresp.hovered() {
                    painter.rect_filled(cr, CornerRadius::same(5), Color32::from_rgb(52, 60, 90));
                }
                ui::draw_icon(&painter, cr.shrink(5.0), Icon::Close, if cresp.hovered() { ui::TEXT } else { ui::MUTED });
                if cresp.clicked() {
                    close = Some(i);
                }
            }
            if resp.clicked() && close.is_none() {
                activate = Some(i);
            }
            if resp.middle_clicked() {
                close = Some(i);
            }
            x += tab_w;
        }
        // new tab button
        let nb = Rect::from_center_size(
            pos2(x + pad + new_w / 2.0, top + (full.bottom() - top) / 2.0 - 1.0),
            Vec2::splat(new_w - 4.0),
        );
        let nresp = ui.interact(nb, ui.id().with("newtab"), Sense::click());
        if nresp.hovered() {
            painter.rect_filled(nb, CornerRadius::same(8), Color32::from_rgb(30, 35, 55));
        }
        ui::draw_icon(&painter, nb.shrink(8.0), Icon::Plus, if nresp.hovered() { ui::TEXT } else { ui::MUTED });
        let nresp = nresp.on_hover_text("New tab (Ctrl+T)");
        if nresp.clicked() {
            self.new_tab(false);
            self.focus_address = true;
        }
        if let Some(i) = close {
            self.close_tab(i);
        } else if let Some(i) = activate {
            self.active = i;
        }
        if any_loading {
            ui.ctx().request_repaint_after(Duration::from_millis(33));
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let h = 34.0;
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(6.0);
            let (can_back, can_fwd, web, loading) = {
                let t = self.tab();
                (t.can_back || (t.internal.is_some() && t.has_web()), t.can_forward, t.showing_web(), t.loading)
            };
            if ui::icon_button(ui, Icon::Back, h, can_back, false, "Back (Alt+Left)").clicked() {
                self.back();
            }
            if ui::icon_button(ui, Icon::Forward, h, can_fwd, false, "Forward (Alt+Right)").clicked() {
                self.forward();
            }
            let (ic, tip) = if loading && web { (Icon::Stop, "Stop loading") } else { (Icon::Reload, "Reload (Ctrl+R)") };
            if ui::icon_button(ui, ic, h, web, false, tip).clicked() {
                self.reload();
            }
            if ui::icon_button(ui, Icon::Home, h, true, false, "Home").clicked() {
                self.open_internal(Internal::Home);
            }
            ui.add_space(6.0);

            let right_w = 3.0 * (h + 4.0) + 58.0 + 24.0;
            let addr_w = (ui.available_width() - right_w).max(160.0);
            self.address_bar(ui, addr_w, h);
            ui.add_space(6.0);

            // Shields button with the count for this page
            let up = self.shields_up_here();
            let count = if self.tab().showing_web() { self.tab().blocked_total() } else { 0 };
            let (rect, resp) = ui.allocate_exact_size(vec2(58.0, h), Sense::click());
            let open = self.drawer == Some(Drawer::Shields);
            let fill = if open {
                Color32::from_rgb(44, 52, 82)
            } else if resp.hovered() {
                Color32::from_rgb(36, 43, 67)
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
            let ic = Rect::from_center_size(pos2(rect.left() + 18.0, rect.center().y), Vec2::splat(22.0));
            ui::draw_icon(
                ui.painter(),
                ic,
                if up { Icon::Shield } else { Icon::ShieldOff },
                if up { ui::TEAL } else { ui::MUTED },
            );
            let label = if count > 999 { "999+".to_string() } else { count.to_string() };
            ui.painter().text(
                pos2(rect.left() + 32.0, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(13.0),
                if count > 0 { ui::TEXT } else { ui::MUTED },
            );
            if resp.on_hover_text("LabSphere Shields: what was blocked on this page").clicked() {
                self.drawer = if open { None } else { Some(Drawer::Shields) };
            }

            if ui::icon_button(ui, Icon::Private, h, true, false, "New private tab (Ctrl+Shift+N)").clicked() {
                self.new_tab(true);
                self.focus_address = true;
            }
            if ui::icon_button(ui, Icon::Burn, h, true, self.confirm_burn, "Forget everything now").clicked() {
                self.confirm_burn = true;
                self.drawer = Some(Drawer::Menu);
            }
            let menu_open = self.drawer == Some(Drawer::Menu);
            if ui::icon_button(ui, Icon::Menu, h, true, menu_open, "Menu").clicked() {
                self.drawer = if menu_open { None } else { Some(Drawer::Menu) };
                if menu_open {
                    self.confirm_burn = false;
                }
            }
        });
    }

    fn address_bar(&mut self, ui: &mut Ui, width: f32, h: f32) {
        let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
        let focused = self.address_focused;
        let private = self.tab().private;
        ui.painter().rect_filled(rect, CornerRadius::same(17), ui::FIELD);
        let border = if focused {
            if private { ui::PRIVATE } else { ui::TEAL }
        } else {
            ui::LINE
        };
        ui.painter().rect_stroke(rect, CornerRadius::same(17), Stroke::new(1.0_f32, border), StrokeKind::Inside);

        // security indicator
        let t = self.tab();
        let web = t.showing_web();
        let scheme = shields::scheme_of(&t.url).to_ascii_lowercase();
        let (icon, color, tip) = if !web {
            (Icon::Search, ui::MUTED, "LabSphere page")
        } else if scheme == "https" {
            (Icon::Lock, ui::GOOD, "Secure connection (HTTPS)")
        } else if scheme == "http" {
            (Icon::Unlock, ui::WARN, "Not secure: this connection isn't encrypted")
        } else {
            (Icon::Globe, ui::MUTED, "Local or special page")
        };
        let sec = Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), Vec2::splat(26.0));
        let sresp = ui.interact(sec, ui.id().with("security"), Sense::click());
        if sresp.hovered() && web {
            ui.painter().rect_filled(sec, CornerRadius::same(13), Color32::from_rgb(34, 40, 62));
        }
        ui::draw_icon(ui.painter(), sec.shrink(6.0), icon, color);
        if sresp.on_hover_text(tip).clicked() && web {
            self.drawer = Some(Drawer::Shields);
        }

        // bookmark star
        let can_star = web && !self.tab().url.is_empty();
        let starred = can_star && self.settings.is_bookmarked(&self.tab().url);
        let star = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), Vec2::splat(26.0));
        if can_star {
            let r = ui.interact(star, ui.id().with("star"), Sense::click());
            if r.hovered() {
                ui.painter().rect_filled(star, CornerRadius::same(13), Color32::from_rgb(34, 40, 62));
            }
            ui::draw_icon(
                ui.painter(),
                star.shrink(5.0),
                if starred { Icon::StarFilled } else { Icon::Star },
                if starred { ui::WARN } else { ui::MUTED },
            );
            if r.on_hover_text(if starred { "Remove from saved sites" } else { "Save this site (Ctrl+D)" }).clicked() {
                self.toggle_bookmark();
            }
        }

        // the text field
        let text_rect = Rect::from_min_max(
            pos2(sec.right() + 4.0, rect.top() + 4.0),
            pos2(star.left() - 4.0, rect.bottom() - 4.0),
        );
        let hint = format!("Search privately with {} or type an address", self.settings.search.name());
        let idx = self.active;
        if !self.address_focused {
            let shown = self.tabs[idx].shown_url();
            self.tabs[idx].address = shown;
        }
        let mut text = std::mem::take(&mut self.tabs[idx].address);
        let id = ui.id().with("address");
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_rect).layout(Layout::left_to_right(Align::Center)));
        let mut out = egui::TextEdit::singleline(&mut text)
            .id(id)
            .frame(false)
            .hint_text(RichText::new(hint).color(ui::FAINT))
            .font(FontId::proportional(14.5))
            .text_color(ui::TEXT)
            .vertical_align(Align::Center)
            .desired_width(text_rect.width())
            .show(&mut child);
        let resp = out.response.clone();
        if self.focus_address {
            self.focus_address = false;
            self.focus_grace = 45;
            resp.request_focus();
            select_all(&mut out, ui.ctx(), &text);
        } else if resp.gained_focus() {
            select_all(&mut out, ui.ctx(), &text);
        }
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let escape = resp.has_focus() && ui.input(|i| i.key_pressed(Key::Escape));
        self.focus_grace = self.focus_grace.saturating_sub(1);
        if resp.has_focus() && !ui.input(|i| i.focused) && self.focus_grace == 0 {
            // The page took the keyboard: let the address follow the page again.
            resp.surrender_focus();
        }
        self.address_focused = resp.has_focus();
        self.tabs[idx].address = text.clone();
        if escape {
            self.tabs[idx].address = self.tabs[idx].shown_url();
            resp.surrender_focus();
        }
        if enter && !text.trim().is_empty() {
            self.address_focused = false;
            self.go(&text);
        }
    }

    fn info_bar(&mut self, ui: &mut Ui) {
        let idx = self.active;
        let Some(info) = &self.tabs[idx].info else { return };
        enum Act {
            None,
            Dismiss,
            ContinueHttp(String),
            GoBack,
            OpenPopup(String),
            Settings,
        }
        let mut act = Act::None;
        ui.horizontal_centered(|ui| {
            ui.add_space(12.0);
            let (icon, tone, text) = match info {
                InfoBar::HttpsFailed { http_url } => (
                    Icon::Unlock,
                    ui::WARN,
                    format!(
                        "HTTPS-Only: {} doesn't offer a secure connection. Others on your network could see or change this page.",
                        shields::host_of(http_url).unwrap_or_default()
                    ),
                ),
                InfoBar::Popup { url } => (
                    Icon::Shield,
                    ui::TEAL,
                    format!("Blocked a pop-up to {}", shields::host_of(url).unwrap_or_else(|| "another page".into())),
                ),
                InfoBar::Permission { what, host } => {
                    (Icon::Shield, ui::TEAL, format!("Blocked {host} from using your {what}"))
                }
                InfoBar::Note { text, tone } => (Icon::Check, *tone, text.clone()),
            };
            let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
            ui::draw_icon(ui.painter(), r, icon, tone);
            ui.label(RichText::new(text).color(ui::TEXT).size(13.5));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                if ui::icon_button(ui, Icon::Close, 26.0, true, false, "Dismiss").clicked() {
                    act = Act::Dismiss;
                }
                match info {
                    InfoBar::HttpsFailed { http_url } => {
                        if ui::ghost_button(ui, "Continue without security").clicked() {
                            act = Act::ContinueHttp(http_url.clone());
                        }
                        if ui::pill_button(ui, "Go back", ui::TEAL, ui::BG).clicked() {
                            act = Act::GoBack;
                        }
                    }
                    InfoBar::Popup { url } => {
                        if ui::ghost_button(ui, "Open it in a new tab").clicked() {
                            act = Act::OpenPopup(url.clone());
                        }
                    }
                    InfoBar::Permission { .. } => {
                        if ui::ghost_button(ui, "Permission settings").clicked() {
                            act = Act::Settings;
                        }
                    }
                    InfoBar::Note { .. } => {}
                }
            });
        });
        match act {
            Act::None => {}
            Act::Dismiss => self.tabs[idx].info = None,
            Act::ContinueHttp(url) => {
                if let Some(h) = shields::host_of(&url) {
                    self.privacy.add_https_exception(&h);
                }
                self.tabs[idx].info = None;
                self.load_in_active(url);
            }
            Act::GoBack => {
                self.tabs[idx].info = None;
                let can = self.tabs[idx].page.as_ref().map(|p| p.can_back()).unwrap_or(false);
                if can {
                    if let Some(p) = &self.tabs[idx].page {
                        p.back();
                    }
                } else {
                    self.tabs[idx].internal = Some(Internal::Home);
                }
            }
            Act::OpenPopup(url) => {
                let private = self.tabs[idx].private;
                self.tabs[idx].info = None;
                self.open_in_new_tab(url, private);
            }
            Act::Settings => {
                self.tabs[idx].info = None;
                self.open_internal(Internal::Settings);
            }
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let t = self.tab();
        ui.horizontal_centered(|ui| {
            ui.add_space(12.0);
            let text = if !t.status.is_empty() && t.showing_web() {
                t.status.clone()
            } else if t.loading && t.showing_web() {
                format!("Loading {}…", t.host().unwrap_or_default())
            } else if t.private {
                "Private tab: nothing from this tab is kept".into()
            } else {
                "Ready".into()
            };
            let mut job = egui::text::LayoutJob::single_section(
                text,
                egui::TextFormat { font_id: FontId::proportional(12.0), color: ui::MUTED, ..Default::default() },
            );
            job.wrap = egui::text::TextWrapping {
                max_width: (ui.available_width() - 470.0).max(80.0),
                max_rows: 1,
                break_anywhere: true,
                overflow_character: Some('…'),
            };
            ui.label(job);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(12.0);
                let chip = |ui: &mut Ui, on: bool, label: &str| {
                    let col = if on { ui::GOOD } else { ui::FAINT };
                    ui.label(RichText::new(label).size(12.0).color(if on { ui::MUTED } else { ui::FAINT }));
                    let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 3.5, col);
                    ui.add_space(8.0);
                };
                chip(ui, self.settings.forget_on_close, "Forget on close");
                chip(ui, self.settings.fingerprint_guard, "Fingerprint guard");
                chip(ui, self.settings.https_only, "HTTPS-Only");
                let n = if t.showing_web() { t.blocked_total() } else { 0 };
                chip(
                    ui,
                    self.settings.block_trackers,
                    &format!("{n} tracker{} blocked here", if n == 1 { "" } else { "s" }),
                );
            });
        });
    }

    fn shields_drawer(&mut self, ui: &mut Ui) {
        drawer_header(ui, Icon::Shield, ui::TEAL, "Shields", &mut self.drawer);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            let Some(host) = self.tab().host() else {
                ui.label(RichText::new("Open a website to see what LabSphere blocked on it.").color(ui::MUTED));
                ui.add_space(12.0);
                self.protection_summary(ui);
                return;
            };
            let base = shields::base_domain(&host);
            ui.label(RichText::new(&host).size(18.0).strong().color(ui::TEXT));
            {
                let t = self.tab();
                let scheme = shields::scheme_of(&t.url).to_ascii_lowercase();
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    if scheme == "https" {
                        ui::draw_icon(ui.painter(), r, Icon::Lock, ui::GOOD);
                        ui.label(
                            RichText::new(if t.upgraded {
                                "Secure connection · upgraded to HTTPS"
                            } else {
                                "Secure connection (HTTPS)"
                            })
                            .color(ui::GOOD)
                            .size(13.0),
                        );
                    } else {
                        ui::draw_icon(ui.painter(), r, Icon::Unlock, ui::WARN);
                        ui.label(RichText::new("Not secure: this page isn't encrypted").color(ui::WARN).size(13.0));
                    }
                });
            }
            ui.add_space(10.0);

            let mut up = self.privacy.shields_up_for(&host);
            let global = self.settings.block_trackers;
            let mut flip = false;
            ui::card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(if up && global { "Shields are up" } else { "Shields are down" })
                                .size(15.0)
                                .strong(),
                        );
                        ui.label(RichText::new(format!("for {base}")).color(ui::MUTED).size(12.5));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::toggle(ui, &mut up).changed() {
                            flip = true;
                        }
                    });
                });
                if !global {
                    ui.label(RichText::new("Tracker blocking is off everywhere in Settings.").color(ui::WARN).size(12.5));
                }
            });
            if flip {
                self.toggle_site_shields();
            }
            ui.add_space(8.0);

            let t = self.tab();
            let total = t.blocked_total();
            stat_row(ui, Icon::Eye, "Trackers & ads blocked", total as u64);
            stat_row(ui, Icon::Close, "Video ads removed or skipped", t.video_ads as u64);
            if !t.blocked.is_empty() {
                egui::CollapsingHeader::new(RichText::new("Show blocked services").size(13.0).color(ui::MUTED))
                    .id_salt("blocked_list")
                    .show(ui, |ui| {
                        let mut list = t.blocked.clone();
                        list.sort_by(|a, b| b.2.cmp(&a.2));
                        for (h, cat, n) in list {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(h).size(12.5).color(ui::TEXT));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(RichText::new(format!("{n}× {}", cat.name())).size(11.5).color(ui::FAINT));
                                });
                            });
                        }
                    });
            }
            ui.label(
                RichText::new("Built-in tracking prevention also stops known trackers before they reach this list.")
                    .size(11.5)
                    .color(ui::FAINT),
            );
            stat_row(ui, Icon::Bolt, "Connection upgraded to HTTPS", if t.upgraded { 1 } else { 0 });
            stat_row(ui, Icon::Globe, "Fingerprinting attempts disrupted", t.fingerprint.len() as u64);
            if !t.fingerprint.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "Read: {}",
                        t.fingerprint.iter().map(|k| fp_label(k)).collect::<Vec<_>>().join(", ")
                    ))
                    .size(12.0)
                    .color(ui::FAINT),
                );
            }
            stat_row(ui, Icon::Lock, "Permission requests blocked", t.denied.len() as u64);
            if !t.denied.is_empty() {
                ui.label(RichText::new(format!("Asked for: {}", t.denied.join(", "))).size(12.0).color(ui::FAINT));
            }
            stat_row(ui, Icon::Close, "Pop-ups blocked", t.popups.len() as u64);
            ui.add_space(12.0);
            let mut forget = false;
            let mut settings = false;
            ui.horizontal_wrapped(|ui| {
                if ui::ghost_button(ui, "Forget this site's cookies").clicked() {
                    forget = true;
                }
                if ui::ghost_button(ui, "All Shields settings").clicked() {
                    settings = true;
                }
            });
            if forget {
                let n = self.forget_site_cookies();
                self.tab_mut().info = Some(InfoBar::Note {
                    text: format!("Removed {n} cookie{} for {base}.", if n == 1 { "" } else { "s" }),
                    tone: ui::GOOD,
                });
            }
            if settings {
                self.open_internal(Internal::Settings);
            }
            ui.add_space(14.0);
            ui.separator();
            self.protection_summary(ui);
        });
    }

    fn protection_summary(&self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.label(RichText::new("THIS SESSION").size(11.5).color(ui::FAINT).strong());
        ui.add_space(2.0);
        let s = &self.session;
        for (label, n) in [
            ("Trackers & ads blocked", s.trackers),
            ("Video ads removed or skipped", s.video_ads),
            ("Connections made secure", s.upgrades),
            ("Fingerprinting attempts disrupted", s.fingerprinting),
            ("Permission requests blocked", s.permissions),
            ("Pop-ups blocked", s.popups),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).size(13.0).color(ui::MUTED));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(n.to_string()).size(13.0).color(ui::TEXT).strong());
                });
            });
        }
    }

    fn menu_drawer(&mut self, ui: &mut Ui) {
        drawer_header(ui, Icon::Menu, ui::TEXT, "Menu", &mut self.drawer);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);
            if self.confirm_burn {
                let mut erase = false;
                let mut cancel = false;
                egui::Frame::new()
                    .fill(Color32::from_rgb(52, 30, 36))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(120, 60, 70)))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                            ui::draw_icon(ui.painter(), r, Icon::Burn, ui::BAD);
                            ui.label(RichText::new("Forget everything now?").size(15.0).strong());
                        });
                        ui.label(
                            RichText::new("Closes all tabs and erases cookies, logins, site data, cache and history.")
                                .size(12.5)
                                .color(ui::MUTED),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui::pill_button(ui, "Erase now", ui::BAD, ui::BG).clicked() {
                                erase = true;
                            }
                            if ui::ghost_button(ui, "Cancel").clicked() {
                                cancel = true;
                            }
                        });
                    });
                if erase {
                    self.burn();
                }
                if cancel {
                    self.confirm_burn = false;
                }
                ui.add_space(10.0);
            }
            let web = self.tab().showing_web();
            let starred = web && self.settings.is_bookmarked(&self.tab().url);
            if menu_item(ui, Icon::Plus, "New tab", "Ctrl+T") {
                self.new_tab(false);
                self.focus_address = true;
                self.drawer = None;
            }
            if menu_item(ui, Icon::Private, "New private tab", "Ctrl+Shift+N") {
                self.new_tab(true);
                self.focus_address = true;
                self.drawer = None;
            }
            ui.separator();
            if web
                && menu_item(
                    ui,
                    if starred { Icon::StarFilled } else { Icon::Star },
                    if starred { "Remove from saved sites" } else { "Save this site" },
                    "Ctrl+D",
                )
            {
                self.toggle_bookmark();
            }
            if menu_item(ui, Icon::Clock, "History", "Ctrl+H") {
                self.open_internal(Internal::History);
            }
            if menu_item(ui, Icon::Gear, "Settings", "") {
                self.open_internal(Internal::Settings);
            }
            if !self.confirm_burn && menu_item(ui, Icon::Burn, "Forget everything now", "") {
                self.confirm_burn = true;
            }
            ui.separator();
            if web {
                let zoom = self.tab().zoom;
                let mut new_zoom = None;
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Zoom").size(14.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui::icon_button(ui, Icon::Plus, 28.0, true, false, "Zoom in").clicked() {
                            new_zoom = Some(zoom + 0.1);
                        }
                        ui.label(RichText::new(format!("{:.0}%", zoom * 100.0)).size(13.0).color(ui::MUTED));
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(8), Color32::from_rgb(36, 43, 67));
                        }
                        ui.painter().line_segment(
                            [r.center() - vec2(7.0, 0.0), r.center() + vec2(7.0, 0.0)],
                            Stroke::new(2.0_f32, ui::TEXT),
                        );
                        if resp.clicked() {
                            new_zoom = Some(zoom - 0.1);
                        }
                    });
                });
                if let Some(z) = new_zoom {
                    self.set_zoom(z);
                }
                if menu_item(ui, Icon::Globe, "Print…", "") {
                    if let Some(p) = &self.tab().page {
                        p.print();
                    }
                    self.drawer = None;
                }
                ui.separator();
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                ui::draw_logo(ui.painter(), r.center(), 9.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("LabSphere {}", env!("CARGO_PKG_VERSION"))).size(13.5).strong());
                    let eng = match &self.engine_version {
                        Some(v) => format!("Chromium engine (WebView2) {v}"),
                        None => "Chromium engine not found".into(),
                    };
                    ui.label(RichText::new(eng).size(12.0).color(ui::FAINT));
                });
            });
        });
    }

    fn page_area(&mut self, ui: &mut Ui) {
        let internal = self.tab().internal;
        match internal {
            Some(Internal::Home) => self.home_page(ui),
            Some(Internal::Settings) => self.settings_page(ui),
            Some(Internal::History) => self.history_page(ui),
            None => {
                let rect = ui.max_rect();
                let t = self.tab();
                if let Some(err) = &self.engine_error {
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(60.0)), |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(rect.height() * 0.2);
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                            ui::draw_icon(ui.painter(), r, Icon::ShieldOff, ui::WARN);
                            ui.label(RichText::new("This page can't be shown").size(22.0).strong());
                            ui.label(RichText::new(err).color(ui::MUTED));
                        });
                    });
                } else if !self.start_page_engine || (t.page.is_none() && t.pending_url.is_some()) {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "Opening a protected page…",
                        FontId::proportional(15.0),
                        ui::MUTED,
                    );
                }
            }
        }
    }

    fn run_selftest(&mut self, ctx: &egui::Context) {
        let Some(st) = &mut self.selftest else { return };
        ctx.request_repaint_after(Duration::from_millis(100));
        let t = &self.tabs[self.active];
        if st.started.is_none() && t.page.is_some() {
            st.started = Some(Instant::now());
        }
        let Some(started) = st.started else {
            if self.launched.elapsed() > Duration::from_secs(30) {
                st.log.push(format!("engine error: {:?}", self.engine_error));
                write_selftest(st, &self.tabs[self.active], &self.session);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        };
        let el = started.elapsed();
        if !st.injected && el > Duration::from_secs(6) {
            st.injected = true;
            if let Some(p) = &t.page {
                p.run_script(
                    "(()=>{const s=document.createElement('script');s.src='https://www.googletagmanager.com/gtag/js?id=G-TEST';document.head.appendChild(s);\
                     const i=new Image();i.src='https://www.google-analytics.com/collect?v=1&t=pageview';\
                     const f=document.createElement('iframe');f.src='https://securepubads.g.doubleclick.net/';document.body.appendChild(f);\
                     const c=document.createElement('canvas');c.getContext('2d').fillRect(0,0,4,4);c.toDataURL();\
                     const L=((document.body&&document.body.innerText)||'').split(String.fromCharCode(10));\
                     const m=L.find(l=>/sec-ch-ua[^-]/i.test(l))||'none';const g=L.find(l=>/sec-gpc/i.test(l))||'none';\
                     document.title='GPC='+navigator.globalPrivacyControl+' cores='+navigator.hardwareConcurrency+' brands='+(navigator.userAgentData?navigator.userAgentData.brands.map(b=>b.brand).join('/'):'-')+' hdr='+m.trim()+' gpc='+g.trim();})()",
                );
                st.log.push("injected tracker test".into());
            }
        }
        if el > Duration::from_secs(10) {
            st.log.push(format!("engine error: {:?}", self.engine_error));
            write_selftest(st, &self.tabs[self.active], &self.session);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            self.selftest = None;
        }
    }
}

fn write_selftest(st: &SelfTest, t: &Tab, s: &SessionStats) {
    let mut out = String::new();
    out.push_str(&format!(
        "requested: {}\nurl: {}\ntitle: {}\nupgraded: {}\n",
        st.url, t.url, t.title, t.upgraded
    ));
    out.push_str(&format!("blocked_total: {}\n", t.blocked_total()));
    for b in &t.blocked {
        out.push_str(&format!("  blocked {} ({:?}) x{}\n", b.0, b.1, b.2));
    }
    out.push_str(&format!("fingerprint: {:?}\n", t.fingerprint));
    out.push_str(&format!("video_ads: {}\n", t.video_ads));
    let info = match &t.info {
        Some(InfoBar::HttpsFailed { http_url }) => format!("https-failed {http_url}"),
        Some(InfoBar::Popup { url }) => format!("popup {url}"),
        Some(InfoBar::Permission { what, .. }) => format!("permission {what}"),
        Some(InfoBar::Note { text, .. }) => format!("note {text}"),
        None => "none".into(),
    };
    out.push_str(&format!("infobar: {info}\n"));
    out.push_str(&format!(
        "session: trackers={} upgrades={} fp={}\n",
        s.trackers, s.upgrades, s.fingerprinting
    ));
    for l in &st.log {
        out.push_str(&format!("log: {l}\n"));
    }
    let _ = std::fs::write(&st.out, out);
}

fn select_all(out: &mut egui::text_edit::TextEditOutput, ctx: &egui::Context, text: &str) {
    let n = text.chars().count();
    out.state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(0),
        egui::text::CCursor::new(n),
    )));
    out.state.clone().store(ctx, out.response.id);
}

fn drawer_header(ui: &mut Ui, icon: Icon, color: Color32, title: &str, drawer: &mut Option<Drawer>) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
        ui::draw_icon(ui.painter(), r, icon, color);
        ui.label(RichText::new(title).size(17.0).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui::icon_button(ui, Icon::Close, 28.0, true, false, "Close").clicked() {
                *drawer = None;
            }
        });
    });
    ui.add_space(6.0);
}

fn stat_row(ui: &mut Ui, icon: Icon, label: &str, n: u64) {
    ui.horizontal(|ui| {
        ui.set_min_height(30.0);
        let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
        ui::draw_icon(ui.painter(), r, icon, if n > 0 { ui::TEAL } else { ui::FAINT });
        ui.label(RichText::new(label).size(13.5).color(ui::TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(n.to_string())
                    .size(16.0)
                    .strong()
                    .color(if n > 0 { ui::TEAL } else { ui::MUTED }),
            );
        });
    });
}

fn menu_item(ui: &mut Ui, icon: Icon, label: &str, keys: &str) -> bool {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(34, 40, 62));
    }
    let ic = Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), Vec2::splat(18.0));
    ui::draw_icon(ui.painter(), ic, icon, ui::MUTED);
    ui.painter().text(
        pos2(rect.left() + 42.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        ui::TEXT,
    );
    if !keys.is_empty() {
        ui.painter().text(
            pos2(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            keys,
            FontId::proportional(12.0),
            ui::FAINT,
        );
    }
    resp.clicked()
}

pub fn fp_label(kind: &str) -> &str {
    match kind {
        "canvas" => "canvas image",
        "webgl" => "graphics card",
        "audio" => "audio processing",
        "hardware" => "hardware details",
        "battery" => "battery level",
        "client-hints" => "detailed browser version",
        other => other,
    }
}

pub enum Target {
    Web(String),
    Internal(Internal),
}

/// Turn what the person typed into a page to open.
pub fn resolve_input(input: &str, settings: &Settings) -> Option<Target> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("labsphere://") {
        return Some(Target::Internal(match rest.trim_end_matches('/') {
            "settings" => Internal::Settings,
            "history" => Internal::History,
            _ => Internal::Home,
        }));
    }
    if lower.starts_with("http://") || lower.starts_with("https://") || lower == "about:blank" {
        return Some(Target::Web(s.to_string()));
    }
    if lower.starts_with("javascript:") || lower.starts_with("file:") || lower.starts_with("data:") {
        return Some(Target::Web(settings.search.query_url(s)));
    }
    if !s.contains(char::is_whitespace) {
        let host_part = lower.split(['/', '?', '#']).next().unwrap_or("");
        let host_only = host_part
            .rsplit_once(':')
            .filter(|(_, p)| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            .map(|(h, _)| h)
            .unwrap_or(host_part);
        let tld_ok = host_only
            .rsplit('.')
            .next()
            .map(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(false);
        let looks_like_host = host_only == "localhost"
            || host_only.parse::<std::net::IpAddr>().is_ok()
            || (host_only.contains('.') && !host_only.starts_with('.') && !host_only.ends_with('.') && tld_ok);
        if looks_like_host {
            let scheme = if shields::is_local_host(host_only) { "http" } else { "https" };
            return Some(Target::Web(format!("{scheme}://{s}")));
        }
    }
    Some(Target::Web(settings.search.query_url(s)))
}

fn purge_old_sessions(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let me = format!("{}-", std::process::id());
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with(&me) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

impl eframe::App for LabSphere {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        ui::BG.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.process_events(ctx);
        self.keyboard(ctx);

        // Clicking the LabSphere controls takes the keyboard back from the page.
        if ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(p) = &self.tab().page {
                p.focus_window();
            }
        }

        if !self.fullscreen {
            egui::TopBottomPanel::top("tabs")
                .exact_height(42.0)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(ui::STRIP))
                .show(ctx, |ui| self.tab_strip(ui));
            egui::TopBottomPanel::top("toolbar")
                .exact_height(50.0)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(ui::BAR))
                .show(ctx, |ui| self.toolbar(ui));
            let private = self.tab().private;
            egui::TopBottomPanel::top("toolbar_line")
                .exact_height(1.0)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(if private {
                    ui::alpha(ui::PRIVATE, 0.6)
                } else {
                    Color32::from_rgb(34, 40, 62)
                }))
                .show(ctx, |_| {});
            if self.tab().info.is_some() {
                let tone = match &self.tab().info {
                    Some(InfoBar::HttpsFailed { .. }) => Color32::from_rgb(52, 44, 22),
                    Some(InfoBar::Note { .. }) => Color32::from_rgb(20, 44, 36),
                    _ => Color32::from_rgb(20, 38, 48),
                };
                egui::TopBottomPanel::top("infobar")
                    .exact_height(44.0)
                    .show_separator_line(false)
                    .frame(egui::Frame::new().fill(tone))
                    .show(ctx, |ui| self.info_bar(ui));
            }
            egui::TopBottomPanel::bottom("status")
                .exact_height(26.0)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(ui::STRIP))
                .show(ctx, |ui| self.status_bar(ui));
            if let Some(d) = self.drawer {
                egui::SidePanel::right("drawer")
                    .exact_width(340.0)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgb(18, 22, 36))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(34, 40, 62)))
                            .inner_margin(egui::Margin { left: 16, right: 14, top: 4, bottom: 10 }),
                    )
                    .show(ctx, |ui| match d {
                        Drawer::Shields => self.shields_drawer(ui),
                        Drawer::Menu => self.menu_drawer(ui),
                    });
            }
        }
        let content = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ui::BG))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                self.page_area(ui);
                r
            })
            .inner;
        self.sync_pages(ctx, frame, content);

        // Window title
        let t = self.tab();
        let title = if t.internal.is_some() && !t.private {
            "LabSphere".to_string()
        } else {
            format!("{}{} — LabSphere", if t.private { "Private · " } else { "" }, t.display_title())
        };
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        if self.dirty && self.persist && self.last_save.elapsed() > Duration::from_secs(3) {
            self.settings.save();
            self.dirty = false;
            self.last_save = Instant::now();
        }

        self.run_selftest(ctx);

        // --screenshot: draw ~120 frames, save the window's picture, quit.
        if let Some(shot) = &mut self.shot {
            shot.frames += 1;
            let mut done = false;
            ctx.input(|i| {
                for ev in &i.raw.events {
                    if let egui::Event::Screenshot { image, .. } = ev {
                        let [w, h] = image.size;
                        let mut bytes = Vec::with_capacity(w * h * 4);
                        for p in &image.pixels {
                            bytes.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
                        }
                        if let Some(parent) = shot.path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Err(e) =
                            image::save_buffer(&shot.path, &bytes, w as u32, h as u32, image::ColorType::Rgba8)
                        {
                            eprintln!("LabSphere: couldn't save the screenshot: {e}");
                        }
                        done = true;
                    }
                }
            });
            if done || shot.frames > 900 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if shot.frames >= 120 && !shot.asked {
                shot.asked = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.persist {
            self.settings.save();
        }
        self.tabs.clear();
        self.engine = None;
    }
}
