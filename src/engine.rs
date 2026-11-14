//! The page engine: Microsoft WebView2 (Chromium) hosted as a child of the
//! LabSphere window, with LabSphere Shields wired into its request pipeline.

use crate::shields::{Category, Perm, Privacy};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    NewTab,
    NewPrivateTab,
    CloseTab,
    FocusAddress,
    NextTab,
    PrevTab,
    Reload,
}

#[derive(Debug)]
pub enum EngineEvent {
    NavStarting { tab: u64, url: String },
    Upgraded { tab: u64, https_url: String },
    UpgradeFailed { tab: u64, http_url: String },
    Title { tab: u64, title: String },
    Loading { tab: u64, started: bool, url: String },
    UrlChanged { tab: u64, url: String },
    Blocked { tab: u64, host: String, category: Category },
    Fingerprint { tab: u64, kind: String },
    PermissionDenied { tab: u64, what: &'static str },
    PopupBlocked { tab: u64, url: String },
    OpenTab { from: u64, url: String },
    Shortcut(Shortcut),
    StatusText { tab: u64, text: String },
    FullScreen { tab: u64, on: bool },
    Closed { tab: u64 },
}

/// Sends events to the window and wakes it up to draw them.
#[derive(Clone)]
pub struct Notifier {
    pub tx: Sender<EngineEvent>,
    pub ctx: eframe::egui::Context,
}

impl Notifier {
    pub fn send(&self, ev: EngineEvent) {
        let _ = self.tx.send(ev);
        self.ctx.request_repaint();
    }
}

/// What the request filter needs to know about its tab, updated from callbacks.
#[derive(Default)]
pub struct TabShared {
    pub main_url: Mutex<String>,
    pub pending_upgrade: Mutex<Option<String>>,
    pub upgrade_hops: Mutex<(String, u32)>,
}

/// Live engine options that apply per page (others need a new tab or restart).
#[derive(Clone, Debug)]
pub struct PageOptions {
    pub fingerprint_guard: bool,
    pub javascript: bool,
    pub save_passwords: bool,
    pub autofill: bool,
    pub smartscreen: bool,
    pub tracking_level: crate::settings::Tracking,
}

pub fn engine_version() -> Option<String> {
    #[cfg(windows)]
    {
        wry::webview_version().ok()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
pub use win::{Engine, Page};

#[cfg(not(windows))]
pub use other::{Engine, Page};

#[cfg(windows)]
mod win {
    use super::*;
    use crate::shields::Decision;
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT};
    use windows_core::{BOOL, HSTRING, Interface, PWSTR, w};
    use wry::dpi::{PhysicalPosition, PhysicalSize};
    use wry::{
        PageLoadEvent, PermissionKind, PermissionResponse, Rect, WebContext, WebView, WebViewBuilder,
        WebViewBuilderExtWindows, WebViewExtWindows,
    };

    pub struct Engine {
        web_context: WebContext,
        env: Option<ICoreWebView2Environment>,
        privacy: Arc<Privacy>,
        notifier: Notifier,
        browser_args: String,
        user_agent: Option<String>,
        seed: u32,
        pub failed: Option<String>,
    }

    /// One tab's page view.
    pub struct Page {
        pub view: WebView,
        pub shared: Arc<TabShared>,
        last_bounds: [i32; 4],
        visible: bool,
    }

    impl Engine {
        pub fn new(data_dir: std::path::PathBuf, privacy: Arc<Privacy>, notifier: Notifier, webrtc_guard: bool) -> Engine {
            let _ = std::fs::create_dir_all(&data_dir);
            let mut args = String::from(
                "--disable-features=msWebOOUI,msPdfOOUI,AutofillServerCommunication,OptimizationHints,\
                 MediaRouter,PrivacySandboxAdsAPIs,InterestFeedContentSuggestions,BrowsingTopics,Fledge,\
                 msEdgeShoppingUI,msImplicitSignin,msEdgeTranslate \
                 --no-pings --disable-domain-reliability",
            );
            if webrtc_guard {
                args.push_str(" --force-webrtc-ip-handling-policy=disable_non_proxied_udp --webrtc-ip-handling-policy=disable_non_proxied_udp");
            }
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| (d.as_nanos() as u64 ^ (std::process::id() as u64) << 20) as u32)
                .unwrap_or(0x5eed_1ab5);
            Engine {
                web_context: WebContext::new(Some(data_dir)),
                env: None,
                privacy,
                notifier,
                browser_args: args,
                user_agent: None,
                seed,
                failed: None,
            }
        }

        fn chrome_like_user_agent() -> Option<String> {
            let version = wry::webview_version().ok()?;
            let major = version.split('.').next()?.trim().to_string();
            Some(format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{major}.0.0.0 Safari/537.36"
            ))
        }

        /// Make a new page view for `tab` inside the window and load `url`.
        pub fn create(
            &mut self,
            window: &eframe::Frame,
            tab: u64,
            private: bool,
            url: &str,
            bounds: [i32; 4],
            opts: &PageOptions,
        ) -> std::result::Result<Page, String> {
            if self.user_agent.is_none() && self.privacy.blend_in.load(std::sync::atomic::Ordering::Relaxed) {
                self.user_agent = Self::chrome_like_user_agent();
            }
            let shared = Arc::new(TabShared::default());
            let n = self.notifier.clone();

            let mut builder = match &self.env {
                Some(env) => WebViewBuilder::new().with_environment(env.clone()),
                None => WebViewBuilder::new_with_web_context(&mut self.web_context),
            };
            builder = builder
                .with_bounds(rect(bounds))
                .with_visible(true)
                .with_focused(false)
                .with_incognito(private)
                .with_devtools(false)
                .with_clipboard(false)
                .with_hotkeys_zoom(true)
                .with_general_autofill_enabled(opts.autofill)
                .with_background_color((14, 17, 28, 255))
                .with_additional_browser_args(self.browser_args.clone())
                .with_browser_accelerator_keys(true)
                .with_default_context_menus(true);
            if let Some(ua) = &self.user_agent {
                builder = builder.with_user_agent(ua.clone());
            }
            if !opts.javascript {
                builder = builder.with_javascript_disabled();
            }
            if opts.fingerprint_guard {
                builder = builder.with_initialization_script(crate::shields::fingerprint_script(self.seed));
                let n2 = n.clone();
                builder = builder.with_ipc_handler(move |req: wry::http::Request<String>| {
                    if let Some(kind) = req.body().strip_prefix("fp:") {
                        let kind: String = kind.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(24).collect();
                        n2.send(EngineEvent::Fingerprint { tab, kind });
                    }
                });
            }

            // HTTPS-only and navigation bookkeeping (main frame only).
            {
                let n = n.clone();
                let privacy = self.privacy.clone();
                let shared = shared.clone();
                builder = builder.with_navigation_handler(move |url: String| {
                    let scheme = crate::shields::scheme_of(&url).to_ascii_lowercase();
                    if scheme == "labsphere" || scheme == "file" {
                        return false;
                    }
                    if let Some(https) = privacy.upgrade(&url) {
                        let host = crate::shields::host_of(&url).unwrap_or_default();
                        let mut hops = shared.upgrade_hops.lock().unwrap();
                        if hops.0 == host {
                            hops.1 += 1;
                        } else {
                            *hops = (host, 1);
                        }
                        let looping = hops.1 > 3;
                        drop(hops);
                        if looping {
                            // The site keeps sending us back to plain HTTP.
                            n.send(EngineEvent::UpgradeFailed { tab, http_url: url });
                        } else {
                            *shared.pending_upgrade.lock().unwrap() = Some(https.clone());
                            n.send(EngineEvent::Upgraded { tab, https_url: https });
                        }
                        return false;
                    }
                    *shared.main_url.lock().unwrap() = url.clone();
                    n.send(EngineEvent::NavStarting { tab, url });
                    true
                });
            }
            {
                let n = n.clone();
                builder = builder.with_document_title_changed_handler(move |title| {
                    n.send(EngineEvent::Title { tab, title });
                });
            }
            {
                let n = n.clone();
                builder = builder.with_on_page_load_handler(move |ev, url| {
                    n.send(EngineEvent::Loading { tab, started: matches!(ev, PageLoadEvent::Started), url });
                });
            }
            {
                let n = n.clone();
                let privacy = self.privacy.clone();
                builder = builder.with_permission_handler(move |kind| {
                    let perm = match kind {
                        PermissionKind::Camera | PermissionKind::DisplayCapture => Perm::Camera,
                        PermissionKind::Microphone => Perm::Microphone,
                        PermissionKind::Geolocation => Perm::Location,
                        PermissionKind::Notifications => Perm::Notifications,
                        PermissionKind::ClipboardRead => Perm::Clipboard,
                        PermissionKind::Midi
                        | PermissionKind::Sensors
                        | PermissionKind::LocalFonts
                        | PermissionKind::WindowManagement
                        | PermissionKind::MediaKeySystemAccess => Perm::Other,
                        _ => return PermissionResponse::Default,
                    };
                    if privacy.perm_blocked(perm) {
                        n.send(EngineEvent::PermissionDenied { tab, what: perm.name() });
                        PermissionResponse::Deny
                    } else {
                        PermissionResponse::Default
                    }
                });
            }

            let view = builder.build_as_child(window).map_err(|e| format!("{e}"))?;
            if self.env.is_none() {
                self.env = Some(view.environment());
            }
            unsafe {
                if let Err(e) = install_hooks(&view, tab, private, &shared, &self.privacy, &n, opts) {
                    eprintln!("LabSphere: some protections could not be attached: {e:?}");
                }
            }
            let page = Page { view, shared, last_bounds: bounds, visible: true };
            page.navigate(url);
            Ok(page)
        }
    }

    fn rect(b: [i32; 4]) -> Rect {
        Rect {
            position: PhysicalPosition::new(b[0], b[1]).into(),
            size: PhysicalSize::new(b[2].max(1) as u32, b[3].max(1) as u32).into(),
        }
    }

    impl Page {
        pub fn navigate(&self, url: &str) {
            *self.shared.main_url.lock().unwrap() = url.to_string();
            let _ = self.view.load_url(url);
        }

        pub fn place(&mut self, bounds: [i32; 4], visible: bool) {
            if visible && bounds != self.last_bounds {
                let _ = self.view.set_bounds(rect(bounds));
                self.last_bounds = bounds;
            }
            if visible != self.visible {
                let _ = self.view.set_visible(visible);
                self.visible = visible;
            }
        }

        pub fn back(&self) {
            let _ = self.view.go_back();
        }
        pub fn forward(&self) {
            let _ = self.view.go_forward();
        }
        pub fn reload(&self) {
            let _ = self.view.reload();
        }
        pub fn stop(&self) {
            unsafe {
                let _ = self.view.webview().Stop();
            }
        }
        pub fn can_back(&self) -> bool {
            self.view.can_go_back().unwrap_or(false)
        }
        pub fn can_forward(&self) -> bool {
            self.view.can_go_forward().unwrap_or(false)
        }
        pub fn zoom(&self, z: f64) {
            let _ = self.view.zoom(z);
        }
        pub fn print(&self) {
            let _ = self.view.print();
        }
        pub fn focus_window(&self) {
            let _ = self.view.focus_parent();
        }
        pub fn run_script(&self, js: &str) {
            let _ = self.view.evaluate_script(js);
        }

        pub fn set_javascript(&self, on: bool) {
            unsafe {
                if let Ok(s) = self.view.webview().Settings() {
                    let _ = s.SetIsScriptEnabled(on);
                }
            }
        }

        pub fn apply_options(&self, opts: &PageOptions) {
            unsafe {
                let _ = apply_settings(&self.view.webview(), opts);
            }
        }

        /// Forget cookies, storage and cache for every site in this tab's profile.
        pub fn clear_profile_data(&self) {
            let _ = self.view.clear_all_browsing_data();
        }

        /// Delete the cookies a site has set.
        pub fn clear_site_cookies(&self, url: &str) -> usize {
            let mut n = 0;
            if let Ok(cookies) = self.view.cookies_for_url(url) {
                for c in cookies {
                    if self.view.delete_cookie(&c).is_ok() {
                        n += 1;
                    }
                }
            }
            n
        }
    }

    unsafe fn apply_settings(core: &ICoreWebView2, opts: &PageOptions) -> windows_core::Result<()> {
        unsafe {
            let settings = core.Settings()?;
            settings.SetIsScriptEnabled(opts.javascript)?;
            settings.SetIsStatusBarEnabled(false)?;
            if let Ok(s4) = settings.cast::<ICoreWebView2Settings4>() {
                let _ = s4.SetIsPasswordAutosaveEnabled(opts.save_passwords);
                let _ = s4.SetIsGeneralAutofillEnabled(opts.autofill);
            }
            if let Ok(s8) = settings.cast::<ICoreWebView2Settings8>() {
                let _ = s8.SetIsReputationCheckingRequired(opts.smartscreen);
            }
            if let Ok(c13) = core.cast::<ICoreWebView2_13>() {
                if let Ok(profile) = c13.Profile() {
                    if let Ok(p3) = profile.cast::<ICoreWebView2Profile3>() {
                        let level = match opts.tracking_level {
                            crate::settings::Tracking::Basic => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BASIC,
                            crate::settings::Tracking::Balanced => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_BALANCED,
                            crate::settings::Tracking::Strict => COREWEBVIEW2_TRACKING_PREVENTION_LEVEL_STRICT,
                        };
                        let _ = p3.SetPreferredTrackingPreventionLevel(level);
                    }
                    if let Ok(p6) = profile.cast::<ICoreWebView2Profile6>() {
                        let _ = p6.SetIsPasswordAutosaveEnabled(opts.save_passwords);
                        let _ = p6.SetIsGeneralAutofillEnabled(opts.autofill);
                    }
                }
            }
            Ok(())
        }
    }

    fn header(headers: &ICoreWebView2HttpRequestHeaders, name: &HSTRING) -> Option<String> {
        unsafe {
            let mut has = BOOL::default();
            headers.Contains(name, &mut has).ok()?;
            if !has.as_bool() {
                return None;
            }
            let mut v = PWSTR::null();
            headers.GetHeader(name, &mut v).ok()?;
            Some(take_pwstr(v))
        }
    }

    unsafe fn install_hooks(
        view: &WebView,
        tab: u64,
        _private: bool,
        shared: &Arc<TabShared>,
        privacy: &Arc<Privacy>,
        n: &Notifier,
        opts: &PageOptions,
    ) -> windows_core::Result<()> {
        unsafe {
            let core = view.webview();
            let controller = view.controller();
            let env = view.environment();
            let mut token = 0i64;

            apply_settings(&core, opts)?;

            // Every request goes past the shields first.
            match core.cast::<ICoreWebView2_22>() {
                Ok(c22) => c22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                    w!("*"),
                    COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                    COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
                )?,
                Err(_) => core.AddWebResourceRequestedFilter(w!("*"), COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)?,
            }
            {
                let shared = shared.clone();
                let privacy = privacy.clone();
                let n = n.clone();
                let reason = HSTRING::from("Blocked by LabSphere Shields");
                let empty = HSTRING::new();
                let sec_gpc = HSTRING::from("Sec-GPC");
                let dnt = HSTRING::from("DNT");
                let one = HSTRING::from("1");
                let referer = HSTRING::from("Referer");
                let ch_ua = HSTRING::from("sec-ch-ua");
                let ch_full = HSTRING::from("sec-ch-ua-full-version-list");
                core.add_WebResourceRequested(
                    &WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let request = args.Request()?;
                        let mut uri = PWSTR::null();
                        request.Uri(&mut uri)?;
                        let uri = take_pwstr(uri);
                        if !(uri.starts_with("http://") || uri.starts_with("https://")) {
                            return Ok(());
                        }
                        let mut context = COREWEBVIEW2_WEB_RESOURCE_CONTEXT::default();
                        args.ResourceContext(&mut context)?;
                        let page = shared.main_url.lock().map(|s| s.clone()).unwrap_or_default();
                        let is_main = context == COREWEBVIEW2_WEB_RESOURCE_CONTEXT_DOCUMENT
                            && (uri == page || page.is_empty());
                        match privacy.decide(&uri, &page, is_main) {
                            Decision::Block { host, category } => {
                                let response = env.CreateWebResourceResponse(None, 403, &reason, &empty)?;
                                args.SetResponse(&response)?;
                                n.send(EngineEvent::Blocked { tab, host, category });
                            }
                            Decision::Allow { third_party } => {
                                let headers = request.Headers()?;
                                if privacy.send_gpc.load(std::sync::atomic::Ordering::Relaxed) {
                                    let _ = headers.SetHeader(&sec_gpc, &one);
                                    let _ = headers.SetHeader(&dnt, &one);
                                }
                                if third_party && privacy.trim_referrers.load(std::sync::atomic::Ordering::Relaxed) {
                                    if let Some(r) = header(&headers, &referer) {
                                        if let Some(host) = crate::shields::host_of(&r) {
                                            let origin = format!("{}://{}/", crate::shields::scheme_of(&r), host);
                                            if origin != r {
                                                let _ = headers.SetHeader(&referer, &HSTRING::from(origin));
                                            }
                                        }
                                    }
                                }
                                if privacy.blend_in.load(std::sync::atomic::Ordering::Relaxed) {
                                    for name in [&ch_ua, &ch_full] {
                                        if let Some(v) = header(&headers, name) {
                                            let clean = crate::shields::generic_brands(&v);
                                            if clean != v {
                                                let _ = headers.SetHeader(name, &HSTRING::from(clean));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // Pop-ups: links the person clicked open as tabs; the rest are stopped.
            {
                let privacy = privacy.clone();
                let n = n.clone();
                core.add_NewWindowRequested(
                    &NewWindowRequestedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let mut uri = PWSTR::null();
                        args.Uri(&mut uri)?;
                        let url = take_pwstr(uri);
                        let mut by_person = BOOL::default();
                        let _ = args.IsUserInitiated(&mut by_person);
                        args.SetHandled(true)?;
                        if by_person.as_bool() || !privacy.block_popups.load(std::sync::atomic::Ordering::Relaxed) {
                            n.send(EngineEvent::OpenTab { from: tab, url });
                        } else {
                            n.send(EngineEvent::PopupBlocked { tab, url });
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // Did an HTTPS upgrade fail to connect?
            {
                let shared = shared.clone();
                let n = n.clone();
                core.add_NavigationCompleted(
                    &NavigationCompletedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let mut ok = BOOL::default();
                        args.IsSuccess(&mut ok)?;
                        let mut status = COREWEBVIEW2_WEB_ERROR_STATUS::default();
                        let _ = args.WebErrorStatus(&mut status);
                        if !ok.as_bool() && status == COREWEBVIEW2_WEB_ERROR_STATUS_OPERATION_CANCELED {
                            return Ok(());
                        }
                        let pending = shared.pending_upgrade.lock().unwrap().take();
                        if !ok.as_bool() {
                            let connection_problem = status != COREWEBVIEW2_WEB_ERROR_STATUS_HOST_NAME_NOT_RESOLVED
                                && status != COREWEBVIEW2_WEB_ERROR_STATUS_DISCONNECTED;
                            if let Some(https) = pending {
                                if connection_problem {
                                    let http_url = format!("http://{}", https.trim_start_matches("https://"));
                                    n.send(EngineEvent::UpgradeFailed { tab, http_url });
                                }
                            }
                        } else {
                            *shared.upgrade_hops.lock().unwrap() = (String::new(), 0);
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // Address changes inside single-page sites.
            {
                let n = n.clone();
                core.add_SourceChanged(
                    &SourceChangedEventHandler::create(Box::new(move |sender, _| {
                        if let Some(sender) = sender {
                            let mut s = PWSTR::null();
                            sender.Source(&mut s)?;
                            n.send(EngineEvent::UrlChanged { tab, url: take_pwstr(s) });
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // Link address under the mouse, for the status line.
            if let Ok(c12) = core.cast::<ICoreWebView2_12>() {
                let n = n.clone();
                c12.add_StatusBarTextChanged(
                    &StatusBarTextChangedEventHandler::create(Box::new(move |sender, _| {
                        if let Some(sender) = sender.and_then(|s| s.cast::<ICoreWebView2_12>().ok()) {
                            let mut s = PWSTR::null();
                            sender.StatusBarText(&mut s)?;
                            n.send(EngineEvent::StatusText { tab, text: take_pwstr(s) });
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // Full-screen video.
            {
                let n = n.clone();
                core.add_ContainsFullScreenElementChanged(
                    &ContainsFullScreenElementChangedEventHandler::create(Box::new(move |sender, _| {
                        if let Some(sender) = sender {
                            let mut on = BOOL::default();
                            sender.ContainsFullScreenElement(&mut on)?;
                            n.send(EngineEvent::FullScreen { tab, on: on.as_bool() });
                        }
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // window.close() from the page closes the tab.
            {
                let n = n.clone();
                core.add_WindowCloseRequested(
                    &WindowCloseRequestedEventHandler::create(Box::new(move |_, _| {
                        n.send(EngineEvent::Closed { tab });
                        Ok(())
                    })),
                    &mut token,
                )?;
            }

            // Browser shortcuts still work while the page has the keyboard.
            {
                let n = n.clone();
                controller.add_AcceleratorKeyPressed(
                    &AcceleratorKeyPressedEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
                        args.KeyEventKind(&mut kind)?;
                        if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN {
                            return Ok(());
                        }
                        let mut vk = 0u32;
                        args.VirtualKey(&mut vk)?;
                        let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
                        let shift = GetKeyState(VK_SHIFT.0 as i32) < 0;
                        let shortcut = match (ctrl, shift, vk) {
                            (true, false, 0x54) => Shortcut::NewTab,
                            (true, true, 0x4E) => Shortcut::NewPrivateTab,
                            (true, false, 0x57) | (true, false, 0x73) => Shortcut::CloseTab,
                            (true, false, 0x4C) => Shortcut::FocusAddress,
                            (true, false, 0x09) => Shortcut::NextTab,
                            (true, true, 0x09) => Shortcut::PrevTab,
                            _ => return Ok(()),
                        };
                        args.SetHandled(true)?;
                        n.send(EngineEvent::Shortcut(shortcut));
                        Ok(())
                    })),
                    &mut token,
                )?;
            }
            Ok(())
        }
    }
}

#[cfg(not(windows))]
mod other {
    use super::*;

    /// Chromium page engine is only wired up for Windows in this version.
    pub struct Engine {
        pub failed: Option<String>,
    }
    pub struct Page {
        pub shared: Arc<TabShared>,
    }

    impl Engine {
        pub fn new(_d: std::path::PathBuf, _p: Arc<Privacy>, _n: Notifier, _w: bool) -> Engine {
            Engine { failed: Some("The Chromium page engine is available on Windows.".into()) }
        }
        pub fn create(
            &mut self,
            _w: &eframe::Frame,
            _tab: u64,
            _private: bool,
            _url: &str,
            _b: [i32; 4],
            _o: &PageOptions,
        ) -> std::result::Result<Page, String> {
            Err(self.failed.clone().unwrap_or_default())
        }
    }

    impl Page {
        pub fn navigate(&self, _u: &str) {}
        pub fn place(&mut self, _b: [i32; 4], _v: bool) {}
        pub fn back(&self) {}
        pub fn forward(&self) {}
        pub fn reload(&self) {}
        pub fn stop(&self) {}
        pub fn can_back(&self) -> bool {
            false
        }
        pub fn can_forward(&self) -> bool {
            false
        }
        pub fn zoom(&self, _z: f64) {}
        pub fn print(&self) {}
        pub fn focus_window(&self) {}
        pub fn run_script(&self, _js: &str) {}
        pub fn set_javascript(&self, _on: bool) {}
        pub fn apply_options(&self, _o: &PageOptions) {}
        pub fn clear_profile_data(&self) {}
        pub fn clear_site_cookies(&self, _u: &str) -> usize {
            0
        }
    }
}
