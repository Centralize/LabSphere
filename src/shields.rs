//! LabSphere Shields: tracker blocking, HTTPS-only rules, fingerprint guard and
//! permission policy, shared between the window and the page engine.

use crate::settings::{PermMode, Settings};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Category {
    Ads,
    Analytics,
    Social,
    Fingerprinting,
    Cryptomining,
}

impl Category {
    pub fn name(self) -> &'static str {
        match self {
            Category::Ads => "Advertising",
            Category::Analytics => "Analytics",
            Category::Social => "Social tracking",
            Category::Fingerprinting => "Fingerprinting",
            Category::Cryptomining => "Cryptomining",
        }
    }
}

use Category::*;

/// Built-in list of well-known tracking and advertising hosts. A host matches
/// when it equals an entry or is a subdomain of one.
const BLOCKLIST: &[(&str, Category)] = &[
    // Advertising networks and ad exchanges
    ("doubleclick.net", Ads),
    ("googlesyndication.com", Ads),
    ("googleadservices.com", Ads),
    ("adservice.google.com", Ads),
    ("2mdn.net", Ads),
    ("adnxs.com", Ads),
    ("adsrvr.org", Ads),
    ("advertising.com", Ads),
    ("amazon-adsystem.com", Ads),
    ("criteo.com", Ads),
    ("criteo.net", Ads),
    ("taboola.com", Ads),
    ("outbrain.com", Ads),
    ("pubmatic.com", Ads),
    ("rubiconproject.com", Ads),
    ("openx.net", Ads),
    ("casalemedia.com", Ads),
    ("indexww.com", Ads),
    ("moatads.com", Ads),
    ("adform.net", Ads),
    ("smartadserver.com", Ads),
    ("yieldmo.com", Ads),
    ("sharethrough.com", Ads),
    ("teads.tv", Ads),
    ("bidswitch.net", Ads),
    ("33across.com", Ads),
    ("contextweb.com", Ads),
    ("revcontent.com", Ads),
    ("mgid.com", Ads),
    ("zedo.com", Ads),
    ("serving-sys.com", Ads),
    ("adroll.com", Ads),
    ("adsafeprotected.com", Ads),
    ("doubleverify.com", Ads),
    ("adcolony.com", Ads),
    ("applovin.com", Ads),
    ("inmobi.com", Ads),
    ("popads.net", Ads),
    ("propellerads.com", Ads),
    ("exoclick.com", Ads),
    ("adsterra.com", Ads),
    ("trafficjunky.net", Ads),
    ("media.net", Ads),
    ("lijit.com", Ads),
    ("sovrn.com", Ads),
    ("gumgum.com", Ads),
    ("spotxchange.com", Ads),
    ("spotx.tv", Ads),
    ("springserve.com", Ads),
    ("tremorhub.com", Ads),
    ("undertone.com", Ads),
    ("yieldlab.net", Ads),
    ("adition.com", Ads),
    ("smaato.net", Ads),
    ("mathtag.com", Ads),
    ("turn.com", Ads),
    ("rlcdn.com", Ads),
    ("agkn.com", Ads),
    ("bluekai.com", Ads),
    ("krxd.net", Ads),
    ("exelator.com", Ads),
    ("eyeota.net", Ads),
    ("tapad.com", Ads),
    ("crwdcntrl.net", Ads),
    ("adsymptotic.com", Ads),
    ("ads.yahoo.com", Ads),
    ("ads.linkedin.com", Ads),
    ("ads-twitter.com", Ads),
    ("ads.tiktok.com", Ads),
    ("bat.bing.com", Ads),
    ("ads.pinterest.com", Ads),
    ("pagead2.googlesyndication.com", Ads),
    ("imasdk.googleapis.com", Ads),
    // Video advertising (pre-roll and mid-roll ads on video players)
    ("fwmrm.net", Ads),
    ("freewheel.tv", Ads),
    ("innovid.com", Ads),
    ("stickyadstv.com", Ads),
    ("aniview.com", Ads),
    ("connatix.com", Ads),
    ("primis.tech", Ads),
    ("vidoomy.com", Ads),
    ("dmxleo.com", Ads),
    ("vi.ai", Ads),
    ("jwpltx.com", Analytics),
    // Analytics and session recording
    ("google-analytics.com", Analytics),
    ("analytics.google.com", Analytics),
    ("googletagmanager.com", Analytics),
    ("googletagservices.com", Analytics),
    ("scorecardresearch.com", Analytics),
    ("quantserve.com", Analytics),
    ("quantcount.com", Analytics),
    ("hotjar.com", Analytics),
    ("hotjar.io", Analytics),
    ("mixpanel.com", Analytics),
    ("segment.io", Analytics),
    ("cdn.segment.com", Analytics),
    ("api.segment.io", Analytics),
    ("amplitude.com", Analytics),
    ("heapanalytics.com", Analytics),
    ("fullstory.com", Analytics),
    ("mouseflow.com", Analytics),
    ("crazyegg.com", Analytics),
    ("clarity.ms", Analytics),
    ("nr-data.net", Analytics),
    ("chartbeat.com", Analytics),
    ("chartbeat.net", Analytics),
    ("omtrdc.net", Analytics),
    ("demdex.net", Analytics),
    ("2o7.net", Analytics),
    ("everesttech.net", Analytics),
    ("kissmetrics.com", Analytics),
    ("statcounter.com", Analytics),
    ("mc.yandex.ru", Analytics),
    ("mc.yandex.com", Analytics),
    ("appsflyer.com", Analytics),
    ("adjust.com", Analytics),
    ("kochava.com", Analytics),
    ("branch.io", Analytics),
    ("comscore.com", Analytics),
    ("newrelic.com", Analytics),
    ("smartlook.com", Analytics),
    ("luckyorange.com", Analytics),
    ("inspectlet.com", Analytics),
    ("logrocket.io", Analytics),
    ("sessioncam.com", Analytics),
    ("decibelinsight.net", Analytics),
    ("contentsquare.net", Analytics),
    ("parsely.com", Analytics),
    ("parse.ly", Analytics),
    ("woopra.com", Analytics),
    ("clicky.com", Analytics),
    ("histats.com", Analytics),
    ("gemius.pl", Analytics),
    ("ioam.de", Analytics),
    // Social widgets and pixels
    ("connect.facebook.net", Social),
    ("pixel.facebook.com", Social),
    ("analytics.twitter.com", Social),
    ("snap.licdn.com", Social),
    ("px.ads.linkedin.com", Social),
    ("analytics.tiktok.com", Social),
    ("ct.pinterest.com", Social),
    ("analytics.pinterest.com", Social),
    ("sc-static.net", Social),
    ("tr.snapchat.com", Social),
    ("alb.reddit.com", Social),
    ("pixel.reddit.com", Social),
    ("addthis.com", Social),
    ("sharethis.com", Social),
    ("disqusads.com", Social),
    // Fingerprinting scripts
    ("fpjs.io", Fingerprinting),
    ("fpcdn.io", Fingerprinting),
    ("fingerprintjs.com", Fingerprinting),
    ("fingerprint.com", Fingerprinting),
    ("clientgear.com", Fingerprinting),
    ("deviceatlas.com", Fingerprinting),
    ("51.la", Fingerprinting),
    ("openfpcdn.io", Fingerprinting),
    // In-browser cryptominers
    ("coinhive.com", Cryptomining),
    ("coin-hive.com", Cryptomining),
    ("crypto-loot.com", Cryptomining),
    ("cryptoloot.pro", Cryptomining),
    ("jsecoin.com", Cryptomining),
    ("minero.cc", Cryptomining),
    ("webminepool.com", Cryptomining),
    ("coinimp.com", Cryptomining),
    ("monerominer.rocks", Cryptomining),
];

pub fn blocklist_len() -> usize {
    BLOCKLIST.iter().filter(|(d, _)| !d.contains('/')).count()
}

fn blocklist() -> &'static HashMap<&'static str, Category> {
    static MAP: OnceLock<HashMap<&'static str, Category>> = OnceLock::new();
    MAP.get_or_init(|| BLOCKLIST.iter().filter(|(d, _)| !d.contains('/')).copied().collect())
}

/// The host part of a URL, lower-cased, without port or brackets.
pub fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, r)| r)?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
    let host = if let Some(stripped) = authority.strip_prefix('[') {
        stripped.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if host.is_empty() {
        None
    } else {
        Some(host.trim_end_matches('.').to_ascii_lowercase())
    }
}

pub fn scheme_of(url: &str) -> &str {
    url.split_once("://").map(|(s, _)| s).unwrap_or("")
}

/// A rough "registrable domain": the last two labels, or three for common
/// two-part country suffixes such as co.uk.
pub fn base_domain(host: &str) -> String {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return host.to_string();
    }
    let labels: Vec<&str> = host.split('.').filter(|l| !l.is_empty()).collect();
    if labels.len() <= 2 {
        return labels.join(".");
    }
    let n = labels.len();
    let second = labels[n - 2];
    let tld = labels[n - 1];
    let two_part = tld.len() == 2
        && matches!(second, "co" | "com" | "org" | "net" | "gov" | "ac" | "edu" | "ne" | "or" | "go");
    let take = if two_part { 3 } else { 2 };
    labels[n - take..].join(".")
}

pub fn tracker_match(host: &str) -> Option<(&'static str, Category)> {
    let map = blocklist();
    let mut h = host;
    loop {
        if let Some((k, c)) = map.get_key_value(h) {
            return Some((k, *c));
        }
        match h.find('.') {
            Some(i) => h = &h[i + 1..],
            None => return None,
        }
    }
}

pub fn is_local_host(host: &str) -> bool {
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".lan") {
        return true;
    }
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Ok(std::net::IpAddr::V6(ip)) => ip.is_loopback(),
        Err(_) => false,
    }
}

/// Live privacy choices, readable from the page engine's callbacks.
pub struct Privacy {
    pub block_trackers: AtomicBool,
    pub https_only: AtomicBool,
    pub send_gpc: AtomicBool,
    pub trim_referrers: AtomicBool,
    pub block_popups: AtomicBool,
    pub blend_in: AtomicBool,
    pub block_video_ads: AtomicBool,
    perms: [AtomicU8; 6],
    shields_off: RwLock<HashSet<String>>,
    https_exceptions: RwLock<HashSet<String>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perm {
    Camera = 0,
    Microphone = 1,
    Location = 2,
    Notifications = 3,
    Clipboard = 4,
    Other = 5,
}

impl Perm {
    pub fn name(self) -> &'static str {
        match self {
            Perm::Camera => "camera",
            Perm::Microphone => "microphone",
            Perm::Location => "location",
            Perm::Notifications => "notifications",
            Perm::Clipboard => "clipboard",
            Perm::Other => "device access",
        }
    }
}

pub enum Decision {
    Allow { third_party: bool },
    Block { host: String, category: Category },
}

impl Privacy {
    pub fn new(s: &Settings) -> Privacy {
        let p = Privacy {
            block_trackers: AtomicBool::new(true),
            https_only: AtomicBool::new(true),
            send_gpc: AtomicBool::new(true),
            trim_referrers: AtomicBool::new(true),
            block_popups: AtomicBool::new(true),
            blend_in: AtomicBool::new(true),
            block_video_ads: AtomicBool::new(true),
            perms: Default::default(),
            shields_off: RwLock::new(HashSet::new()),
            https_exceptions: RwLock::new(HashSet::new()),
        };
        p.apply(s);
        p
    }

    pub fn apply(&self, s: &Settings) {
        self.block_trackers.store(s.block_trackers, Ordering::Relaxed);
        self.https_only.store(s.https_only, Ordering::Relaxed);
        self.send_gpc.store(s.send_gpc, Ordering::Relaxed);
        self.trim_referrers.store(s.trim_referrers, Ordering::Relaxed);
        self.block_popups.store(s.block_popups, Ordering::Relaxed);
        self.blend_in.store(s.blend_in_user_agent, Ordering::Relaxed);
        self.block_video_ads.store(s.block_video_ads, Ordering::Relaxed);
        let modes = [
            s.perm_camera,
            s.perm_microphone,
            s.perm_location,
            s.perm_notifications,
            s.perm_clipboard,
            s.perm_other,
        ];
        for (slot, m) in self.perms.iter().zip(modes) {
            slot.store(if m == PermMode::Block { 0 } else { 1 }, Ordering::Relaxed);
        }
        if let Ok(mut set) = self.shields_off.write() {
            *set = s.shields_off_sites.iter().cloned().collect();
        }
    }

    pub fn perm_blocked(&self, p: Perm) -> bool {
        self.perms[p as usize].load(Ordering::Relaxed) == 0
    }

    pub fn shields_up_for(&self, page_host: &str) -> bool {
        let base = base_domain(page_host);
        self.shields_off.read().map(|s| !s.contains(&base)).unwrap_or(true)
    }

    pub fn https_exception(&self, host: &str) -> bool {
        self.https_exceptions.read().map(|s| s.contains(host)).unwrap_or(false)
    }

    pub fn add_https_exception(&self, host: &str) {
        if let Ok(mut s) = self.https_exceptions.write() {
            s.insert(host.to_string());
        }
    }

    pub fn https_exceptions(&self) -> Vec<String> {
        let mut v: Vec<String> = self.https_exceptions.read().map(|s| s.iter().cloned().collect()).unwrap_or_default();
        v.sort();
        v
    }

    pub fn remove_https_exception(&self, host: &str) {
        if let Ok(mut s) = self.https_exceptions.write() {
            s.remove(host);
        }
    }

    /// Should this request leave the browser? `is_main_document` is true for
    /// the page the person asked for itself.
    pub fn decide(&self, request_url: &str, page_url: &str, is_main_document: bool) -> Decision {
        let Some(req_host) = host_of(request_url) else {
            return Decision::Allow { third_party: false };
        };
        let page_host = host_of(page_url).unwrap_or_default();
        let third_party = !page_host.is_empty() && base_domain(&req_host) != base_domain(&page_host);
        if is_main_document {
            return Decision::Allow { third_party };
        }
        // Video sites serve their ad requests from their own domain, so these are
        // matched by address even when they're first-party.
        if self.block_video_ads.load(Ordering::Relaxed) && self.shields_up_for(&page_host) {
            if let Some(label) = video_ad_rule(&req_host, request_url) {
                return Decision::Block { host: label, category: Category::Ads };
            }
        }
        if !third_party {
            return Decision::Allow { third_party };
        }
        if !self.block_trackers.load(Ordering::Relaxed) || !self.shields_up_for(&page_host) {
            return Decision::Allow { third_party };
        }
        match tracker_match(&req_host) {
            Some((_, category)) => Decision::Block { host: req_host, category },
            None => Decision::Allow { third_party },
        }
    }

    /// The HTTPS address to use instead of a plain-HTTP one, when HTTPS-only applies.
    pub fn upgrade(&self, url: &str) -> Option<String> {
        if !self.https_only.load(Ordering::Relaxed) || !url.starts_with("http://") {
            return None;
        }
        let host = host_of(url)?;
        if is_local_host(&host) || self.https_exception(&host) {
            return None;
        }
        Some(format!("https://{}", &url["http://".len()..]))
    }
}

/// Ad requests that video sites send to their own servers: (host suffix, path prefix).
const VIDEO_AD_PATHS: &[(&str, &str)] = &[
    ("youtube.com", "/api/stats/ads"),
    ("youtube.com", "/pagead/"),
    ("youtube.com", "/ptracking"),
    ("youtube.com", "/get_midroll_"),
    ("youtube.com", "/youtubei/v1/player/ad_break"),
    ("youtube.com", "/pcs/activeview"),
    ("youtube-nocookie.com", "/api/stats/ads"),
    ("youtube-nocookie.com", "/pagead/"),
    ("youtube-nocookie.com", "/ptracking"),
    ("youtube-nocookie.com", "/youtubei/v1/player/ad_break"),
];

/// A label for `url` when it's a known video-ad request, else None.
pub fn video_ad_rule(host: &str, url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, r)| r)?;
    let path = rest.find('/').map(|i| &rest[i..]).unwrap_or("/");
    let path = path.split(['?', '#']).next().unwrap_or(path);
    VIDEO_AD_PATHS
        .iter()
        .find(|(h, p)| (host == *h || host.ends_with(&format!(".{h}"))) && path.starts_with(p))
        .map(|(h, p)| format!("{h}{} (video ad)", p.trim_end_matches('/')))
}

/// Runs in YouTube pages and embedded players before YouTube's own code:
/// removes the ad schedule from the player's data, skips and mutes any ad
/// that still starts, closes overlay ads and hides sponsored tiles and banners.
pub fn video_ad_script() -> String {
    VIDEO_AD_SCRIPT.to_string()
}

const VIDEO_AD_SCRIPT: &str = r#"(() => {
  'use strict';
  if (!/(^|\.)(youtube\.com|youtube-nocookie\.com)$/.test(location.hostname)) return;
  const last = Object.create(null);
  const report = (kind) => {
    const now = Date.now();
    if (last[kind] && now - last[kind] < 4000) return;
    last[kind] = now;
    try { if (window.ipc && window.ipc.postMessage) window.ipc.postMessage('ad:' + kind); } catch (e) {}
  };

  // 1. Take the ads out of the player's data before the player reads it.
  const AD_KEYS = ['adPlacements', 'playerAds', 'adSlots', 'adBreakHeartbeatParams', 'adBreakParams'];
  const prune = (o, depth) => {
    let n = 0;
    if (!o || typeof o !== 'object' || depth > 2) return 0;
    for (const k of AD_KEYS) {
      if (Object.prototype.hasOwnProperty.call(o, k)) {
        const v = o[k];
        if (!(Array.isArray(v) && v.length === 0)) n++;
        try { delete o[k]; } catch (e) {}
      }
    }
    if (o.playerResponse) n += prune(o.playerResponse, depth + 1);
    if (Array.isArray(o)) for (const x of o) n += prune(x, depth + 1);
    return n;
  };
  const check = (v) => { try { if (prune(v, 0) > 0) report('removed'); } catch (e) {} return v; };

  const origParse = JSON.parse;
  JSON.parse = function (text, reviver) { return check(origParse.call(this, text, reviver)); };
  try { Object.defineProperty(JSON.parse, 'toString', { value: () => 'function parse() { [native code] }' }); } catch (e) {}

  const origJson = Response.prototype.json;
  Response.prototype.json = function () { return origJson.call(this).then(check); };

  for (const name of ['ytInitialPlayerResponse', 'playerResponse']) {
    try {
      let value;
      Object.defineProperty(window, name, {
        configurable: true,
        get() { return value; },
        set(v) { value = check(v); },
      });
    } catch (e) {}
  }

  // 2. Hide sponsored tiles, banners and ad slots.
  const CSS = `
    #masthead-ad, #player-ads, #panels > ytd-engagement-panel-section-list-renderer[target-id="engagement-panel-ads"],
    .ytp-ad-overlay-container, .ytp-ad-image-overlay, .ytp-ad-text-overlay, .ytp-featured-product,
    ytd-ad-slot-renderer, ytd-in-feed-ad-layout-renderer, ytd-promoted-sparkles-web-renderer,
    ytd-promoted-sparkles-text-search-renderer, ytd-display-ad-renderer, ytd-promoted-video-renderer,
    ytd-compact-promoted-video-renderer, ytd-banner-promo-renderer, ytd-video-masthead-ad-v3-renderer,
    ytd-primetime-promo-renderer, ytd-player-legacy-desktop-watch-ads-renderer, ytd-merch-shelf-renderer,
    ytd-search-pyv-renderer, ytd-action-companion-ad-renderer, ytd-companion-slot-renderer,
    ytd-rich-item-renderer:has(ytd-ad-slot-renderer), ytd-rich-section-renderer:has(ytd-statement-banner-renderer),
    ytm-promoted-sparkles-web-renderer, ytm-companion-ad-renderer, ad-slot-renderer,
    tp-yt-paper-dialog:has(ytd-enforcement-message-view-model) { display: none !important; }`;
  let styled = false;
  const ensureStyle = () => {
    if (styled || !document.documentElement) return;
    const s = document.createElement('style');
    s.textContent = CSS;
    (document.head || document.documentElement).appendChild(s);
    styled = true;
  };
  document.addEventListener('DOMContentLoaded', ensureStyle);

  // 3. If an ad still starts: skip it, or mute it and jump to its end.
  let inAd = false, savedMuted = false, savedRate = 1;
  const SKIP = '.ytp-skip-ad-button, .ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-ad-skip-button-slot button, .ytp-ad-skip-button-container button';
  const tick = () => {
    ensureStyle();
    if (window.top !== window && !/\/embed\//.test(location.pathname)) return;
    const player = document.querySelector('#movie_player, .html5-video-player');
    if (!player) return;
    const video = player.querySelector('video');
    const showing = player.classList.contains('ad-showing') || player.classList.contains('ad-interrupting');
    if (showing && video) {
      if (!inAd) {
        inAd = true;
        savedMuted = video.muted;
        savedRate = video.playbackRate || 1;
        report('skipped');
      }
      const skip = player.querySelector(SKIP);
      if (skip) skip.click();
      video.muted = true;
      if (isFinite(video.duration) && video.duration > 0 && video.currentTime < video.duration - 0.2) {
        video.currentTime = video.duration - 0.1;
      }
      if (video.playbackRate < 16) video.playbackRate = 16;
    } else if (inAd && video) {
      inAd = false;
      video.muted = savedMuted;
      video.playbackRate = savedRate;
    }
    const close = document.querySelector('.ytp-ad-overlay-close-button');
    if (close) { close.click(); report('overlay'); }
    // "Ad blockers are not allowed" walls: remove them and keep the video playing.
    const wall = document.querySelector('ytd-enforcement-message-view-model');
    if (wall) {
      const dialog = wall.closest('tp-yt-paper-dialog');
      (dialog || wall).remove();
      document.querySelectorAll('tp-yt-iron-overlay-backdrop').forEach((b) => b.remove());
      if (video && video.paused) video.play().catch(() => {});
      report('wall');
    }
  };
  setInterval(tick, 300);
})();"#;

/// Chromium brand list without the embedded-engine entry, so the browser looks
/// like plain Chromium rather than a rarer embedded build.
pub fn generic_brands(header: &str) -> String {
    header
        .split(',')
        .map(str::trim)
        .filter(|b| !b.contains("WebView2") && !b.contains("Edge"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Runs in every page and frame before the site's own scripts. Smooths the
/// values sites read to tell browsers apart and reports what was asked for.
pub fn fingerprint_script(seed: u32) -> String {
    FP_SCRIPT.replace("__SEED__", &seed.to_string())
}

const FP_SCRIPT: &str = r#"(() => {
  'use strict';
  const SEED = __SEED__ >>> 0;
  const told = Object.create(null);
  const report = (kind) => {
    if (told[kind]) return;
    told[kind] = 1;
    try { if (window.ipc && window.ipc.postMessage) window.ipc.postMessage('fp:' + kind); } catch (e) {}
  };
  const getter = (proto, name, fn) => {
    try {
      const d = Object.getOwnPropertyDescriptor(proto, name);
      if (!d || d.configurable) Object.defineProperty(proto, name, { get: fn, configurable: true, enumerable: true });
    } catch (e) {}
  };
  const wrap = (proto, name, make) => {
    try {
      const orig = proto[name];
      if (typeof orig !== 'function') return;
      const w = make(orig);
      Object.defineProperty(w, 'name', { value: name });
      Object.defineProperty(w, 'toString', { value: () => 'function ' + name + '() { [native code] }' });
      proto[name] = w;
    } catch (e) {}
  };
  // Same answer for the same pixel within a session, different between sessions
  const hash = (i) => {
    let x = Math.imul((i ^ SEED) >>> 0, 2654435761) >>> 0;
    x ^= x >>> 15; x = Math.imul(x, 2246822519) >>> 0; x ^= x >>> 13;
    return x >>> 0;
  };

  // Global Privacy Control and Do Not Track
  getter(Navigator.prototype, 'globalPrivacyControl', () => true);
  getter(Navigator.prototype, 'doNotTrack', () => '1');

  // Hardware details: report the most common values
  getter(Navigator.prototype, 'hardwareConcurrency', () => { report('hardware'); return 4; });
  getter(Navigator.prototype, 'deviceMemory', () => { report('hardware'); return 8; });

  // Battery level is a tracking vector
  if ('getBattery' in Navigator.prototype) {
    wrap(Navigator.prototype, 'getBattery', () => function () {
      report('battery');
      return Promise.reject(new DOMException('Blocked by LabSphere', 'NotAllowedError'));
    });
  }

  // Embedded-engine brand hidden from user-agent client hints
  try {
    if (self.NavigatorUAData) {
      const P = NavigatorUAData.prototype;
      const d = Object.getOwnPropertyDescriptor(P, 'brands');
      const clean = (list) => (list || []).filter(b => !/WebView2|Edge/.test(b.brand));
      if (d && d.get) Object.defineProperty(P, 'brands', { get: function () { return clean(d.get.call(this)); }, configurable: true });
      wrap(P, 'getHighEntropyValues', (orig) => function (hints) {
        report('client-hints');
        return orig.call(this, hints).then(v => {
          if (v.brands) v.brands = clean(v.brands);
          if (v.fullVersionList) v.fullVersionList = clean(v.fullVersionList);
          return v;
        });
      });
    }
  } catch (e) {}

  // Canvas: add invisible per-session noise so read-backs can't identify you
  const noise = (data) => {
    for (let i = 0; i < data.length; i += 4 * 61) {
      const h = hash(i);
      data[i] = data[i] ^ (h & 1);
      data[i + 1] = data[i + 1] ^ ((h >>> 1) & 1);
    }
  };
  wrap(CanvasRenderingContext2D.prototype, 'getImageData', (orig) => function (...a) {
    report('canvas');
    const img = orig.apply(this, a);
    try { noise(img.data); } catch (e) {}
    return img;
  });
  const viaCopy = (canvas) => {
    try {
      if (!canvas.width || !canvas.height) return null;
      const c = document.createElement('canvas');
      c.width = canvas.width; c.height = canvas.height;
      const x = c.getContext('2d');
      x.drawImage(canvas, 0, 0);
      const img = CanvasRenderingContext2D.prototype.getImageData.call(x, 0, 0, c.width, c.height);
      x.putImageData(img, 0, 0);
      return c;
    } catch (e) { return null; }
  };
  wrap(HTMLCanvasElement.prototype, 'toDataURL', (orig) => function (...a) {
    report('canvas');
    const c = viaCopy(this);
    return orig.apply(c || this, a);
  });
  wrap(HTMLCanvasElement.prototype, 'toBlob', (orig) => function (...a) {
    report('canvas');
    const c = viaCopy(this);
    return orig.apply(c || this, a);
  });

  // WebGL: hide the exact graphics card
  const glParam = (orig) => function (p) {
    if (p === 37445) { report('webgl'); return 'Google Inc.'; }
    if (p === 37446) { report('webgl'); return 'ANGLE (Generic GPU)'; }
    return orig.call(this, p);
  };
  if (self.WebGLRenderingContext) wrap(WebGLRenderingContext.prototype, 'getParameter', glParam);
  if (self.WebGL2RenderingContext) wrap(WebGL2RenderingContext.prototype, 'getParameter', glParam);

  // Audio: tiny noise in rendered samples
  if (self.AudioBuffer) {
    const seen = new WeakSet();
    wrap(AudioBuffer.prototype, 'getChannelData', (orig) => function (...a) {
      const d = orig.apply(this, a);
      if (!seen.has(d)) {
        seen.add(d);
        report('audio');
        for (let i = 0; i < d.length; i += 101) d[i] += ((hash(i) & 255) / 255 - 0.5) * 1e-7;
      }
      return d;
    });
  }

  // Screen: hide the taskbar size, which differs from machine to machine
  const sw = Object.getOwnPropertyDescriptor(Screen.prototype, 'width');
  const sh = Object.getOwnPropertyDescriptor(Screen.prototype, 'height');
  if (sw && sw.get) getter(Screen.prototype, 'availWidth', function () { return sw.get.call(this); });
  if (sh && sh.get) getter(Screen.prototype, 'availHeight', function () { return sh.get.call(this); });
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_and_domains() {
        assert_eq!(host_of("https://User@Sub.Example.co.uk:8080/x?y").as_deref(), Some("sub.example.co.uk"));
        assert_eq!(base_domain("sub.example.co.uk"), "example.co.uk");
        assert_eq!(base_domain("a.b.example.com"), "example.com");
        assert_eq!(tracker_match("www.google-analytics.com").map(|t| t.0), Some("google-analytics.com"));
        assert!(tracker_match("example.com").is_none());
    }

    #[test]
    fn decisions() {
        let p = Privacy::new(&Settings::default());
        assert!(matches!(
            p.decide("https://www.google-analytics.com/a.js", "https://news.example/", false),
            Decision::Block { .. }
        ));
        assert!(matches!(
            p.decide("https://cdn.example.com/a.js", "https://www.example.com/", false),
            Decision::Allow { third_party: false }
        ));
        assert_eq!(p.upgrade("http://example.com/a").as_deref(), Some("https://example.com/a"));
        assert!(p.upgrade("http://localhost:3000/").is_none());
        assert!(matches!(
            p.decide("https://www.youtube.com/api/stats/ads?ver=2", "https://www.youtube.com/watch?v=x", false),
            Decision::Block { .. }
        ));
        assert!(matches!(
            p.decide("https://www.youtube.com/youtubei/v1/player?key=1", "https://www.youtube.com/watch?v=x", false),
            Decision::Allow { .. }
        ));
        assert_eq!(
            generic_brands(r#""Chromium";v="157", "Microsoft Edge WebView2";v="157", "Not?A_Brand";v="99""#),
            r#""Chromium";v="157", "Not?A_Brand";v="99""#
        );
    }
}
