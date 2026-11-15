# LabSphere

**A Chromium-based web browser focused on privacy and security.**
Private by default. Secure by design.

LabSphere is a desktop browser for Windows. Web pages are rendered by Chromium
(through the Microsoft Edge WebView2 runtime that ships with Windows 11), and
everything around the page — tabs, address bar, Shields, settings — is
LabSphere's own interface. Every protection is switched on from the first
launch, and each one is explained in plain words in Settings.

---

## Contents

1. [Features at a glance](#features-at-a-glance)
2. [Requirements](#requirements)
3. [Building and running](#building-and-running)
4. [Using LabSphere](#using-labsphere)
5. [Keyboard shortcuts](#keyboard-shortcuts)
6. [How the protections work](#how-the-protections-work)
7. [Settings reference](#settings-reference)
8. [Where LabSphere keeps things](#where-labsphere-keeps-things)
9. [Command-line options](#command-line-options)
10. [Project layout](#project-layout)
11. [Known limitations](#known-limitations)
12. [Troubleshooting](#troubleshooting)

---

## Features at a glance

| Protection | What it does | Default |
|---|---|---|
| **Tracker & ad blocking** | Blocks 166 known advertising, video-advertising, analytics, social-tracking, fingerprinting and cryptomining services when they load on other sites | On |
| **Video ad blocking** | Removes ads from YouTube videos before they play, skips any that slip through, hides sponsored videos and banners, and blocks the ad networks other video sites use | On |
| **Built-in tracking prevention** | The engine's own tracker protection (Basic / Balanced / Strict) | Strict |
| **HTTPS-Only mode** | Upgrades every connection to encrypted HTTPS and asks before using an insecure one | On |
| **Fingerprint guard** | Adds invisible noise to canvas, audio and graphics read-backs, reports common hardware values, hides the battery level | On |
| **Global Privacy Control** | Tells every site not to sell or share your data (`Sec-GPC: 1` and `DNT: 1`) | On |
| **Referrer trimming** | Other sites only learn which site you came from, not the exact page | On |
| **Pop-up blocking** | Pages can only open new tabs when you click something | On |
| **WebRTC IP-leak protection** | Stops video-call technology revealing your local network address | On |
| **Blend into the crowd** | Presents LabSphere as standard Chromium (the most common browser) instead of a rarer embedded build | On |
| **Permission blocking** | Camera, microphone, location, notifications, clipboard reading and other device access are refused | Block |
| **Forget on close** | Cookies, logins, cache and site data only live until LabSphere closes | On |
| **No password or form saving** | Nothing you type into forms is stored | Off |
| **Private tabs** | Separate in-memory cookie jar, no history, data gone when the tab closes | — |
| **Forget everything now** | One click closes all tabs and erases cookies, site data, cache and history | — |
| **Private search** | Words typed in the address bar go to DuckDuckGo (or Brave Search, Startpage, Mojeek) | DuckDuckGo |

Also included: tabs, private tabs, saved sites, session history, zoom, printing,
full-screen video, a privacy dashboard on the new-tab page and a Shields panel
showing exactly what was blocked on the current site.

---

## Requirements

**To run**

- Windows 10 or Windows 11 (64-bit)
- Microsoft Edge WebView2 Runtime — already installed on Windows 11 and on
  most Windows 10 machines. If it's missing, LabSphere says so on the page area;
  install the "Evergreen" runtime from Microsoft and start LabSphere again.

**To build**

- Rust toolchain (stable, 2024 edition support — Rust 1.85 or newer) from <https://rustup.rs>
- The MSVC build tools that `rustup` asks for on Windows
- An internet connection the first time, to download dependencies

---

## Building and running

From the `app` folder:

```powershell
# Optimised build (recommended for everyday use, no console window)
cargo build --release
.\target\release\labsphere.exe

# Development build (faster to compile, shows a console window for messages)
cargo build
.\target\debug\labsphere.exe

# Run the unit tests (address parsing, tracker matching, HTTPS upgrades)
cargo test
```

The release program is a single file, `target\release\labsphere.exe`
(about 6 MB). Copy it anywhere; it needs no installer.

---

## Using LabSphere

### The window

```
┌───────────────────────────────────────────────────────────────────┐
│ [tab] [tab] [+]                                         tab strip │
│ ← → ⟳ ⌂ [🔒 address or search ................ ☆] 🛡 3  🕶 🔥 ≡   │
│ (info bar: blocked pop-ups, HTTPS warnings, notes)                │
├───────────────────────────────────────────────┬───────────────────┤
│                                               │ Shields / Menu    │
│                 web page                      │ side panel        │
│                                               │ (when open)       │
├───────────────────────────────────────────────┴───────────────────┤
│ link under mouse / status      3 trackers blocked here · HTTPS-Only │
└───────────────────────────────────────────────────────────────────┘
```

- **Back, Forward, Reload/Stop, Home** — the four buttons on the left.
- **Address bar** — type an address (`example.com`) or words to search. The
  icon on the left shows the connection: green lock = secure (HTTPS), amber
  open lock = not secure. Click it to open the Shields panel.
- **Star** — save the current site to the home page (`Ctrl+D`).
- **Shield button** — shows how many trackers were blocked on this page; click
  it for the Shields panel.
- **Hat-and-glasses button** — open a new private tab.
- **Flame button** — "Forget everything now" (asks to confirm first).
- **Menu button** — new tabs, history, settings, zoom, print, version info.
- **Status line** — the address of the link under the mouse, and which
  protections are on.

### The new-tab page

- A large private search box.
- Status chips for the main protections (click one to open Settings).
- A dashboard: trackers blocked, connections made secure, fingerprinting
  attempts disrupted, permission and pop-up requests blocked — for this session,
  with an all-time total in the corner of each card.
- **Saved sites** as tiles. Right-click a tile to open it in a private tab or
  remove it.

### The Shields panel

For the site in the current tab:

- The site name and whether the connection is secure (and whether LabSphere
  upgraded it to HTTPS).
- **Shields up / down for this site** — turn blocking off for a site that
  breaks, then back on later. Sites with Shields down are listed in Settings.
- Counts of trackers blocked (with the list of services and their category),
  video ads removed or skipped,
  fingerprinting attempts (which details the site tried to read), permission
  requests and pop-ups.
- **Forget this site's cookies.**
- This session's totals.

### Private tabs

Open one with the hat-and-glasses button or `Ctrl+Shift+N`. Private tabs have
a violet accent. They use a separate in-memory cookie jar, are never written to
history, and their site data disappears when the tab closes. All Shields stay
on. Your internet provider, employer and the sites themselves can still see
that you visited them.

### When a site has no HTTPS

With HTTPS-Only on, LabSphere tries the secure address first. If the site can't
offer it, an amber bar appears: **Go back**, or **Continue without security**
(that site is then allowed without HTTPS for the rest of the session; remove it
again in Settings → Connection security).

### Forget everything now

Flame button → **Erase now**. All tabs close and cookies, logins, site data,
cache and this session's history are erased. A green note confirms it.

---

## Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+T` | New tab |
| `Ctrl+Shift+N` | New private tab |
| `Ctrl+W` / `Ctrl+F4` | Close tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Ctrl+L` | Jump to the address bar |
| `Enter` (in address bar) | Go / search |
| `Esc` (in address bar) | Undo typing |
| `Ctrl+R` / `F5` | Reload |
| `Alt+←` / `Alt+→` | Back / forward |
| `Ctrl+D` | Save or unsave the site |
| `Ctrl+H` | History |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Zoom in / out / reset |
| Middle-click a tab | Close it |

The tab shortcuts (`Ctrl+T`, `Ctrl+Shift+N`, `Ctrl+W`, `Ctrl+Tab`, `Ctrl+L`)
work even while a web page has the keyboard. Inside pages, Chromium's own
page shortcuts (such as reload and back/forward) also work.

---

## How the protections work

**Tracker & ad blocking.** Every request a page makes passes LabSphere's filter
before it leaves the computer. A request is blocked when it goes to a *different*
site than the page you're on and its address matches the built-in list (a host
matches an entry or any subdomain of it). Visiting a listed site directly is
never blocked, so logging in to a service still works. Blocked requests get an
empty "403 Blocked by LabSphere Shields" answer.

**Video ad blocking.** Video sites such as YouTube serve their ads from their own
servers, so blocking by site name isn't enough. On YouTube (including embedded
players on other sites) LabSphere:

- removes the ad schedule from the player's data before the player reads it, so
  pre-roll and mid-roll ads never start;
- blocks YouTube's own ad and ad-measurement addresses (`/pagead/`, `/ptracking`,
  `/api/stats/ads`, `/get_midroll_`, ad-break requests);
- if an ad still starts, presses *Skip*, or mutes it and jumps to its end, then
  restores your volume and speed;
- closes overlay ads and hides sponsored videos, promoted search results,
  banners and merchandise shelves;
- removes "ad blockers are not allowed" pop-ups and keeps the video playing.

On other video sites the common video-ad networks (FreeWheel, Innovid, SpotX,
Connatix, Google's IMA video-ad SDK and others) are blocked by name. Each video
ad removed or skipped is counted in the Shields panel and on the home page.
Turning Shields down for YouTube turns all of this off for YouTube.

**Built-in tracking prevention.** The engine's own tracker protection runs
first. At *Strict* it stops many trackers before LabSphere's filter sees them —
those are blocked but not included in LabSphere's counts, so the real number is
higher than the dashboard shows.

**HTTPS-Only.** Plain `http://` page loads are cancelled and reloaded as
`https://`. Local addresses (`localhost`, `192.168.x.x`, `*.local`, …) are left
alone. If the secure address fails to connect, has certificate problems, or the
site keeps redirecting back to plain HTTP, the amber warning bar appears.

**Fingerprint guard.** A small script runs in every page and frame before the
site's own code:

- canvas images read back by the page get tiny invisible changes (stable within
  a session, different between sessions);
- audio processing output gets inaudible noise;
- the exact graphics card name is replaced by a generic one;
- processor count and memory are reported as common values (4 cores, 8 GB);
- the battery level is refused;
- the taskbar size is hidden from screen measurements.

Each kind of read is counted once per page in the Shields panel.

**Blending in.** The browser identifies itself like standard Chromium: the
user-agent string, the `Sec-CH-UA` brand headers and the matching JavaScript
values have the embedded-engine brand removed.

**Network hardening (engine switches).** LabSphere starts the engine with
hyperlink-auditing pings, Domain Reliability reporting, shopping and sign-in
prompts, autofill server lookups and advertising-related browser APIs turned
off, and (by default) WebRTC limited to proxied connections.

**Permissions.** Requests for camera, microphone, location, notifications,
clipboard reading, MIDI, motion sensors, installed fonts and window placement
are refused and reported in an info bar, unless you set them to *Ask*.

**Forget on close.** Each run gets a fresh, throw-away data folder for the
engine. When LabSphere closes, that folder is deleted; leftovers from a crash
are removed the next time LabSphere starts.

---

## Settings reference

Open with Menu → **Settings** (or type `labsphere://settings`).

**Shields** — Block trackers and ads · Block video ads *(new tabs)* · Fingerprint guard *(new tabs)* · Tell
sites not to sell or share my data · Trim cross-site referrers · Block pop-ups ·
Prevent WebRTC IP leaks *(after restart)* · Blend into the crowd *(after restart)* ·
Built-in tracking prevention level (Basic / Balanced / Strict).

**Connection security** — HTTPS-Only mode · Dangerous-site warnings (Microsoft
SmartScreen; off by default because it shares the sites you visit with
Microsoft) · list of sites allowed without HTTPS this session.

**Site permissions** — Camera · Microphone · Location · Notifications ·
Clipboard reading · Other device access — each *Block* or *Ask*.

**Your data** — Forget everything when LabSphere closes *(after restart)* ·
Offer to save passwords · Autofill addresses and forms · Block JavaScript
everywhere (open pages reload) · sites where Shields are down · Forget
everything now.

**Search** — DuckDuckGo · Brave Search · Startpage · Mojeek.

**About** — LabSphere version, the Chromium engine version, and all-time totals.

Changes apply immediately unless marked otherwise; a yellow note appears when a
restart is needed.

---

## Where LabSphere keeps things

| What | Where |
|---|---|
| Your choices, saved sites and all-time counters (no browsing data) | `%APPDATA%\LabSphere\settings.json` |
| Engine data while "Forget on close" is on (deleted on close) | `%LOCALAPPDATA%\LabSphere\Sessions\<run>` |
| Engine data while "Forget on close" is off (kept) | `%LOCALAPPDATA%\LabSphere\Profile` |
| History | Memory only — never written to disk |

To reset LabSphere completely, close it and delete both `LabSphere` folders.

---

## Command-line options

```text
labsphere.exe [address]
labsphere.exe --screenshot <file.png> [address]
labsphere.exe --selftest <report.txt> [url] [basic]
```

| Option | Purpose |
|---|---|
| `address` | Open this address or search on start. Also accepts `labsphere://home`, `labsphere://settings`, `labsphere://history`, `labsphere://private` (a private tab), `labsphere://menu` and `labsphere://shields` (open with that side panel). |
| `--screenshot <file.png>` | Start without taking focus (and behind other windows), draw about 120 frames, save a picture of the window to the file and exit. Web pages aren't loaded in this mode and nothing is saved to settings. Used by automated checks. |
| `--selftest <report.txt> [url] [basic]` | Developer check: open `url` (default `http://example.com/`) through the real engine, inject sample trackers, then write a report of the final address, HTTPS upgrade, blocked requests, fingerprint reads and any warning bar, and exit. `basic` lowers the built-in tracking prevention for that run so LabSphere's own blocker sees every tracker. Settings are not saved. |

---

## Project layout

```
app/
├── Cargo.toml        dependencies and build settings
├── README.md         this file
└── src/
    ├── main.rs       start-up, command-line options, window setup, cleanup on exit
    ├── app.rs        the browser window: tabs, toolbar, address bar, info bar,
    │                 Shields and Menu panels, status line, engine events
    ├── pages.rs      LabSphere's own pages: home dashboard, Settings, History
    ├── engine.rs     the Chromium (WebView2) page engine and the hooks that
    │                 connect it to Shields
    ├── shields.rs    tracker list, blocking decisions, HTTPS upgrades,
    │                 permission policy, fingerprint-guard script
    ├── settings.rs   saved choices, search engines, storage locations
    └── ui.rs         colours, drawn icons, the logo and custom controls
```

Built with Rust, [egui/eframe](https://github.com/emilk/egui) for the interface
and [wry](https://github.com/tauri-apps/wry) +
[webview2-com](https://github.com/wravery/webview2-rs) for the Chromium engine.

---

## Known limitations

- **Windows only.** The Chromium engine comes from Microsoft Edge WebView2,
  which exists only on Windows. Other systems show a message instead of pages.
- **Engine updates come from Microsoft.** LabSphere doesn't bundle its own
  Chromium; security updates to the engine arrive through Windows/Edge updates.
- **Blocked counts are a lower bound** when built-in tracking prevention is at
  Strict (see [How the protections work](#how-the-protections-work)).
- **Fingerprint guard and some switches apply to new tabs or after a restart**,
  as noted next to each setting.
- **No extensions, sync, password manager or download manager of its own.**
  Downloads use the engine's standard download panel and go to your Downloads
  folder.
- **History is session-only** and favicons aren't fetched (sites show a
  coloured letter instead), to avoid extra requests.
- The tracker list is built in; it doesn't update itself yet.
- **Video ad blocking is an ongoing contest.** YouTube changes how it delivers
  ads often; when it does, some ads may play (LabSphere then skips them) until
  LabSphere is updated. Ads that a streamer or creator speaks or edits into the
  video itself can't be removed.

---

## Troubleshooting

| Problem | What to do |
|---|---|
| "This page can't be shown" / engine not installed | Install the Microsoft Edge WebView2 Runtime (Evergreen) and restart LabSphere. |
| A YouTube video won't play or shows an ad-blocker warning | Reload the page. If it persists, turn Shields down for youtube.com in the Shields panel (this also turns video ad blocking off there). |
| A site doesn't work properly | Open the Shields panel and turn Shields down for that site, then reload. If it still fails, try turning off Fingerprint guard (applies to new tabs) or lowering tracking prevention to Balanced. |
| Amber "doesn't offer a secure connection" bar | The site has no working HTTPS. Choose **Continue without security** only if you trust the network you're on. |
| A site asks for the camera or microphone and nothing happens | Permissions are blocked by default. Set that permission to *Ask* in Settings → Site permissions. |
| I want to stay logged in between sessions | Turn off "Forget everything when LabSphere closes" in Settings → Your data and restart LabSphere. |
| Typing goes into the page instead of the address bar | Click the address bar or press `Ctrl+L`. |
