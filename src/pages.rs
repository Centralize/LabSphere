//! LabSphere's own pages: the new-tab home with the privacy dashboard,
//! Settings and this session's History.

use crate::app::{Internal, LabSphere};
use crate::settings::{PermMode, SearchEngine, Tracking};
use crate::shields;
use crate::ui::{self, Icon};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Key, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, pos2, vec2,
};

impl LabSphere {
    pub fn home_page(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let private = self.tab().private;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let col_w = (full.width() - 64.0).min(860.0);
            let left = full.center().x - col_w / 2.0;
            let top_pad = ((full.height() - 640.0) * 0.28).clamp(24.0, 120.0);
            ui.add_space(top_pad);
            let start = ui.cursor().min.y;
            ui.scope_builder(
                UiBuilder::new().max_rect(Rect::from_min_size(pos2(left, start), vec2(col_w, 2000.0))),
                |ui| {
                    ui.set_width(col_w);
                    // brand
                    ui.vertical_centered(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(120.0, 86.0), Sense::hover());
                        if private {
                            ui.painter().circle_filled(r.center(), 38.0, ui::alpha(ui::PRIVATE, 0.16));
                            ui::draw_icon(ui.painter(), Rect::from_center_size(r.center(), Vec2::splat(52.0)), Icon::Private, ui::PRIVATE);
                        } else {
                            ui::draw_logo(ui.painter(), r.center(), 30.0);
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new(if private { "Private tab" } else { "LabSphere" }).size(34.0).strong().color(ui::TEXT));
                        ui.label(
                            RichText::new(if private {
                                "Nothing from this tab is kept: no history, cookies or site data, even while LabSphere is open."
                            } else {
                                "Private by default. Secure by design."
                            })
                            .size(15.0)
                            .color(ui::MUTED),
                        );
                    });
                    ui.add_space(22.0);
                    self.home_search(ui, col_w, private);
                    ui.add_space(14.0);
                    self.protection_chips(ui, col_w);
                    ui.add_space(26.0);
                    self.stat_cards(ui, col_w);
                    ui.add_space(28.0);
                    if !private {
                        self.saved_sites(ui, col_w);
                    } else {
                        self.private_notes(ui, col_w);
                    }
                    ui.add_space(40.0);
                },
            );
        });
    }

    fn home_search(&mut self, ui: &mut Ui, w: f32, private: bool) {
        let h = 54.0;
        let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect.translate(vec2(0.0, 3.0)), CornerRadius::same(27), Color32::from_rgba_unmultiplied(0, 0, 0, 60));
        p.rect_filled(rect, CornerRadius::same(27), ui::FIELD);
        let id = ui.id().with("home_search");
        let focused = ui.memory(|m| m.has_focus(id));
        let accent = if private { ui::PRIVATE } else { ui::TEAL };
        p.rect_stroke(
            rect,
            CornerRadius::same(27),
            Stroke::new(if focused { 1.5_f32 } else { 1.0 }, if focused { accent } else { ui::LINE }),
            StrokeKind::Inside,
        );
        ui::draw_icon(p, Rect::from_center_size(pos2(rect.left() + 28.0, rect.center().y), Vec2::splat(20.0)), Icon::Search, ui::MUTED);
        let badge = format!("{} · private search", self.settings.search.name());
        let galley = p.layout_no_wrap(badge.clone(), FontId::proportional(12.0), ui::MUTED);
        let bw = galley.size().x + 22.0;
        let brect = Rect::from_center_size(pos2(rect.right() - 14.0 - bw / 2.0, rect.center().y), vec2(bw, 26.0));
        p.rect_filled(brect, CornerRadius::same(13), Color32::from_rgb(28, 34, 54));
        p.galley(brect.center() - galley.size() / 2.0, galley, ui::MUTED);

        let text_rect = Rect::from_min_max(pos2(rect.left() + 50.0, rect.top() + 6.0), pos2(brect.left() - 10.0, rect.bottom() - 6.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(text_rect).layout(Layout::left_to_right(Align::Center)));
        let resp = egui::TextEdit::singleline(&mut self.home_query)
            .id(id)
            .frame(false)
            .hint_text(RichText::new("Search privately or type a web address").color(ui::FAINT).size(16.0))
            .font(FontId::proportional(16.5))
            .vertical_align(Align::Center)
            .desired_width(text_rect.width())
            .show(&mut child)
            .response;
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let q = std::mem::take(&mut self.home_query);
            if !q.trim().is_empty() {
                self.go(&q);
            }
        }
    }

    fn protection_chips(&mut self, ui: &mut Ui, w: f32) {
        let items = [
            (self.settings.block_trackers, "Tracker & ad blocking"),
            (self.settings.https_only, "HTTPS-Only"),
            (self.settings.fingerprint_guard, "Fingerprint guard"),
            (self.settings.send_gpc, "Global Privacy Control"),
            (self.settings.forget_on_close, "Forget on close"),
        ];
        let font = FontId::proportional(12.5);
        let widths: Vec<f32> = items
            .iter()
            .map(|(_, l)| ui.painter().layout_no_wrap(l.to_string(), font.clone(), ui::TEXT).size().x + 34.0)
            .collect();
        let gap = 8.0;
        let total: f32 = widths.iter().sum::<f32>() + gap * (items.len() as f32 - 1.0);
        let (row, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
        let mut x = row.center().x - total / 2.0;
        let mut open_settings = false;
        for ((on, label), cw) in items.iter().zip(widths) {
            let r = Rect::from_min_size(pos2(x, row.top()), vec2(cw, 28.0));
            let resp = ui.interact(r, ui.id().with(("chip", *label)), Sense::click());
            let p = ui.painter();
            let fill = if resp.hovered() { Color32::from_rgb(32, 38, 60) } else { Color32::from_rgb(22, 27, 43) };
            p.rect_filled(r, CornerRadius::same(14), fill);
            p.rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0_f32, Color32::from_rgb(40, 47, 72)), StrokeKind::Inside);
            p.circle_filled(pos2(r.left() + 14.0, r.center().y), 4.0, if *on { ui::GOOD } else { ui::FAINT });
            p.text(pos2(r.left() + 24.0, r.center().y), Align2::LEFT_CENTER, *label, font.clone(), if *on { ui::TEXT } else { ui::FAINT });
            if resp.on_hover_text(if *on { "On · click to change in Settings" } else { "Off · click to change in Settings" }).clicked() {
                open_settings = true;
            }
            x += cw + gap;
        }
        if open_settings {
            self.open_internal(Internal::Settings);
        }
    }

    fn stat_cards(&self, ui: &mut Ui, w: f32) {
        let s = &self.session;
        let t = &self.settings.totals;
        let cards = [
            (Icon::Eye, "Trackers & ads blocked", s.trackers, t.trackers, ui::TEAL),
            (Icon::Lock, "Connections made secure", s.upgrades, t.upgrades, ui::GOOD),
            (Icon::Globe, "Fingerprinting attempts disrupted", s.fingerprinting, t.fingerprinting, ui::VIOLET),
            (Icon::Shield, "Permission & pop-up requests blocked", s.permissions + s.popups, t.permissions + t.popups, ui::WARN),
        ];
        let gap = 12.0;
        let cw = (w - gap * 3.0) / 4.0;
        let (row, _) = ui.allocate_exact_size(vec2(w, 132.0), Sense::hover());
        let p = ui.painter();
        for (i, (icon, label, now, all, col)) in cards.iter().enumerate() {
            let r = Rect::from_min_size(pos2(row.left() + i as f32 * (cw + gap), row.top()), vec2(cw, 128.0));
            p.rect_filled(r, CornerRadius::same(14), ui::CARD);
            p.rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0_f32, Color32::from_rgb(36, 42, 64)), StrokeKind::Inside);
            // accent glow strip
            p.rect_filled(
                Rect::from_min_size(r.min + vec2(16.0, 0.0), vec2(36.0, 3.0)),
                CornerRadius::same(2),
                *col,
            );
            let ic = Rect::from_min_size(r.min + vec2(14.0, 16.0), Vec2::splat(22.0));
            p.circle_filled(ic.center(), 15.0, ui::alpha(*col, 0.14));
            ui::draw_icon(p, ic.shrink(2.0), *icon, *col);
            p.text(r.min + vec2(16.0, 46.0), Align2::LEFT_TOP, format_count(*now), FontId::proportional(30.0), ui::TEXT);
            let mut job = egui::text::LayoutJob::single_section(
                label.to_string(),
                egui::TextFormat { font_id: FontId::proportional(12.5), color: ui::MUTED, ..Default::default() },
            );
            job.wrap.max_width = cw - 30.0;
            job.wrap.max_rows = 2;
            let g = p.layout_job(job);
            p.galley(r.min + vec2(16.0, 84.0), g, ui::MUTED);
            p.text(
                pos2(r.right() - 14.0, r.top() + 22.0),
                Align2::RIGHT_CENTER,
                format!("{} all-time", format_count(*all)),
                FontId::proportional(11.0),
                ui::FAINT,
            );
        }
    }

    fn saved_sites(&mut self, ui: &mut Ui, w: f32) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Saved sites").size(15.0).strong().color(ui::TEXT));
            ui.label(RichText::new("· add more with the star in the address bar").size(12.5).color(ui::FAINT));
        });
        ui.add_space(8.0);
        let per_row = ((w + 12.0) / 134.0).floor().max(3.0) as usize;
        let tw = (w - 12.0 * (per_row as f32 - 1.0)) / per_row as f32;
        let th = 104.0;
        let mut open = None;
        let mut remove = None;
        let marks = self.settings.bookmarks.clone();
        for chunk in marks.chunks(per_row).enumerate() {
            let (row_i, items) = chunk;
            let (row, _) = ui.allocate_exact_size(vec2(w, th + 12.0), Sense::hover());
            for (j, b) in items.iter().enumerate() {
                let idx = row_i * per_row + j;
                let r = Rect::from_min_size(pos2(row.left() + j as f32 * (tw + 12.0), row.top()), vec2(tw, th));
                let resp = ui.interact(r, ui.id().with(("site", idx)), Sense::click());
                let p = ui.painter();
                let host = shields::host_of(&b.url).unwrap_or_default();
                p.rect_filled(r, CornerRadius::same(14), if resp.hovered() { ui::CARD_HI } else { ui::CARD });
                if resp.hovered() {
                    p.rect_stroke(r, CornerRadius::same(14), Stroke::new(1.0_f32, ui::alpha(ui::site_color(&host), 0.5)), StrokeKind::Inside);
                }
                ui::monogram(p, pos2(r.center().x, r.top() + 38.0), 20.0, &b.title, &host);
                let mut job = egui::text::LayoutJob::single_section(
                    b.title.clone(),
                    egui::TextFormat { font_id: FontId::proportional(13.0), color: ui::TEXT, ..Default::default() },
                );
                job.wrap = egui::text::TextWrapping { max_width: tw - 16.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
                job.halign = Align::Center;
                let g = p.layout_job(job);
                p.galley(pos2(r.center().x, r.top() + 70.0), g, ui::TEXT);
                let host_short = host.trim_start_matches("www.").to_string();
                let mut job = egui::text::LayoutJob::single_section(
                    host_short,
                    egui::TextFormat { font_id: FontId::proportional(11.0), color: ui::FAINT, ..Default::default() },
                );
                job.wrap = egui::text::TextWrapping { max_width: tw - 16.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
                job.halign = Align::Center;
                let g = p.layout_job(job);
                p.galley(pos2(r.center().x, r.top() + 87.0), g, ui::FAINT);
                let resp = resp.on_hover_text(&b.url);
                if resp.clicked() {
                    open = Some(b.url.clone());
                }
                resp.context_menu(|ui| {
                    if ui.button("Open in private tab").clicked() {
                        remove = Some((idx, true));
                        ui.close();
                    }
                    if ui.button("Remove from saved sites").clicked() {
                        remove = Some((idx, false));
                        ui.close();
                    }
                });
            }
        }
        if marks.is_empty() {
            ui.label(RichText::new("No saved sites yet.").color(ui::FAINT));
        }
        if let Some(url) = open {
            self.load_in_active(url);
        }
        if let Some((idx, private)) = remove {
            if private {
                let url = self.settings.bookmarks[idx].url.clone();
                self.open_in_new_tab(url, true);
            } else {
                self.settings.bookmarks.remove(idx);
                self.mark_dirty();
            }
        }
    }

    fn private_notes(&self, ui: &mut Ui, w: f32) {
        let _ = w;
        ui::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("How private tabs protect you").size(15.0).strong());
            ui.add_space(6.0);
            for line in [
                "A separate, in-memory cookie jar: sites can't link this tab to your other tabs.",
                "Nothing is written to history, and site data disappears when the tab closes.",
                "All LabSphere Shields stay on: trackers, fingerprinting and insecure connections are still blocked.",
                "Your internet provider, employer and the sites themselves can still see that you visited them.",
            ] {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    ui::draw_icon(ui.painter(), r, Icon::Check, ui::PRIVATE);
                    ui.label(RichText::new(line).size(13.5).color(ui::MUTED));
                });
            }
        });
    }

    // ------------------------------------------------------------ settings

    pub fn settings_page(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let col_w = (full.width() - 64.0).min(760.0);
            let left = full.center().x - col_w / 2.0;
            ui.add_space(28.0);
            let start = ui.cursor().min.y;
            ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(pos2(left, start), vec2(col_w, 20000.0))), |ui| {
                ui.set_width(col_w);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                    ui::draw_icon(ui.painter(), r, Icon::Gear, ui::TEAL);
                    ui.label(RichText::new("Settings").size(28.0).strong());
                });
                ui.label(RichText::new("LabSphere starts with the strongest protections on. Every choice is explained.").color(ui::MUTED));
                if self.restart_needed {
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(52, 44, 22))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new("Some changes take effect the next time you start LabSphere.").color(ui::WARN));
                        });
                }
                ui.add_space(16.0);
                self.settings_shields(ui);
                ui.add_space(14.0);
                self.settings_connections(ui);
                ui.add_space(14.0);
                self.settings_permissions(ui);
                ui.add_space(14.0);
                self.settings_data(ui);
                ui.add_space(14.0);
                self.settings_search(ui);
                ui.add_space(14.0);
                self.settings_about(ui);
                ui.add_space(40.0);
            });
        });
    }

    fn settings_shields(&mut self, ui: &mut Ui) {
        section(ui, Icon::Shield, ui::TEAL, "Shields", |ui| {
            let s = &mut self.settings;
            let mut changed = false;
            let mut restart = false;
            changed |= ui::setting_row(
                ui,
                "Block trackers and ads",
                &format!("Stops {} known tracking, advertising and cryptomining services from loading on the sites you visit.", shields::blocklist_len()),
                &mut s.block_trackers,
            );
            changed |= ui::setting_row(
                ui,
                "Fingerprint guard",
                "Adds invisible noise to canvas, audio and graphics read-backs and reports common hardware values, so sites can't single you out. Applies to new tabs.",
                &mut s.fingerprint_guard,
            );
            changed |= ui::setting_row(
                ui,
                "Tell sites not to sell or share my data",
                "Sends the Global Privacy Control and Do Not Track signals with every request.",
                &mut s.send_gpc,
            );
            changed |= ui::setting_row(
                ui,
                "Trim cross-site referrers",
                "Other sites only learn which site you came from, never the exact page.",
                &mut s.trim_referrers,
            );
            changed |= ui::setting_row(ui, "Block pop-ups", "Pages can only open new tabs when you click something.", &mut s.block_popups);
            if ui::setting_row(
                ui,
                "Prevent WebRTC IP leaks",
                "Stops video-call technology from revealing your local network address. Takes effect after a restart.",
                &mut s.block_webrtc_leak,
            ) {
                changed = true;
                restart = true;
            }
            if ui::setting_row(
                ui,
                "Blend into the crowd",
                "Presents LabSphere as a standard Chromium browser, the most common kind, so you're harder to tell apart.",
                &mut s.blend_in_user_agent,
            ) {
                changed = true;
                restart = true;
            }
            ui.add_space(6.0);
            ui.label(RichText::new("Built-in tracking prevention").size(14.5));
            ui.label(RichText::new(s.tracking.blurb()).size(12.5).color(ui::MUTED));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for level in Tracking::ALL {
                    if segment(ui, level.name(), s.tracking == level) {
                        s.tracking = level;
                        changed = true;
                    }
                }
            });
            if restart {
                self.restart_needed = true;
            }
            if changed {
                self.settings_changed();
            }
        });
    }

    fn settings_connections(&mut self, ui: &mut Ui) {
        section(ui, Icon::Lock, ui::GOOD, "Connection security", |ui| {
            let mut changed = false;
            changed |= ui::setting_row(
                ui,
                "HTTPS-Only mode",
                "Every connection is upgraded to encrypted HTTPS. If a site can't do it, LabSphere asks before using an insecure connection.",
                &mut self.settings.https_only,
            );
            changed |= ui::setting_row(
                ui,
                "Dangerous-site warnings (Microsoft SmartScreen)",
                "Checks addresses with Microsoft to warn about phishing and malware. Off by default because it shares the sites you visit with Microsoft.",
                &mut self.settings.smartscreen,
            );
            if changed {
                self.settings_changed();
            }
            let exceptions = self.privacy.https_exceptions();
            if !exceptions.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("Allowed without HTTPS this session").size(13.5).color(ui::MUTED));
                for h in exceptions {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        ui::draw_icon(ui.painter(), r, Icon::Unlock, ui::WARN);
                        ui.label(RichText::new(&h).size(13.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui::ghost_button(ui, "Remove").clicked() {
                                self.privacy.remove_https_exception(&h);
                            }
                        });
                    });
                }
            }
        });
    }

    fn settings_permissions(&mut self, ui: &mut Ui) {
        section(ui, Icon::Eye, ui::VIOLET, "Site permissions", |ui| {
            ui.label(
                RichText::new("Blocked requests are refused silently; “Ask” lets a site request access and you decide each time.")
                    .size(12.5)
                    .color(ui::MUTED),
            );
            ui.add_space(4.0);
            let s = &mut self.settings;
            let mut changed = false;
            for (label, detail, slot) in [
                ("Camera", "Video calls and photo capture", &mut s.perm_camera),
                ("Microphone", "Voice and video calls", &mut s.perm_microphone),
                ("Location", "Maps and local results", &mut s.perm_location),
                ("Notifications", "Pop-up messages from sites", &mut s.perm_notifications),
                ("Clipboard reading", "Reading what you copied", &mut s.perm_clipboard),
                ("Other device access", "MIDI, motion sensors, installed fonts and window placement", &mut s.perm_other),
            ] {
                ui.horizontal(|ui| {
                    ui.set_min_height(40.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).size(14.5));
                        ui.label(RichText::new(detail).size(12.0).color(ui::FAINT));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if segment(ui, "Ask", *slot == PermMode::Ask) {
                            *slot = PermMode::Ask;
                            changed = true;
                        }
                        if segment(ui, "Block", *slot == PermMode::Block) {
                            *slot = PermMode::Block;
                            changed = true;
                        }
                    });
                });
            }
            if changed {
                self.settings_changed();
            }
        });
    }

    fn settings_data(&mut self, ui: &mut Ui) {
        section(ui, Icon::Burn, ui::BAD, "Your data", |ui| {
            let mut changed = false;
            if ui::setting_row(
                ui,
                "Forget everything when LabSphere closes",
                "Cookies, logins, cache and site data live only until you close LabSphere. Takes effect after a restart.",
                &mut self.settings.forget_on_close,
            ) {
                changed = true;
                self.restart_needed = true;
            }
            changed |= ui::setting_row(
                ui,
                "Offer to save passwords",
                "Off: use a dedicated password manager instead.",
                &mut self.settings.save_passwords,
            );
            changed |= ui::setting_row(ui, "Autofill addresses and forms", "Off: nothing you type into forms is stored.", &mut self.settings.autofill);
            let mut js_blocked = self.settings.block_javascript;
            if ui::setting_row(
                ui,
                "Block JavaScript everywhere",
                "The safest setting, but many sites stop working. Open pages reload.",
                &mut js_blocked,
            ) {
                self.set_javascript_blocked(js_blocked);
            }
            if changed {
                self.settings_changed();
            }
            if !self.settings.shields_off_sites.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new("Sites where you turned Shields down").size(13.5).color(ui::MUTED));
                let mut restore = None;
                for (i, site) in self.settings.shields_off_sites.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        ui::draw_icon(ui.painter(), r, Icon::ShieldOff, ui::MUTED);
                        ui.label(RichText::new(site).size(13.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui::ghost_button(ui, "Shields up").clicked() {
                                restore = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = restore {
                    self.settings.shields_off_sites.remove(i);
                    self.settings_changed();
                }
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui::pill_button(ui, "Forget everything now", ui::BAD, ui::BG).clicked() {
                    self.confirm_burn = true;
                    self.drawer = Some(crate::app::Drawer::Menu);
                }
                ui.label(RichText::new("Closes all tabs and erases cookies, cache, site data and history.").size(12.5).color(ui::FAINT));
            });
        });
    }

    fn settings_search(&mut self, ui: &mut Ui) {
        section(ui, Icon::Search, ui::TEAL, "Search", |ui| {
            ui.label(RichText::new("Used when you type words instead of an address. All of these avoid profiling you.").size(12.5).color(ui::MUTED));
            ui.add_space(6.0);
            let mut chosen = None;
            for e in SearchEngine::ALL {
                let on = self.settings.search == e;
                let w = ui.available_width();
                let (r, resp) = ui.allocate_exact_size(vec2(w, 46.0), Sense::click());
                let p = ui.painter();
                p.rect_filled(r, CornerRadius::same(10), if on { Color32::from_rgb(22, 44, 52) } else if resp.hovered() { ui::CARD_HI } else { Color32::TRANSPARENT });
                if on {
                    p.rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0_f32, ui::alpha(ui::TEAL, 0.6)), StrokeKind::Inside);
                }
                let c = pos2(r.left() + 22.0, r.center().y);
                p.circle_stroke(c, 8.0, Stroke::new(1.5_f32, if on { ui::TEAL } else { ui::FAINT }));
                if on {
                    p.circle_filled(c, 4.5, ui::TEAL);
                }
                p.text(pos2(r.left() + 42.0, r.center().y - 8.0), Align2::LEFT_CENTER, e.name(), FontId::proportional(14.5), ui::TEXT);
                p.text(pos2(r.left() + 42.0, r.center().y + 9.0), Align2::LEFT_CENTER, e.blurb(), FontId::proportional(12.0), ui::FAINT);
                if resp.clicked() {
                    chosen = Some(e);
                }
            }
            if let Some(e) = chosen {
                self.settings.search = e;
                self.mark_dirty();
            }
        });
    }

    fn settings_about(&mut self, ui: &mut Ui) {
        section(ui, Icon::Globe, ui::MUTED, "About LabSphere", |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
                ui::draw_logo(ui.painter(), r.center(), 14.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("LabSphere {}", env!("CARGO_PKG_VERSION"))).size(15.0).strong());
                    let eng = match &self.engine_version {
                        Some(v) => format!("Page engine: Chromium via Microsoft Edge WebView2, version {v}"),
                        None => "Page engine: Chromium (Microsoft Edge WebView2) was not found on this computer".into(),
                    };
                    ui.label(RichText::new(eng).size(12.5).color(ui::MUTED));
                });
            });
            ui.add_space(6.0);
            let t = &self.settings.totals;
            ui.label(
                RichText::new(format!(
                    "Since you started using LabSphere: {} trackers blocked, {} connections secured, {} fingerprinting attempts disrupted.",
                    format_count(t.trackers),
                    format_count(t.upgrades),
                    format_count(t.fingerprinting)
                ))
                .size(12.5)
                .color(ui::FAINT),
            );
        });
    }

    // ------------------------------------------------------------ history

    pub fn history_page(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let mut open = None;
        let mut clear = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let col_w = (full.width() - 64.0).min(760.0);
            let left = full.center().x - col_w / 2.0;
            ui.add_space(28.0);
            let start = ui.cursor().min.y;
            ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_size(pos2(left, start), vec2(col_w, 40000.0))), |ui| {
                ui.set_width(col_w);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                    ui::draw_icon(ui.painter(), r, Icon::Clock, ui::TEAL);
                    ui.label(RichText::new("History").size(28.0).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.history.is_empty() && ui::ghost_button(ui, "Clear history").clicked() {
                            clear = true;
                        }
                    });
                });
                ui.label(
                    RichText::new(if self.settings.forget_on_close {
                        "Only this session. History is erased when LabSphere closes, and private tabs never appear here."
                    } else {
                        "This session's pages. Private tabs never appear here."
                    })
                    .color(ui::MUTED),
                );
                ui.add_space(14.0);
                if self.history.is_empty() {
                    ui::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("Nothing here yet.").color(ui::FAINT));
                    });
                }
                ui::card_frame().inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                    if self.history.is_empty() {
                        return;
                    }
                    ui.set_width(ui.available_width());
                    for (i, v) in self.history.iter().enumerate().rev() {
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 44.0), Sense::click());
                        let p = ui.painter();
                        if resp.hovered() {
                            p.rect_filled(r, CornerRadius::same(8), ui::CARD_HI);
                        }
                        let host = shields::host_of(&v.url).unwrap_or_default();
                        ui::monogram(p, pos2(r.left() + 22.0, r.center().y), 11.0, host.trim_start_matches("www."), &host);
                        let title = if v.title.is_empty() { host.clone() } else { v.title.clone() };
                        let mut job = egui::text::LayoutJob::single_section(
                            title,
                            egui::TextFormat { font_id: FontId::proportional(14.0), color: ui::TEXT, ..Default::default() },
                        );
                        job.wrap = egui::text::TextWrapping { max_width: w - 160.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
                        let g = p.layout_job(job);
                        p.galley(pos2(r.left() + 44.0, r.top() + 5.0), g, ui::TEXT);
                        let mut job = egui::text::LayoutJob::single_section(
                            v.url.clone(),
                            egui::TextFormat { font_id: FontId::proportional(11.5), color: ui::FAINT, ..Default::default() },
                        );
                        job.wrap = egui::text::TextWrapping { max_width: w - 160.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
                        let g = p.layout_job(job);
                        p.galley(pos2(r.left() + 44.0, r.top() + 25.0), g, ui::FAINT);
                        let mins = v.when.elapsed().as_secs() / 60;
                        let ago = if mins == 0 { "just now".to_string() } else if mins < 60 { format!("{mins} min ago") } else { format!("{} h ago", mins / 60) };
                        p.text(pos2(r.right() - 12.0, r.center().y), Align2::RIGHT_CENTER, ago, FontId::proportional(12.0), ui::FAINT);
                        if resp.clicked() {
                            open = Some(i);
                        }
                    }
                });
                ui.add_space(40.0);
            });
        });
        if clear {
            self.history.clear();
        }
        if let Some(i) = open {
            let url = self.history[i].url.clone();
            self.load_in_active(url);
        }
    }
}

fn section(ui: &mut Ui, icon: Icon, color: Color32, title: &str, body: impl FnOnce(&mut Ui)) {
    ui::card_frame().inner_margin(egui::Margin::symmetric(20, 16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 15.0, ui::alpha(color, 0.14));
            ui::draw_icon(ui.painter(), r.shrink(2.0), icon, color);
            ui.add_space(4.0);
            ui.label(RichText::new(title).size(17.0).strong());
        });
        ui.add_space(8.0);
        body(ui);
    });
}

fn segment(ui: &mut Ui, label: &str, on: bool) -> bool {
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(13.0), ui::TEXT);
    let (r, resp) = ui.allocate_exact_size(vec2(galley.size().x + 26.0, 30.0), Sense::click());
    let fill = if on {
        Color32::from_rgb(26, 92, 98)
    } else if resp.hovered() {
        ui::CARD_HI
    } else {
        Color32::from_rgb(28, 33, 52)
    };
    ui.painter().rect_filled(r, CornerRadius::same(15), fill);
    if on {
        ui.painter().rect_stroke(r, CornerRadius::same(15), Stroke::new(1.0_f32, ui::TEAL), StrokeKind::Inside);
    }
    ui.painter().galley(r.center() - galley.size() / 2.0, galley, if on { ui::TEXT } else { ui::MUTED });
    resp.clicked()
}

fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1e3)
    } else {
        let s = n.to_string();
        if s.len() > 3 { format!("{},{}", &s[..s.len() - 3], &s[s.len() - 3..]) } else { s }
    }
}
