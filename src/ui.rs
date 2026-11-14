//! LabSphere's look: colours, drawn icons and small custom controls.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use std::f32::consts::{PI, TAU};

pub const BG: Color32 = Color32::from_rgb(14, 17, 28);
pub const STRIP: Color32 = Color32::from_rgb(11, 13, 22);
pub const BAR: Color32 = Color32::from_rgb(22, 27, 43);
pub const CARD: Color32 = Color32::from_rgb(24, 29, 46);
pub const CARD_HI: Color32 = Color32::from_rgb(31, 37, 58);
pub const FIELD: Color32 = Color32::from_rgb(13, 16, 27);
pub const LINE: Color32 = Color32::from_rgb(44, 51, 78);
pub const TEXT: Color32 = Color32::from_rgb(229, 233, 245);
pub const MUTED: Color32 = Color32::from_rgb(136, 145, 176);
pub const FAINT: Color32 = Color32::from_rgb(88, 96, 126);
pub const TEAL: Color32 = Color32::from_rgb(61, 219, 217);
pub const VIOLET: Color32 = Color32::from_rgb(139, 108, 255);
pub const PRIVATE: Color32 = Color32::from_rgb(176, 124, 255);
pub const GOOD: Color32 = Color32::from_rgb(74, 222, 128);
pub const WARN: Color32 = Color32::from_rgb(251, 191, 36);
pub const BAD: Color32 = Color32::from_rgb(248, 113, 113);

pub fn setup_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut(|style| {
        use egui::TextStyle::*;
        style.text_styles = [
            (Heading, FontId::new(22.0, FontFamily::Proportional)),
            (Body, FontId::new(14.5, FontFamily::Proportional)),
            (Button, FontId::new(14.0, FontFamily::Proportional)),
            (Small, FontId::new(12.0, FontFamily::Proportional)),
            (Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = BG;
        v.window_fill = CARD;
        v.extreme_bg_color = FIELD;
        v.faint_bg_color = CARD;
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = TEAL;
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(61, 219, 217, 70);
        v.selection.stroke = Stroke::new(1.0_f32, TEAL);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(10);
        v.window_stroke = Stroke::new(1.0_f32, LINE);
        let r = CornerRadius::same(8);
        v.widgets.noninteractive.bg_fill = CARD;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        v.widgets.noninteractive.corner_radius = r;
        v.widgets.inactive.bg_fill = CARD_HI;
        v.widgets.inactive.weak_bg_fill = CARD_HI;
        v.widgets.inactive.bg_stroke = Stroke::NONE;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        v.widgets.inactive.corner_radius = r;
        v.widgets.hovered.bg_fill = Color32::from_rgb(40, 47, 72);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(40, 47, 72);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 80, 115));
        v.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT);
        v.widgets.hovered.corner_radius = r;
        v.widgets.active.bg_fill = Color32::from_rgb(48, 57, 88);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(48, 57, 88);
        v.widgets.active.bg_stroke = Stroke::new(1.0_f32, TEAL);
        v.widgets.active.fg_stroke = Stroke::new(1.5_f32, TEXT);
        v.widgets.active.corner_radius = r;
        v.widgets.open = v.widgets.active;
    });
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

pub fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A stable pleasant colour for a site's monogram.
pub fn site_color(key: &str) -> Color32 {
    let mut h: u32 = 2166136261;
    for b in key.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    let palette = [
        Color32::from_rgb(61, 219, 217),
        Color32::from_rgb(139, 108, 255),
        Color32::from_rgb(96, 165, 250),
        Color32::from_rgb(244, 114, 182),
        Color32::from_rgb(251, 146, 60),
        Color32::from_rgb(74, 222, 128),
        Color32::from_rgb(250, 204, 21),
        Color32::from_rgb(167, 139, 250),
    ];
    palette[(h % palette.len() as u32) as usize]
}

// ---------------------------------------------------------------- icons

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Back,
    Forward,
    Reload,
    Stop,
    Home,
    Lock,
    Unlock,
    Shield,
    ShieldOff,
    Private,
    Burn,
    Menu,
    Star,
    StarFilled,
    Close,
    Plus,
    Gear,
    Search,
    Clock,
    Globe,
    Eye,
    Bolt,
    Check,
}

fn line(p: &Painter, pts: Vec<Pos2>, s: Stroke) {
    p.add(Shape::line(pts, s));
}

pub fn draw_icon(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.5;
    let w = (r * 0.17).max(1.4);
    let s = Stroke::new(w, color);
    match icon {
        Icon::Back | Icon::Forward => {
            let d = if icon == Icon::Back { -1.0 } else { 1.0 };
            let tip = c + vec2(d * r * 0.62, 0.0);
            line(p, vec![c - vec2(d * r * 0.62, 0.0), tip], s);
            line(p, vec![tip + vec2(-d * r * 0.5, -r * 0.5), tip, tip + vec2(-d * r * 0.5, r * 0.5)], s);
        }
        Icon::Reload => {
            let rr = r * 0.6;
            let pts: Vec<Pos2> = (0..=28)
                .map(|i| {
                    let a = -PI * 0.35 + (i as f32 / 28.0) * TAU * 0.82;
                    c + vec2(a.cos(), a.sin()) * rr
                })
                .collect();
            let end = *pts.last().unwrap();
            line(p, pts, s);
            let a = -PI * 0.35 + TAU * 0.82;
            let tangent = vec2(-a.sin(), a.cos());
            let normal = vec2(a.cos(), a.sin());
            let head = end + tangent * r * 0.32;
            p.add(Shape::convex_polygon(
                vec![head, end + normal * r * 0.28, end - normal * r * 0.28],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Stop | Icon::Close => {
            let k = if icon == Icon::Close { 0.42 } else { 0.5 };
            line(p, vec![c + vec2(-r * k, -r * k), c + vec2(r * k, r * k)], s);
            line(p, vec![c + vec2(r * k, -r * k), c + vec2(-r * k, r * k)], s);
        }
        Icon::Plus => {
            line(p, vec![c - vec2(r * 0.55, 0.0), c + vec2(r * 0.55, 0.0)], s);
            line(p, vec![c - vec2(0.0, r * 0.55), c + vec2(0.0, r * 0.55)], s);
        }
        Icon::Home => {
            let pts = vec![
                c + vec2(-r * 0.62, -r * 0.05),
                c + vec2(0.0, -r * 0.66),
                c + vec2(r * 0.62, -r * 0.05),
                c + vec2(r * 0.46, -r * 0.05),
                c + vec2(r * 0.46, r * 0.62),
                c + vec2(-r * 0.46, r * 0.62),
                c + vec2(-r * 0.46, -r * 0.05),
            ];
            p.add(Shape::closed_line(pts, s));
            p.rect_filled(
                Rect::from_center_size(c + vec2(0.0, r * 0.36), vec2(r * 0.3, r * 0.52)),
                CornerRadius::same(1),
                color,
            );
        }
        Icon::Lock | Icon::Unlock => {
            let body = Rect::from_center_size(c + vec2(0.0, r * 0.25), vec2(r * 1.15, r * 0.85));
            p.rect_filled(body, CornerRadius::same((r * 0.18) as u8), color);
            let sr = r * 0.36;
            let base = body.top();
            let sc = pos2(c.x, base);
            let start = if icon == Icon::Unlock { PI * 1.0 } else { PI };
            let end = if icon == Icon::Unlock { PI * 1.75 } else { TAU };
            let mut pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let a = start + (end - start) * i as f32 / 16.0;
                    sc + vec2(a.cos(), a.sin() * 1.25) * sr
                })
                .collect();
            pts.insert(0, pos2(c.x - sr, base + 1.0));
            if icon == Icon::Lock {
                pts.push(pos2(c.x + sr, base + 1.0));
            }
            line(p, pts, Stroke::new(w * 1.05, color));
            p.circle_filled(body.center(), r * 0.11, BG);
        }
        Icon::Shield | Icon::ShieldOff => {
            let pts = shield_points(c, r * 0.82);
            if icon == Icon::Shield {
                p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
                let k = r * 0.82;
                line(
                    p,
                    vec![c + vec2(-k * 0.38, -k * 0.02), c + vec2(-k * 0.08, k * 0.3), c + vec2(k * 0.42, -k * 0.32)],
                    Stroke::new(w * 1.1, BG),
                );
            } else {
                p.add(Shape::closed_line(pts, s));
                line(p, vec![c + vec2(-r * 0.6, -r * 0.6), c + vec2(r * 0.6, r * 0.6)], s);
            }
        }
        Icon::Private => {
            // Hat
            let hat = vec![
                c + vec2(-r * 0.42, -r * 0.12),
                c + vec2(-r * 0.3, -r * 0.62),
                c + vec2(r * 0.3, -r * 0.62),
                c + vec2(r * 0.42, -r * 0.12),
            ];
            p.add(Shape::convex_polygon(hat, color, Stroke::NONE));
            line(p, vec![c + vec2(-r * 0.78, -r * 0.1), c + vec2(r * 0.78, -r * 0.1)], s);
            // Glasses
            let gl = c + vec2(-r * 0.36, r * 0.38);
            let gr = c + vec2(r * 0.36, r * 0.38);
            p.circle_stroke(gl, r * 0.24, s);
            p.circle_stroke(gr, r * 0.24, s);
            line(p, vec![gl + vec2(r * 0.24, 0.0), gr - vec2(r * 0.24, 0.0)], s);
        }
        Icon::Burn => {
            let outer = drop_points(c + vec2(0.0, r * 0.12), r * 0.56, r * 1.5);
            p.add(Shape::convex_polygon(outer, color, Stroke::NONE));
            let inner = drop_points(c + vec2(0.0, r * 0.38), r * 0.26, r * 0.7);
            p.add(Shape::convex_polygon(inner, BG, Stroke::NONE));
        }
        Icon::Menu => {
            for dy in [-0.45, 0.0, 0.45] {
                line(p, vec![c + vec2(-r * 0.6, r * dy), c + vec2(r * 0.6, r * dy)], s);
            }
        }
        Icon::Star | Icon::StarFilled => {
            let pts = star_points(c, r * 0.72, r * 0.3);
            if icon == Icon::StarFilled {
                for i in 0..10 {
                    let a = pts[i];
                    let b = pts[(i + 1) % 10];
                    p.add(Shape::convex_polygon(vec![c, a, b], color, Stroke::NONE));
                }
            } else {
                p.add(Shape::closed_line(pts, Stroke::new(w * 0.9, color)));
            }
        }
        Icon::Gear => {
            for i in 0..8 {
                let a = i as f32 / 8.0 * TAU;
                let d = vec2(a.cos(), a.sin());
                line(p, vec![c + d * r * 0.42, c + d * r * 0.72], Stroke::new(w * 1.6, color));
            }
            p.circle_stroke(c, r * 0.46, Stroke::new(w * 1.2, color));
            p.circle_filled(c, r * 0.16, color);
        }
        Icon::Search => {
            let cc = c + vec2(-r * 0.12, -r * 0.12);
            p.circle_stroke(cc, r * 0.42, s);
            line(p, vec![cc + vec2(r * 0.3, r * 0.3), c + vec2(r * 0.62, r * 0.62)], Stroke::new(w * 1.2, color));
        }
        Icon::Clock => {
            p.circle_stroke(c, r * 0.62, s);
            line(p, vec![c + vec2(0.0, -r * 0.38), c, c + vec2(r * 0.3, r * 0.16)], s);
        }
        Icon::Globe => {
            p.circle_stroke(c, r * 0.62, s);
            line(p, vec![c - vec2(r * 0.62, 0.0), c + vec2(r * 0.62, 0.0)], Stroke::new(w * 0.8, color));
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = -PI / 2.0 + PI * i as f32 / 20.0;
                    c + vec2(a.cos() * r * 0.28, a.sin() * r * 0.62)
                })
                .collect();
            line(p, pts.clone(), Stroke::new(w * 0.8, color));
            line(p, pts.iter().map(|q| pos2(2.0 * c.x - q.x, q.y)).collect(), Stroke::new(w * 0.8, color));
        }
        Icon::Eye => {
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = i as f32 / 24.0 * TAU;
                    c + vec2(a.cos() * r * 0.75, a.sin() * r * 0.42)
                })
                .collect();
            p.add(Shape::closed_line(pts, s));
            p.circle_filled(c, r * 0.22, color);
        }
        Icon::Bolt => {
            p.add(Shape::convex_polygon(
                vec![c + vec2(r * 0.15, -r * 0.75), c + vec2(-r * 0.45, r * 0.12), c + vec2(r * 0.02, r * 0.12)],
                color,
                Stroke::NONE,
            ));
            p.add(Shape::convex_polygon(
                vec![c + vec2(-r * 0.02, -r * 0.12), c + vec2(r * 0.45, -r * 0.12), c + vec2(-r * 0.15, r * 0.75)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Check => {
            line(p, vec![c + vec2(-r * 0.5, 0.0), c + vec2(-r * 0.12, r * 0.4), c + vec2(r * 0.55, -r * 0.4)], s);
        }
    }
}

fn shield_points(c: Pos2, r: f32) -> Vec<Pos2> {
    let mut pts = vec![c + vec2(-r * 0.78, -r * 0.72), c + vec2(0.0, -r * 0.98), c + vec2(r * 0.78, -r * 0.72)];
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        pts.push(c + vec2(r * 0.78 * (1.0 - t * t), -r * 0.72 + t * r * 1.74 - (t * PI).sin() * r * 0.0));
    }
    for i in (0..10).rev() {
        let t = i as f32 / 10.0;
        pts.push(c + vec2(-r * 0.78 * (1.0 - t * t), -r * 0.72 + t * r * 1.74));
    }
    pts
}

fn drop_points(bottom_center: Pos2, radius: f32, height: f32) -> Vec<Pos2> {
    let mut pts = Vec::new();
    // round bottom
    for i in 0..=14 {
        let a = PI * 0.05 + PI * 0.9 * i as f32 / 14.0;
        pts.push(bottom_center + vec2(a.cos() * radius, a.sin() * radius));
    }
    // taper to the tip
    pts.push(bottom_center + vec2(-radius * 0.55, -height * 0.55));
    pts.push(bottom_center + vec2(radius * 0.05, -height * 0.95));
    pts.push(bottom_center + vec2(radius * 0.7, -height * 0.45));
    pts.reverse();
    pts
}

fn star_points(c: Pos2, outer: f32, inner: f32) -> Vec<Pos2> {
    (0..10)
        .map(|i| {
            let a = -PI / 2.0 + i as f32 * PI / 5.0;
            let rr = if i % 2 == 0 { outer } else { inner };
            c + vec2(a.cos(), a.sin()) * rr
        })
        .collect()
}

/// The LabSphere mark: a lit sphere with an orbit ring.
pub fn draw_logo(p: &Painter, c: Pos2, r: f32) {
    let tilt = -0.42f32;
    let ring = |a: f32| {
        let x = a.cos() * r * 1.42;
        let y = a.sin() * r * 0.42;
        c + vec2(x * tilt.cos() - y * tilt.sin(), x * tilt.sin() + y * tilt.cos())
    };
    let ring_stroke = Stroke::new((r * 0.075).max(1.2), TEAL);
    // back half of the ring
    let back: Vec<Pos2> = (0..=40).map(|i| ring(PI + PI * i as f32 / 40.0)).collect();
    p.add(Shape::line(back, Stroke::new(ring_stroke.width, alpha(TEAL, 0.45))));
    // sphere shading
    p.circle_filled(c, r * 1.12, alpha(VIOLET, 0.10));
    let steps = 26;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let rr = r * (1.0 - t * 0.92);
        let off = vec2(-r * 0.32, -r * 0.36) * t;
        let col = if t < 0.55 {
            lerp_color(Color32::from_rgb(52, 32, 128), VIOLET, t / 0.55)
        } else {
            lerp_color(VIOLET, Color32::from_rgb(170, 245, 250), (t - 0.55) / 0.45)
        };
        p.circle_filled(c + off, rr, col);
    }
    // front half of the ring
    let front: Vec<Pos2> = (0..=40).map(|i| ring(PI * i as f32 / 40.0)).collect();
    p.add(Shape::line(front, ring_stroke));
    p.circle_filled(ring(PI * 0.18), r * 0.13, Color32::WHITE);
}

// ---------------------------------------------------------------- controls

/// A square button showing one icon.
pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, enabled: bool, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), if enabled { Sense::click() } else { Sense::hover() });
    let hovered = enabled && resp.hovered();
    if active {
        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(44, 52, 82));
    } else if hovered {
        ui.painter().rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(36, 43, 67));
    }
    let col = if !enabled {
        FAINT
    } else if hovered || active {
        TEXT
    } else {
        Color32::from_rgb(196, 202, 222)
    };
    draw_icon(ui.painter(), rect.shrink(size * 0.24), icon, col);
    if enabled && !tip.is_empty() {
        resp.on_hover_text(tip)
    } else {
        resp
    }
}

/// An on/off switch.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let size = vec2(40.0, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let bg = lerp_color(Color32::from_rgb(52, 60, 90), Color32::from_rgb(34, 166, 165), t);
    ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
    let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    ui.painter().circle_filled(pos2(x, rect.center().y), 8.0, Color32::WHITE);
    resp
}

/// A setting row: title, explanation and a switch on the right.
pub fn setting_row(ui: &mut Ui, title: &str, detail: &str, on: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.set_min_height(44.0);
        let w = ui.available_width() - 56.0;
        ui.allocate_ui_with_layout(vec2(w, 44.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(title).size(14.5).color(TEXT));
            if !detail.is_empty() {
                ui.label(egui::RichText::new(detail).size(12.5).color(MUTED));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if toggle(ui, on).changed() {
                changed = true;
            }
        });
    });
    changed
}

/// A filled rounded button with an accent colour.
pub fn pill_button(ui: &mut Ui, text: &str, fill: Color32, fg: Color32) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(14.0), fg);
    let size = vec2(galley.size().x + 28.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let f = if resp.hovered() { lerp_color(fill, Color32::WHITE, 0.12) } else { fill };
    ui.painter().rect_filled(rect, CornerRadius::same(16), f);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fg);
    resp
}

/// Ghost button: outline only.
pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(13.5), TEXT);
    let size = vec2(galley.size().x + 24.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = if resp.hovered() { Color32::from_rgb(40, 47, 72) } else { Color32::TRANSPARENT };
    ui.painter().rect_filled(rect, CornerRadius::same(15), fill);
    ui.painter().rect_stroke(rect, CornerRadius::same(15), Stroke::new(1.0_f32, LINE), StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, TEXT);
    resp
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(36, 42, 64)))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
}

pub fn monogram(p: &Painter, c: Pos2, r: f32, label: &str, key: &str) {
    let col = site_color(key);
    p.circle_filled(c, r, alpha(col, 0.22));
    p.circle_stroke(c, r, Stroke::new(1.0_f32, alpha(col, 0.55)));
    let letter: String = label.chars().find(|ch| ch.is_alphanumeric()).map(|ch| ch.to_uppercase().collect()).unwrap_or_else(|| "•".into());
    p.text(c, Align2::CENTER_CENTER, letter, FontId::proportional(r * 0.95), col);
}

/// Window icon drawn in software.
pub fn window_icon() -> egui::IconData {
    let n = 64usize;
    let mut rgba = vec![0u8; n * n * 4];
    let c = n as f32 / 2.0;
    let r = n as f32 * 0.34;
    for y in 0..n {
        for x in 0..n {
            let fx = x as f32 + 0.5 - c;
            let fy = y as f32 + 0.5 - c;
            let i = (y * n + x) * 4;
            let d = (fx * fx + fy * fy).sqrt();
            // orbit ring (tilted ellipse)
            let tilt = -0.42f32;
            let rx = fx * tilt.cos() + fy * tilt.sin();
            let ry = -fx * tilt.sin() + fy * tilt.cos();
            let e = ((rx / (r * 1.42)).powi(2) + (ry / (r * 0.42)).powi(2)).sqrt();
            let on_ring = (e - 1.0).abs() < 0.06;
            let front = ry > 0.0;
            let mut px = [0u8; 4];
            if d <= r {
                let lx = fx + r * 0.35;
                let ly = fy + r * 0.38;
                let t = 1.0 - ((lx * lx + ly * ly).sqrt() / (r * 1.6)).clamp(0.0, 1.0);
                let col = if t < 0.55 {
                    lerp_color(Color32::from_rgb(52, 32, 128), VIOLET, t / 0.55)
                } else {
                    lerp_color(VIOLET, Color32::from_rgb(170, 245, 250), (t - 0.55) / 0.45)
                };
                let edge = ((r - d) * 1.5).clamp(0.0, 1.0);
                px = [col.r(), col.g(), col.b(), (edge * 255.0) as u8];
            }
            if on_ring && (front || d > r) {
                px = [TEAL.r(), TEAL.g(), TEAL.b(), 255];
            }
            rgba[i..i + 4].copy_from_slice(&px);
        }
    }
    egui::IconData { rgba, width: n as u32, height: n as u32 }
}
