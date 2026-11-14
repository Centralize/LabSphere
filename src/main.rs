//! LabSphere: a Chromium-based web browser focused on privacy and security.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod pages;
mod settings;
mod shields;
mod ui;

use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut screenshot: Option<PathBuf> = None;
    let mut selftest: Option<app::SelfTest> = None;
    let mut open_url: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                screenshot = args.get(i + 1).map(PathBuf::from);
                i += 1;
            }
            "--selftest" => {
                let out = args.get(i + 1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("selftest.txt"));
                let url = args.get(i + 2).cloned().unwrap_or_else(|| "http://example.com/".into());
                let basic = args.get(i + 3).map(|s| s == "basic").unwrap_or(false);
                selftest = Some(app::SelfTest { out, url, started: None, injected: false, log: Vec::new(), basic });
                i += if basic { 3 } else { 2 };
            }
            a if !a.starts_with("--") => open_url = Some(a.to_string()),
            _ => {}
        }
        i += 1;
    }
    let background = screenshot.is_some() || selftest.is_some();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("LabSphere")
        .with_app_id("labsphere")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([760.0, 520.0])
        .with_icon(Arc::new(ui::window_icon()));
    if background {
        // Checks must never get in the person's way: no focus, and on Windows behind other windows.
        viewport = viewport.with_active(false);
        #[cfg(windows)]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }

    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    #[cfg(target_os = "macos")]
    if background {
        options.event_loop_builder = Some(Box::new(|builder| {
            use eframe::egui_winit::winit::platform::macos::EventLoopBuilderExtMacOS;
            builder.with_activate_ignoring_other_apps(false);
        }));
    }

    let cleanup: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    let cleanup_app = cleanup.clone();
    let result = eframe::run_native(
        "LabSphere",
        options,
        Box::new(move |cc| Ok(Box::new(app::LabSphere::new(cc, screenshot, open_url, selftest, cleanup_app)))),
    );

    // "Forget everything when LabSphere closes": remove this session's engine data.
    if let Some(dir) = cleanup.lock().ok().and_then(|mut d| d.take()) {
        for _ in 0..40 {
            if !dir.exists() || std::fs::remove_dir_all(&dir).is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    }
    result
}
