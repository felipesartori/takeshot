//! Component kit shared by both skins: every widget keeps a fixed size and only changes colour on hover.

use crate::ui::i18n::{Msg, t as tr};
use crate::ui::icon;
pub use crate::ui::icon::Ico;
use crate::ui::style::{Weight, font};
use crate::ui::tokens::{SelStyle, ThemeId, Tokens};
use egui::{
    Align2, Color32, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};

const COLOR_TIME: f32 = 0.08;

/// 0.0 at rest, 1.0 hovered, eased over 80 ms; drives colour only, never size.
fn hover_mix(ui: &Ui, resp: &Response) -> f32 {
    ui.ctx()
        .animate_bool_with_time(resp.id.with("hover"), resp.hovered(), COLOR_TIME)
}

fn mix(a: Color32, b: Color32, f: f32) -> Color32 {
    a.lerp_to_gamma(b, f)
}

fn dim(c: Color32, ui: &Ui) -> Color32 {
    if ui.is_enabled() {
        c
    } else {
        c.gamma_multiply(0.45)
    }
}

fn text_size(ui: &Ui, s: &str, size: f32, w: Weight) -> egui::Vec2 {
    ui.painter()
        .layout_no_wrap(s.to_string(), font(size, w), Color32::WHITE)
        .size()
}

fn draw_text(
    p: &Painter,
    pos: egui::Pos2,
    anchor: Align2,
    s: &str,
    size: f32,
    w: Weight,
    c: Color32,
) {
    p.text(pos, anchor, s, font(size, w), c);
}

fn draw_icon(p: &Painter, center: egui::Pos2, i: Ico, size: f32, c: Color32) {
    p.text(
        center,
        Align2::CENTER_CENTER,
        icon::glyph(i),
        icon::font(size),
        c,
    );
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BtnKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

const KBD_PAD: f32 = 5.0;

fn kbd_width(ui: &Ui, keys: &str) -> f32 {
    text_size(ui, keys, 11.0, Weight::Mono).x + KBD_PAD * 2.0
}

fn paint_kbd(ui: &Ui, t: &Tokens, rect: Rect, keys: &str, on_primary: bool) {
    let p = ui.painter();
    if on_primary {
        let [r, g, b, _] = t.on_primary.to_array();
        p.rect_filled(
            rect,
            CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(r, g, b, 38),
        );
    } else {
        p.rect_filled(rect, CornerRadius::same(4), t.raised);
        p.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.0, t.border),
            StrokeKind::Inside,
        );
    }
    let c = if on_primary { t.on_primary } else { t.muted };
    draw_text(
        p,
        rect.center(),
        Align2::CENTER_CENTER,
        keys,
        11.0,
        Weight::Mono,
        dim(c, ui),
    );
}

/// Standalone shortcut chip, e.g. "Ctrl+C".
pub fn kbd(ui: &mut Ui, t: &Tokens, keys: &str, on_primary: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(kbd_width(ui, keys), 18.0), Sense::hover());
    paint_kbd(ui, t, rect, keys, on_primary);
    resp
}

/// Height 32, 12 px side padding, 7 px gaps: [icon] label [shortcut].
pub fn button(
    ui: &mut Ui,
    t: &Tokens,
    kind: BtnKind,
    icon: Option<Ico>,
    label: &str,
    kbd: Option<&str>,
) -> Response {
    let label_w = if label.is_empty() {
        0.0
    } else {
        text_size(ui, label, 13.0, Weight::Medium).x
    };
    let mut w = 24.0 + label_w;
    if icon.is_some() {
        w += 16.0 + if label.is_empty() { 0.0 } else { 7.0 };
    }
    if let Some(k) = kbd {
        w += 7.0 + kbd_width(ui, k);
    }
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let pressed = resp.is_pointer_button_down_on();
    let (rest, over, down, fg) = match kind {
        BtnKind::Primary => (t.primary, t.primary_hover, t.primary_press, t.on_primary),
        BtnKind::Secondary => (t.raised, t.hover, t.press, t.text),
        BtnKind::Ghost => (t.surf, t.raised, t.hover, t.text),
        BtnKind::Danger => (t.danger, t.danger, t.danger, Color32::WHITE),
    };
    let bg = if pressed { down } else { mix(rest, over, h) };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(t.radius), dim(bg, ui));
    if kind == BtnKind::Ghost {
        p.rect_stroke(
            rect,
            CornerRadius::same(t.radius),
            Stroke::new(1.0, t.border),
            StrokeKind::Inside,
        );
    }
    let fg = dim(fg, ui);
    let mut x = rect.left() + 12.0;
    if let Some(i) = icon {
        draw_icon(p, pos2(x + 8.0, rect.center().y), i, 16.0, fg);
        x += 16.0 + if label.is_empty() { 0.0 } else { 7.0 };
    }
    if !label.is_empty() {
        draw_text(
            p,
            pos2(x, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            13.0,
            Weight::Medium,
            fg,
        );
        x += label_w;
    }
    if let Some(k) = kbd {
        let kw = kbd_width(ui, k);
        let r = Rect::from_min_size(pos2(x + 7.0, rect.center().y - 9.0), vec2(kw, 18.0));
        paint_kbd(ui, t, r, k, kind == BtnKind::Primary);
    }
    resp
}

/// 32 × 32 square with a 17 px icon; active uses the theme's tint.
pub fn icon_button(ui: &mut Ui, t: &Tokens, icon: Ico, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if active {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(t.radius), bg);
    let fg = if active {
        t.accent
    } else {
        mix(t.muted, t.text, h)
    };
    draw_icon(p, rect.center(), icon, 17.0, dim(fg, ui));
    resp.on_hover_text(tip)
}

/// Paints a linear gradient (`stops` along `angle` degrees) in `rect` with rounded corners masked
/// by `bg`, the colour behind it.
pub fn paint_gradient(
    p: &Painter,
    rect: Rect,
    stops: &[(f32, [u8; 3])],
    angle: f32,
    rounding: f32,
    bg: Color32,
) {
    let n = 24;
    let a = angle.to_radians();
    let dir = vec2(a.sin(), -a.cos());
    let half = (rect.width() * dir.x.abs() + rect.height() * dir.y.abs()) / 2.0;
    let mut mesh = egui::Mesh::default();
    let t_of = |q: egui::Pos2| {
        ((q - rect.center()).dot(dir) / (2.0 * half.max(1e-3)) + 0.5).clamp(0.0, 1.0)
    };
    for i in 0..=n {
        for j in 0..=n {
            let q = pos2(
                rect.left() + rect.width() * j as f32 / n as f32,
                rect.top() + rect.height() * i as f32 / n as f32,
            );
            let c = crate::engine::presets::sample(stops, t_of(q));
            mesh.colored_vertex(q, Color32::from_rgb(c[0], c[1], c[2]));
        }
    }
    let w = n + 1;
    for i in 0..n {
        for j in 0..n {
            let a0 = (i * w + j) as u32;
            let b0 = a0 + 1;
            let c0 = a0 + w as u32;
            let d0 = c0 + 1;
            mesh.add_triangle(a0, b0, c0);
            mesh.add_triangle(b0, d0, c0);
        }
    }
    p.add(egui::Shape::mesh(mesh));
    if rounding > 0.0 {
        let r = rounding;
        for (corner, sx, sy) in [
            (rect.left_top(), 1.0, 1.0),
            (rect.right_top(), -1.0, 1.0),
            (rect.left_bottom(), 1.0, -1.0),
            (rect.right_bottom(), -1.0, -1.0),
        ] {
            let mut pts = vec![corner];
            for k in 0..=8 {
                let t = k as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                let cx = corner.x + sx * r;
                let cy = corner.y + sy * r;
                pts.push(pos2(cx - sx * r * t.cos(), cy - sy * r * t.sin()));
            }
            p.add(egui::Shape::convex_polygon(pts, bg, Stroke::NONE));
        }
    }
}

/// What a background swatch shows: a gradient (stops and angle). The solid colour has its own
/// colour picker instead.
#[derive(Clone, Copy, Debug)]
pub enum Swatch {
    Gradient(&'static [(f32, [u8; 3])], f32),
}

/// A 44 × 36 background choice: the fill inset by 3 px, a 2 px primary ring when selected, a
/// 1 px ring on hover, all inside its own area.
pub fn swatch(ui: &mut Ui, t: &Tokens, fill: Swatch, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 36.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let body = rect.shrink(3.0);
    let Swatch::Gradient(stops, angle) = fill;
    paint_gradient(p, body, stops, angle, 6.0, t.surf);
    let ring = if selected {
        Stroke::new(2.0, t.primary)
    } else {
        Stroke::new(1.0, mix(Color32::TRANSPARENT, t.muted, h))
    };
    p.rect_stroke(
        rect.shrink(1.0),
        CornerRadius::same(8),
        ring,
        StrokeKind::Inside,
    );
    resp
}

/// A collapsible section's header: chevron and title over the full width; a click toggles `open`.
pub fn section_header(ui: &mut Ui, t: &Tokens, title: &str, open: &mut bool) -> Response {
    let w = ui.available_width().max(120.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    if resp.clicked() {
        *open = !*open;
    }
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let fg = mix(t.text, t.accent, h * 0.5);
    let chevron = if *open {
        Ico::ChevronDown
    } else {
        Ico::ChevronRight
    };
    draw_icon(
        p,
        pos2(rect.left() + 8.0, rect.center().y),
        chevron,
        14.0,
        t.muted,
    );
    draw_text(
        p,
        pos2(rect.left() + 22.0, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        13.0,
        Weight::Medium,
        dim(fg, ui),
    );
    resp
}

/// Nine 28 × 20 cells, three by three, each with a dot at its anchor; returns the cell clicked.
pub fn anchor_grid(ui: &mut Ui, t: &Tokens, selected: Option<usize>) -> Option<usize> {
    let (rect, _) = ui.allocate_exact_size(
        vec2(3.0 * 28.0 + 2.0 * 4.0, 3.0 * 20.0 + 2.0 * 4.0),
        Sense::hover(),
    );
    let mut chosen = None;
    for i in 0..9 {
        let (col, row) = ((i % 3) as f32, (i / 3) as f32);
        let cell = Rect::from_min_size(rect.min + vec2(col * 32.0, row * 24.0), vec2(28.0, 20.0));
        let r = ui.interact(cell, ui.scope_id().with(("anchor", i)), Sense::click());
        let h = hover_mix(ui, &r);
        let on = selected == Some(i);
        let p = ui.painter();
        p.rect_filled(
            cell,
            CornerRadius::same(4),
            if on {
                t.tint
            } else {
                mix(t.input, t.raised, h)
            },
        );
        p.rect_stroke(
            cell,
            CornerRadius::same(4),
            Stroke::new(1.0, if on { t.primary } else { t.border }),
            StrokeKind::Inside,
        );
        let dot = pos2(
            cell.left() + 6.0 + col * (cell.width() - 12.0) / 2.0,
            cell.top() + 5.0 + row * (cell.height() - 10.0) / 2.0,
        );
        p.circle_filled(dot, 2.5, if on { t.accent } else { t.muted });
        if r.clicked() {
            chosen = Some(i);
        }
    }
    chosen
}

/// A settings row: label and a short hint on the left, the control on the right, over the full
/// width. The control is laid out first; label and hint are cut with "…" where it begins. Its
/// height follows the hint, never the hover.
pub fn setting_row(
    ui: &mut Ui,
    t: &Tokens,
    label: &str,
    hint: Option<&str>,
    control: impl FnOnce(&mut Ui),
) -> Response {
    let w = ui.available_width().max(240.0);
    let h = if hint.is_some() { 52.0 } else { 40.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let left = rect.left() + 14.0;
    let used = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(left, rect.top()),
                    pos2(rect.right() - 14.0, rect.bottom()),
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            control,
        )
        .response
        .rect;
    let room = (used.left() - 16.0 - left).max(0.0);
    let line = |text: &str, size: f32, weight: Weight, color: Color32| {
        let mut job =
            egui::text::LayoutJob::simple_singleline(text.to_string(), font(size, weight), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room);
        ui.painter().layout_job(job)
    };
    let top = if hint.is_some() {
        rect.top() + 9.0
    } else {
        rect.center().y - 8.0
    };
    let color = dim(t.text, ui);
    let g = line(label, 13.0, Weight::Medium, color);
    ui.painter().galley(pos2(left, top), g, color);
    if let Some(hint) = hint {
        let g = line(hint, 12.0, Weight::Regular, t.muted);
        ui.painter()
            .galley(pos2(left, rect.top() + 28.0), g, t.muted);
    }
    resp
}

/// A theme to choose: a small picture of the main window in that theme's colours (in the current
/// light or dark), and its name. Selected draws a 2 px primary ring; hover only colours the border.
pub fn theme_card(ui: &mut Ui, t: &Tokens, theme: ThemeId, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(148.0, 112.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let other = crate::ui::tokens::tokens(theme, t.dark);
    let pic = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(rect.width() - 16.0, 70.0));
    p.rect_filled(pic, CornerRadius::same(6), other.shell);
    let panel = Rect::from_min_max(pic.min + vec2(28.0, 14.0), pic.max - vec2(6.0, 6.0));
    p.rect_filled(panel, CornerRadius::same(4), other.surf);
    p.rect_filled(
        Rect::from_min_size(pic.min + vec2(6.0, 14.0), vec2(16.0, 50.0)),
        CornerRadius::same(3),
        other.raised,
    );
    p.rect_filled(
        Rect::from_min_size(panel.min + vec2(8.0, 8.0), vec2(panel.width() * 0.45, 7.0)),
        CornerRadius::same(3),
        other.primary,
    );
    p.rect_filled(
        Rect::from_min_size(pic.min + vec2(6.0, 4.0), vec2(26.0, 5.0)),
        CornerRadius::same(2),
        other.accent,
    );
    let name = tr(match theme {
        ThemeId::TakeUi => Msg::ThemeTakeUi,
        ThemeId::Coral => Msg::ThemeCoral,
    });
    draw_text(
        p,
        pos2(rect.left() + 10.0, pic.bottom() + 17.0),
        Align2::LEFT_CENTER,
        name,
        13.0,
        Weight::Medium,
        dim(t.text, ui),
    );
    let ring = if selected {
        Stroke::new(2.0, t.primary)
    } else {
        Stroke::new(1.0, mix(t.border, t.muted, h))
    };
    p.rect_stroke(
        rect.shrink(1.0),
        CornerRadius::same(8),
        ring,
        StrokeKind::Inside,
    );
    resp
}

/// A gallery filter: label and count in a pill; the active one uses the theme's tint.
pub fn filter_pill(ui: &mut Ui, t: &Tokens, label: &str, count: usize, active: bool) -> Response {
    let n = count.to_string();
    let lw = text_size(ui, label, 13.0, Weight::Medium).x;
    let nw = text_size(ui, &n, 11.0, Weight::Regular).x;
    let (rect, resp) =
        ui.allocate_exact_size(vec2(12.0 + lw + 6.0 + nw + 12.0, 28.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if active {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(14), bg);
    let fg = if active {
        t.accent
    } else {
        mix(t.muted, t.text, h)
    };
    let left = pos2(rect.left() + 12.0, rect.center().y);
    draw_text(
        p,
        left,
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Medium,
        dim(fg, ui),
    );
    let count_at = pos2(rect.right() - 12.0, rect.center().y);
    let nc = if active {
        t.accent.gamma_multiply(0.75)
    } else {
        t.faint
    };
    draw_text(
        p,
        count_at,
        Align2::RIGHT_CENTER,
        &n,
        11.0,
        Weight::Regular,
        dim(nc, ui),
    );
    resp
}

/// The picture of a capture card.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CardThumb {
    Loading,
    /// The thumbnail texture and its size in pixels.
    Ready(egui::TextureId, egui::Vec2),
    /// The file is not a picture the app can read.
    Failed,
}

/// What a capture card's hover buttons asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardAction {
    Copy,
    Edit,
    Folder,
    Trash,
}

pub struct CardResponse {
    pub resp: Response,
    pub action: Option<CardAction>,
}

/// Height of the name and size lines under a card's picture.
pub const CARD_TEXT_H: f32 = 44.0;

/// The hover buttons over the top right of a card's picture, right to left: trash, folder, edit,
/// copy (so they read copy, edit, folder, trash).
pub fn card_action_rects(card: Rect) -> [(CardAction, Rect); 4] {
    let s = 24.0;
    let at = |i: f32| {
        Rect::from_min_size(
            pos2(card.right() - 10.0 - s - i * (s + 2.0), card.top() + 10.0),
            vec2(s, s),
        )
    };
    [
        (CardAction::Trash, at(0.0)),
        (CardAction::Folder, at(1.0)),
        (CardAction::Edit, at(2.0)),
        (CardAction::Copy, at(3.0)),
    ]
}

/// A capture in the gallery: picture, name and size. Hover lifts the background and shows copy,
/// edit, show in folder and trash over the picture; selected draws a 2 px primary ring. The size
/// is the caller's (the grid's column width) and never depends on hover.
pub fn capture_card(
    ui: &mut Ui,
    t: &Tokens,
    size: egui::Vec2,
    thumb: CardThumb,
    name: &str,
    meta: &str,
    selected: bool,
) -> CardResponse {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter().clone();
    let radius = CornerRadius::same(t.radius);
    p.rect_filled(rect, radius, mix(Color32::TRANSPARENT, t.raised, h));
    let pic = Rect::from_min_max(
        rect.min + vec2(6.0, 6.0),
        pos2(rect.right() - 6.0, rect.bottom() - CARD_TEXT_H),
    );
    p.rect_filled(pic, CornerRadius::same(6), t.input);
    match thumb {
        CardThumb::Ready(id, px) if px.x > 0.0 && px.y > 0.0 => {
            let k = (pic.width() / px.x).min(pic.height() / px.y);
            let r = Rect::from_center_size(pic.center(), px * k);
            p.image(
                id,
                r,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        CardThumb::Failed => draw_icon(&p, pic.center(), Ico::Images, 22.0, t.faint),
        _ => {}
    }
    let width = rect.width() - 16.0;
    let line = |s: &str, size: f32, w: Weight, c: Color32| {
        let mut job = egui::text::LayoutJob::simple_singleline(s.to_string(), font(size, w), c);
        job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        ui.painter().layout_job(job)
    };
    let name_g = line(name, 12.0, Weight::Medium, dim(t.text, ui));
    p.galley(pos2(rect.left() + 8.0, pic.bottom() + 8.0), name_g, t.text);
    let meta_g = line(meta, 11.0, Weight::Regular, dim(t.faint, ui));
    p.galley(
        pos2(rect.left() + 8.0, pic.bottom() + 25.0),
        meta_g,
        t.faint,
    );
    if selected {
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(2.0, t.primary),
            StrokeKind::Inside,
        );
    }
    let mut action = None;
    if resp.contains_pointer() {
        let rects = card_action_rects(rect);
        let strip = rects
            .iter()
            .fold(Rect::NOTHING, |a, (_, r)| a.union(*r))
            .expand(2.0);
        p.rect_filled(strip, CornerRadius::same(7), t.shell.gamma_multiply(0.92));
        for (a, r) in rects {
            let b = ui.interact(r, resp.id.with(("card-action", a as u8)), Sense::click());
            let bh = hover_mix(ui, &b);
            p.rect_filled(
                r,
                CornerRadius::same(5),
                mix(Color32::TRANSPARENT, t.hover, bh),
            );
            let (icon, tip) = match a {
                CardAction::Copy => (Ico::Copy, Msg::Copy),
                CardAction::Edit => (Ico::Edit, Msg::EditInEditor),
                CardAction::Folder => (Ico::FolderOpen, Msg::ShowInFolder),
                CardAction::Trash => (Ico::Trash, Msg::Delete),
            };
            draw_icon(&p, r.center(), icon, 14.0, mix(t.muted, t.text, bh));
            if b.on_hover_text(tr(tip)).clicked() {
                action = Some(a);
            }
        }
    }
    CardResponse { resp, action }
}

/// A window button of the theme's title bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

/// 40 × 32 window button. Hover fills `hover`; Close fills `danger` with a white glyph. With the
/// window maximized, Maximize shows the restore glyph.
pub fn window_control(ui: &mut Ui, t: &Tokens, c: WindowControl, maximized: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 32.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let (icon, tip, hover_bg, hover_fg) = match c {
        WindowControl::Minimize => (Ico::Minimize, Msg::MinimizeWindow, t.hover, t.text),
        WindowControl::Maximize if maximized => (Ico::Copy, Msg::RestoreWindow, t.hover, t.text),
        WindowControl::Maximize => (Ico::Square, Msg::MaximizeWindow, t.hover, t.text),
        WindowControl::Close => (Ico::Close, Msg::CloseWord, t.danger, Color32::WHITE),
    };
    p.rect_filled(
        rect,
        CornerRadius::same(t.radius),
        mix(Color32::TRANSPARENT, hover_bg, h),
    );
    draw_icon(
        p,
        rect.center(),
        icon,
        15.0,
        dim(mix(t.muted, hover_fg, h), ui),
    );
    resp.on_hover_text(tr(tip))
}

/// 34 × 34 tool. `filled_active` paints the active tool in primary (Coral UI) instead of tint (TakeUI).
/// `None` draws the numbered-step tool: a circle with "1".
pub fn tool_button(
    ui: &mut Ui,
    t: &Tokens,
    icon: Option<Ico>,
    active: bool,
    tip: &str,
    filled_active: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let (bg, fg) = match (active, filled_active) {
        (true, true) => (t.primary, t.on_primary),
        (true, false) => (t.tint, t.accent),
        (false, _) => (
            mix(Color32::TRANSPARENT, t.raised, h),
            mix(t.muted, t.text, h),
        ),
    };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(t.radius), bg);
    let fg = dim(fg, ui);
    match icon {
        Some(i) => draw_icon(p, rect.center(), i, 18.0, fg),
        None => {
            p.circle_stroke(rect.center(), 9.0, Stroke::new(1.6, fg));
            draw_text(
                p,
                rect.center(),
                Align2::CENTER_CENTER,
                "1",
                10.0,
                Weight::Bold,
                fg,
            );
        }
    }
    resp.on_hover_text(tip)
}

/// 36 × 20 switch; the knob jumps, only colours ease.
pub fn switch(ui: &mut Ui, t: &Tokens, on: &mut bool) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(36.0, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(10),
        dim(if *on { t.primary } else { t.track }, ui),
    );
    let knob_x = if *on {
        rect.right() - 10.0
    } else {
        rect.left() + 10.0
    };
    let knob = if *on && t.theme == ThemeId::Coral {
        t.on_primary
    } else {
        Color32::WHITE
    };
    p.circle_filled(pos2(knob_x, rect.center().y), 8.0, knob);
    resp
}

/// 16 px box followed by a 13 px label.
pub fn checkbox(ui: &mut Ui, t: &Tokens, on: &mut bool, label: &str) -> Response {
    let lw = text_size(ui, label, 13.0, Weight::Regular).x;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(16.0 + 8.0 + lw, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let p = ui.painter();
    let b = Rect::from_center_size(pos2(rect.left() + 8.0, rect.center().y), vec2(16.0, 16.0));
    if *on {
        p.rect_filled(b, CornerRadius::same(4), t.primary);
        draw_icon(p, b.center(), Ico::Check, 12.0, t.on_primary);
    } else {
        p.rect_filled(b, CornerRadius::same(4), t.surf);
        p.rect_stroke(
            b,
            CornerRadius::same(4),
            Stroke::new(1.0, t.faint),
            StrokeKind::Inside,
        );
    }
    draw_text(
        p,
        pos2(b.right() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Regular,
        dim(t.text, ui),
    );
    resp
}

/// Pill-shaped choice between a few values; the chosen one sits on `seg_sel`.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    t: &Tokens,
    value: &mut T,
    items: &[(T, &str)],
) -> Response {
    let widths: Vec<f32> = items
        .iter()
        .map(|(_, s)| text_size(ui, s, 12.0, Weight::Medium).x + 20.0)
        .collect();
    let total = widths.iter().sum::<f32>() + 2.0 * (items.len() as f32 - 1.0).max(0.0) + 4.0;
    let (rect, mut resp) = ui.allocate_exact_size(vec2(total, 28.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(t.radius), t.raised);
    let mut x = rect.left() + 2.0;
    for ((v, s), w) in items.iter().zip(widths) {
        let r = Rect::from_min_size(pos2(x, rect.top() + 2.0), vec2(w, 24.0));
        let item = ui.interact(r, resp.id.with(s), Sense::click());
        if item.clicked() && *value != *v {
            *value = *v;
            resp.mark_changed();
        }
        let sel = *value == *v;
        let h = hover_mix(ui, &item);
        let p = ui.painter();
        if sel {
            p.rect_filled(r, CornerRadius::same(6), t.seg_sel);
        }
        let fg = if sel { t.text } else { mix(t.muted, t.text, h) };
        draw_text(
            p,
            r.center(),
            Align2::CENTER_CENTER,
            s,
            12.0,
            Weight::Medium,
            dim(fg, ui),
        );
        x += w + 2.0;
    }
    resp
}

/// Label (84 px), track, and a value box such as "7 %".
pub fn slider_row(
    ui: &mut Ui,
    t: &Tokens,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) -> Response {
    let full_w = ui.available_width().max(240.0);
    let (rect, mut resp) = ui.allocate_exact_size(vec2(full_w, 26.0), Sense::hover());
    let p = ui.painter();
    draw_text(
        p,
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Regular,
        dim(t.text, ui),
    );
    let box_r = Rect::from_min_size(pos2(rect.right() - 58.0, rect.top()), vec2(58.0, 26.0));
    let track = Rect::from_min_max(
        pos2(rect.left() + 96.0, rect.center().y - 8.0),
        pos2(box_r.left() - 12.0, rect.center().y + 8.0),
    );
    let drag = ui.interact(track, resp.id.with("track"), Sense::click_and_drag());
    let (lo, hi) = (*range.start(), *range.end());
    if let Some(pos) = drag.interact_pointer_pos()
        && (drag.dragged() || drag.clicked())
    {
        let f = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let nv = lo + f * (hi - lo);
        if nv != *value {
            *value = nv;
            resp.mark_changed();
        }
    }
    let f = ((*value - lo) / (hi - lo)).clamp(0.0, 1.0);
    let line_h = if t.theme == ThemeId::Coral { 2.0 } else { 4.0 };
    let line = Rect::from_center_size(track.center(), vec2(track.width(), line_h));
    p.rect_filled(line, CornerRadius::same(2), t.track);
    let filled = Rect::from_min_max(
        line.min,
        pos2(line.left() + line.width() * f, line.bottom()),
    );
    p.rect_filled(filled, CornerRadius::same(2), dim(t.primary, ui));
    let knob = pos2(line.left() + line.width() * f, track.center().y);
    if t.theme == ThemeId::Coral {
        p.circle_filled(knob, 6.0, dim(t.primary, ui));
    } else {
        p.circle_filled(knob, 7.0, Color32::WHITE);
        p.circle_stroke(knob, 7.0, Stroke::new(2.0, dim(t.primary, ui)));
    }
    p.rect_filled(box_r, CornerRadius::same(6), t.raised);
    let shown = format!("{:.0} {suffix}", *value);
    draw_text(
        p,
        pos2(box_r.right() - 8.0, box_r.center().y),
        Align2::RIGHT_CENTER,
        &shown,
        12.0,
        Weight::Medium,
        dim(t.text, ui),
    );
    resp
}

/// 32 px tall single-line field with an optional leading icon; focus adds a 2 px primary ring outside.
pub fn text_field(
    ui: &mut Ui,
    t: &Tokens,
    text: &mut String,
    hint: &str,
    icon: Option<Ico>,
    width: f32,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    let p = ui.painter().clone();
    p.rect_filled(rect, CornerRadius::same(t.radius), t.input);
    p.rect_stroke(
        rect,
        CornerRadius::same(t.radius),
        Stroke::new(1.0, t.border),
        StrokeKind::Inside,
    );
    let mut inner = rect.shrink2(vec2(10.0, 0.0));
    if let Some(i) = icon {
        draw_icon(
            &p,
            pos2(inner.left() + 7.5, rect.center().y),
            i,
            15.0,
            t.faint,
        );
        inner.min.x += 23.0;
    }
    let edit = egui::TextEdit::singleline(text)
        .hint_text(egui::RichText::new(hint).color(t.faint))
        .frame(egui::Frame::NONE)
        // `put` stretches the edit to the field height; centre the text, hint and caret in it.
        .align(Align2::LEFT_CENTER)
        .font(font(13.0, Weight::Regular))
        .text_color(t.text)
        .desired_width(inner.width());
    let resp = ui.put(inner, edit);
    if resp.has_focus() {
        p.rect_stroke(
            rect,
            CornerRadius::same(t.radius),
            Stroke::new(2.0, t.primary),
            StrokeKind::Outside,
        );
    }
    resp
}

/// A modal question: title, a short explanation, optional extra content (a "Remember my choice"
/// box) and a row of buttons. Returns the value of the button pressed this frame; Esc or a click
/// outside the dialog answers with `cancel`.
#[allow(clippy::too_many_arguments)]
pub fn confirm_dialog<T: Copy>(
    ctx: &egui::Context,
    t: &Tokens,
    id: &str,
    title: &str,
    body: &str,
    extra: &mut dyn FnMut(&mut Ui),
    buttons: &[(&str, BtnKind, T)],
    cancel: T,
) -> Option<T> {
    let mut chosen = None;
    let modal = egui::Modal::new(egui::Id::unique(id))
        .frame(
            egui::Frame::new()
                .fill(t.surf)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(380.0);
            ui.label(
                egui::RichText::new(title)
                    .font(font(15.0, Weight::Bold))
                    .color(t.text),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(body)
                    .font(font(13.0, Weight::Regular))
                    .color(t.muted),
            );
            ui.add_space(12.0);
            extra(ui);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for (label, kind, value) in buttons {
                    if button(ui, t, *kind, None, label, None).clicked() {
                        chosen = Some(*value);
                    }
                }
            });
        });
    if chosen.is_none() && modal.should_close() {
        chosen = Some(cancel);
    }
    chosen
}

/// Drop-down list built on egui's combo box, styled by the theme visuals.
pub fn select<T: PartialEq + Copy>(
    ui: &mut Ui,
    _t: &Tokens,
    id: &str,
    value: &mut T,
    items: &[(T, &str)],
    width: f32,
) -> Response {
    let current = items
        .iter()
        .find(|(v, _)| *v == *value)
        .map(|(_, s)| *s)
        .unwrap_or("");
    let mut changed = false;
    let mut resp = egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(width)
        .show_ui(ui, |ui| {
            for (v, s) in items {
                if ui.selectable_label(*value == *v, *s).clicked() && *value != *v {
                    *value = *v;
                    changed = true;
                }
            }
        })
        .response;
    if changed {
        resp.mark_changed();
    }
    resp
}

/// TakeUI top navigation: height 30, fully rounded.
pub fn nav_pill(ui: &mut Ui, t: &Tokens, icon: Ico, label: &str, active: bool) -> Response {
    let lw = text_size(ui, label, 13.0, Weight::Medium).x;
    let (rect, resp) =
        ui.allocate_exact_size(vec2(12.0 + 15.0 + 6.0 + lw + 12.0, 30.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if active {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(15), bg);
    let (ic, fg) = if active {
        (t.accent, t.accent)
    } else {
        (t.muted, t.text)
    };
    draw_icon(
        p,
        pos2(rect.left() + 12.0 + 7.5, rect.center().y),
        icon,
        15.0,
        dim(ic, ui),
    );
    draw_text(
        p,
        pos2(rect.left() + 33.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Medium,
        dim(fg, ui),
    );
    resp
}

/// Coral UI side navigation: full width, height 34.
pub fn nav_item(ui: &mut Ui, t: &Tokens, icon: Ico, label: &str, active: bool) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if active {
        t.raised
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(t.radius), bg);
    let ic = if active { t.accent } else { t.muted };
    draw_icon(
        p,
        pos2(rect.left() + 10.0 + 8.5, rect.center().y),
        icon,
        17.0,
        dim(ic, ui),
    );
    draw_text(
        p,
        pos2(rect.left() + 37.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Medium,
        dim(t.text, ui),
    );
    resp
}

/// Menu row: full width, height 32, optional icon, shortcut on the right or a submenu chevron.
pub fn menu_item(
    ui: &mut Ui,
    t: &Tokens,
    icon: Option<Ico>,
    label: &str,
    shortcut: Option<&str>,
    submenu: bool,
) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(6),
        mix(Color32::TRANSPARENT, t.raised, h),
    );
    if let Some(i) = icon {
        draw_icon(
            p,
            pos2(rect.left() + 10.0 + 8.0, rect.center().y),
            i,
            16.0,
            dim(t.muted, ui),
        );
    }
    draw_text(
        p,
        pos2(rect.left() + 36.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Regular,
        dim(t.text, ui),
    );
    let right = pos2(rect.right() - 10.0, rect.center().y);
    if let Some(s) = shortcut {
        draw_text(
            p,
            right,
            Align2::RIGHT_CENTER,
            s,
            11.0,
            Weight::Mono,
            dim(t.faint, ui),
        );
    }
    if submenu {
        draw_icon(
            p,
            pos2(right.x - 7.0, right.y),
            Ico::ChevronRight,
            14.0,
            dim(t.faint, ui),
        );
    }
    resp
}

/// "PrtSc ready" with a green dot, or "PrtSc in use by another app" in the warning colour.
pub fn status_chip(ui: &mut Ui, t: &Tokens, ok: bool) -> Response {
    let label = tr(if ok { Msg::PrtScReady } else { Msg::PrtScBusy });
    let lw = text_size(ui, label, 12.0, Weight::Medium).x;
    let (rect, resp) =
        ui.allocate_exact_size(vec2(10.0 + 7.0 + 6.0 + lw + 10.0, 26.0), Sense::hover());
    let p = ui.painter();
    p.rect_stroke(
        rect,
        CornerRadius::same(13),
        Stroke::new(1.0, t.border),
        StrokeKind::Inside,
    );
    let dot = if ok { t.green } else { t.warn };
    p.circle_filled(pos2(rect.left() + 13.5, rect.center().y), 3.5, dot);
    let fg = if ok { t.muted } else { t.warn };
    draw_text(
        p,
        pos2(rect.left() + 23.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        12.0,
        Weight::Medium,
        fg,
    );
    resp
}

/// Marks a selected thumbnail, swatch or object: a 2 px ring (TakeUI) or four crop ticks (Coral UI).
pub fn selection_marker(painter: &Painter, t: &Tokens, rect: Rect) {
    match t.sel_style {
        SelStyle::Ring => {
            painter.rect_stroke(
                rect.expand(2.0),
                CornerRadius::same(t.radius.saturating_add(2)),
                Stroke::new(2.0, t.primary),
                StrokeKind::Inside,
            );
        }
        SelStyle::CropTicks => {
            let o = rect.expand(5.0);
            let (l, th) = (10.0, 2.0);
            let bars = [
                Rect::from_min_size(o.left_top(), vec2(l, th)),
                Rect::from_min_size(o.left_top(), vec2(th, l)),
                Rect::from_min_size(pos2(o.right() - l, o.top()), vec2(l, th)),
                Rect::from_min_size(pos2(o.right() - th, o.top()), vec2(th, l)),
                Rect::from_min_size(pos2(o.left(), o.bottom() - th), vec2(l, th)),
                Rect::from_min_size(pos2(o.left(), o.bottom() - l), vec2(th, l)),
                Rect::from_min_size(pos2(o.right() - l, o.bottom() - th), vec2(l, th)),
                Rect::from_min_size(pos2(o.right() - th, o.bottom() - l), vec2(th, l)),
            ];
            for b in bars {
                painter.rect_filled(b, CornerRadius::ZERO, t.primary);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tokens::{ThemeId, tokens};

    /// What a few frames with the pointer outside or over a widget produced.
    struct Run {
        alloc: egui::Rect,
        painted: egui::Rect,
        hovered: bool,
    }

    /// Runs five frames 50 ms apart (below the tooltip delay, enough for the 80 ms colour ease)
    /// and returns the allocated rect, the union of everything painted and the hover state.
    fn rect_of(hover: bool, f: &dyn Fn(&mut egui::Ui) -> egui::Response) -> Run {
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, &tokens(ThemeId::TakeUi, true));
        let mut run = Run {
            alloc: egui::Rect::NOTHING,
            painted: egui::Rect::NOTHING,
            hovered: false,
        };
        for frame in 0..5 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 200.0),
                )),
                time: Some(frame as f64 * 0.05),
                ..Default::default()
            };
            if hover {
                input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(20.0, 20.0)));
            }
            let mut full = ctx.run_ui(input, |ui| {
                let r = f(ui);
                run.alloc = r.rect;
                run.hovered = r.hovered();
            });
            full.textures_delta.clear();
            run.painted = full
                .shapes
                .iter()
                .map(|c| c.shape.visual_bounding_rect())
                .fold(egui::Rect::NOTHING, |a, b| a.union(b));
        }
        run
    }

    type Widget<'a> = Box<dyn Fn(&mut egui::Ui) -> egui::Response + 'a>;

    /// Screen rects of the text shapes painted in one frame, with their text.
    fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
        fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
            match s {
                egui::Shape::Text(t) => out.push((
                    t.galley.text().to_owned(),
                    t.galley.rect.translate(t.pos.to_vec2()),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        shapes.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    #[test]
    fn confirm_dialog_answers_with_the_clicked_button_or_cancel() {
        let t = tokens(ThemeId::TakeUi, true);
        let buttons = [
            ("Discard", BtnKind::Danger, 'd'),
            ("Cancel", BtnKind::Secondary, 'c'),
            ("Copy and close", BtnKind::Primary, 'k'),
        ];
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let run = |events: Vec<Vec<egui::Event>>| {
            let ctx = egui::Context::default();
            crate::system::fonts::FontLib::load().install(&ctx);
            crate::ui::style::apply(&ctx, &t);
            let mut answers = Vec::new();
            let mut painted = Vec::new();
            for (i, ev) in std::iter::once(Vec::new()).chain(events).enumerate() {
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(i as f64 * 0.016),
                    events: ev,
                    ..Default::default()
                };
                let mut full = ctx.run_ui(input, |ui| {
                    answers.push(confirm_dialog(
                        ui.ctx(),
                        &t,
                        "test-dialog",
                        "Discard changes?",
                        "This picture has edits.",
                        &mut |_| {},
                        &buttons,
                        'c',
                    ));
                });
                full.textures_delta.clear();
                painted = texts(&full.shapes);
            }
            (answers, painted)
        };
        // egui lays a new modal out invisibly on its first frame; find the button on the second.
        let (answers, painted) = run(vec![vec![]]);
        assert_eq!(answers, vec![None, None]);
        assert!(painted.iter().any(|(s, _)| s == "Discard changes?"));
        for (label, value) in [("Copy and close", 'k'), ("Discard", 'd')] {
            let at = painted
                .iter()
                .find(|(s, _)| s == label)
                .map(|(_, r)| r.center())
                .expect("button label painted");
            let press = |down: bool| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: down,
                modifiers: egui::Modifiers::NONE,
            };
            let (answers, _) = run(vec![
                vec![egui::Event::PointerMoved(at)],
                vec![press(true)],
                vec![press(false)],
            ]);
            assert_eq!(answers.last(), Some(&Some(value)), "{label}: {answers:?}");
        }
        let (answers, _) = run(vec![vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]]);
        assert_eq!(answers.last(), Some(&Some('c')), "Esc answers with cancel");
    }

    #[test]
    fn confirm_dialog_draws_extra_content() {
        let t = tokens(ThemeId::TakeUi, true);
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, &t);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut remember = true;
        let mut box_at = None;
        let mut frame = |events: Vec<egui::Event>, i: usize, remember: &mut bool| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(i as f64 * 0.016),
                events,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                confirm_dialog(
                    ui.ctx(),
                    &t,
                    "extra-dialog",
                    "Close Takeshot?",
                    "It keeps running in the tray.",
                    &mut |ui: &mut egui::Ui| {
                        box_at = Some(checkbox(ui, &t, remember, "Remember my choice").rect);
                    },
                    &[("Quit", BtnKind::Secondary, 0)],
                    0,
                );
            });
            full.textures_delta.clear();
            box_at
        };
        frame(vec![], 0, &mut remember);
        let at = frame(vec![], 1, &mut remember)
            .expect("the extra content is drawn")
            .center();
        let b = |down| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![egui::Event::PointerMoved(at)], 2, &mut remember);
        frame(vec![b(true)], 3, &mut remember);
        frame(vec![b(false)], 4, &mut remember);
        assert!(!remember, "the checkbox inside the dialog toggles");
    }

    #[test]
    fn text_field_centres_text_with_its_icon() {
        let t = tokens(ThemeId::TakeUi, true);
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, &t);
        let glyph = icon::glyph(Ico::Search).to_string();
        for content in ["", "abc"] {
            let mut text = content.to_owned();
            let mut found = Vec::new();
            let (mut used, mut edit) = (egui::Rect::NOTHING, egui::Rect::NOTHING);
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 200.0),
                    )),
                    ..Default::default()
                };
                let mut full = ctx.run_ui(input, |ui| {
                    let top = ui.cursor().min;
                    edit = text_field(
                        ui,
                        &t,
                        &mut text,
                        "Search captures",
                        Some(Ico::Search),
                        220.0,
                    )
                    .rect;
                    used = egui::Rect::from_min_max(top, ui.min_rect().max);
                });
                full.textures_delta.clear();
                found = texts(&full.shapes);
            }
            let rect_of = |s: &str| {
                found
                    .iter()
                    .find(|(t, _)| t == s)
                    .map(|(_, r)| *r)
                    .unwrap_or_else(|| panic!("{s:?} not painted: {found:?}"))
            };
            let icon = rect_of(&glyph);
            let label = rect_of(if content.is_empty() {
                "Search captures"
            } else {
                content
            });
            // The field keeps its size and the typing area stays inside it.
            assert_eq!(used.size(), egui::vec2(220.0, 32.0), "{content:?}");
            assert!(
                used.contains_rect(edit),
                "{content:?}: {edit:?} outside {used:?}"
            );
            let off = label.center().y - icon.center().y;
            assert!(
                off.abs() <= 1.0,
                "{content:?}: text {off:+.1} px from the icon centre ({label:?} vs {icon:?})"
            );
        }
    }

    #[test]
    fn hover_buttons_act_on_their_card() {
        let t = tokens(ThemeId::TakeUi, true);
        let size = egui::vec2(180.0, 160.0);
        let card = Rect::from_min_size(egui::Pos2::ZERO, size);
        for (want, at) in card_action_rects(card) {
            let ctx = egui::Context::default();
            crate::system::fonts::FontLib::load().install(&ctx);
            let mut got = None;
            let p = at.center();
            let press = |down| egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed: down,
                modifiers: egui::Modifiers::NONE,
            };
            let frames = [
                vec![],
                vec![egui::Event::PointerMoved(p)],
                vec![press(true)],
                vec![press(false)],
            ];
            for (i, events) in frames.into_iter().enumerate() {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 300.0),
                    )),
                    time: Some(i as f64 * 0.05),
                    events,
                    ..Default::default()
                };
                let mut full = ctx.run_ui(input, |ui| {
                    let r = capture_card(ui, &t, size, CardThumb::Loading, "a.png", "1 × 1", false);
                    got = got.or(r.action);
                });
                full.textures_delta.clear();
            }
            assert_eq!(got, Some(want), "{want:?} at {at:?}");
        }
    }

    #[test]
    fn sections_and_swatches_keep_their_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        let stops = crate::engine::presets::PRESETS[0].stops;
        let cases: Vec<(&str, Widget)> = vec![
            (
                "section_header",
                Box::new(|ui| {
                    let mut open = true;
                    section_header(ui, &t, "Frame", &mut open)
                }),
            ),
            (
                "swatch",
                Box::new(|ui| swatch(ui, &t, Swatch::Gradient(stops, 135.0), false)),
            ),
            (
                "swatch selected",
                Box::new(|ui| swatch(ui, &t, Swatch::Gradient(stops, 45.0), true)),
            ),
        ];
        for (name, f) in &cases {
            let still = rect_of(false, f.as_ref());
            let hovered = rect_of(true, f.as_ref());
            assert!(!still.hovered && hovered.hovered, "{name}: pointer missed");
            assert_eq!(still.alloc, hovered.alloc, "{name} moves on hover");
            for run in [&still, &hovered] {
                assert!(
                    run.alloc.contains_rect(run.painted),
                    "{name} paints outside"
                );
            }
        }
    }

    #[test]
    fn rows_and_theme_cards_keep_their_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        let cases: Vec<(&str, Widget)> = vec![
            (
                "setting_row",
                Box::new(|ui| {
                    setting_row(
                        ui,
                        &t,
                        "Start with the system",
                        Some("Opens minimized"),
                        |ui| {
                            let mut on = true;
                            switch(ui, &t, &mut on);
                        },
                    )
                }),
            ),
            (
                "theme_card",
                Box::new(|ui| theme_card(ui, &t, ThemeId::Coral, false)),
            ),
            (
                "theme_card selected",
                Box::new(|ui| theme_card(ui, &t, ThemeId::TakeUi, true)),
            ),
        ];
        for (name, f) in &cases {
            let still = rect_of(false, f.as_ref());
            let hovered = rect_of(true, f.as_ref());
            assert!(!still.hovered && hovered.hovered, "{name}: pointer missed");
            assert_eq!(still.alloc, hovered.alloc, "{name} moves on hover");
            for run in [&still, &hovered] {
                assert!(
                    run.alloc.contains_rect(run.painted),
                    "{name} paints outside"
                );
            }
        }
    }

    #[test]
    fn cards_and_pills_keep_their_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        let cases: Vec<(&str, Widget)> = vec![
            (
                "capture_card",
                Box::new(|ui| {
                    capture_card(
                        ui,
                        &t,
                        egui::vec2(180.0, 160.0),
                        CardThumb::Failed,
                        "a.png",
                        "1 × 1",
                        false,
                    )
                    .resp
                }),
            ),
            (
                "filter_pill",
                Box::new(|ui| filter_pill(ui, &t, "Today", 12, false)),
            ),
        ];
        for (name, f) in &cases {
            let still = rect_of(false, f.as_ref());
            let hovered = rect_of(true, f.as_ref());
            assert!(!still.hovered && hovered.hovered, "{name}: pointer missed");
            assert_eq!(still.alloc, hovered.alloc, "{name} moves on hover");
            assert!(
                hovered.alloc.contains_rect(hovered.painted),
                "{name} paints outside"
            );
        }
    }

    #[test]
    fn window_controls_keep_their_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        for c in [
            WindowControl::Minimize,
            WindowControl::Maximize,
            WindowControl::Close,
        ] {
            for maximized in [false, true] {
                let f = |ui: &mut egui::Ui| window_control(ui, &t, c, maximized);
                let still = rect_of(false, &f);
                let hovered = rect_of(true, &f);
                assert_eq!(still.alloc.size(), egui::vec2(40.0, 32.0), "{c:?}");
                assert!(!still.hovered && hovered.hovered, "{c:?}: pointer missed");
                assert_eq!(still.alloc, hovered.alloc, "{c:?} moves on hover");
                assert!(
                    hovered.alloc.contains_rect(hovered.painted),
                    "{c:?} paints outside"
                );
            }
        }
    }

    #[test]
    fn components_keep_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        let cases: Vec<(&str, Widget)> = vec![
            (
                "button",
                Box::new(|ui| {
                    button(
                        ui,
                        &t,
                        BtnKind::Primary,
                        Some(Ico::Copy),
                        "Copy",
                        Some("Ctrl+C"),
                    )
                }),
            ),
            (
                "ghost",
                Box::new(|ui| button(ui, &t, BtnKind::Ghost, None, "Open", None)),
            ),
            (
                "icon_button",
                Box::new(|ui| icon_button(ui, &t, Ico::Pin, false, "Pin")),
            ),
            (
                "tool_button",
                Box::new(|ui| tool_button(ui, &t, Some(Ico::Arrow), false, "Arrow", false)),
            ),
            (
                "nav_pill",
                Box::new(|ui| nav_pill(ui, &t, Ico::Home, "Home", false)),
            ),
            (
                "nav_item",
                Box::new(|ui| nav_item(ui, &t, Ico::Grid, "Home", false)),
            ),
            (
                "menu_item",
                Box::new(|ui| menu_item(ui, &t, Some(Ico::Scan), "Capture", Some("PrtSc"), false)),
            ),
        ];
        for (name, f) in &cases {
            let still = rect_of(false, f.as_ref());
            let hovered = rect_of(true, f.as_ref());
            assert!(still.alloc.width() > 0.0, "{name} sem tamanho");
            assert!(
                !still.hovered && hovered.hovered,
                "{name}: o ponteiro não chegou ao widget"
            );
            assert_eq!(still.alloc, hovered.alloc, "{name} muda a área no hover");
            for (when, run) in [("parado", &still), ("hover", &hovered)] {
                assert!(
                    run.alloc.contains_rect(run.painted),
                    "{name} pinta fora da área ({when}): {:?} fora de {:?}",
                    run.painted,
                    run.alloc
                );
            }
        }
    }
}
