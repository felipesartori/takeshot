//! Component kit shared by both themes: every widget keeps a fixed size and only changes colour on hover.

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
    /// Red text on the surface, outlined: a destructive action that is not the main one.
    DangerGhost,
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

/// The width of `button`'s content: icon, label and shortcut with their gaps.
fn button_content(ui: &Ui, icon: Option<Ico>, label: &str, kbd: Option<&str>) -> (f32, f32) {
    let label_w = if label.is_empty() {
        0.0
    } else {
        text_size(ui, label, 13.0, Weight::Medium).x
    };
    let mut w = label_w;
    if icon.is_some() {
        w += 16.0 + if label.is_empty() { 0.0 } else { 7.0 };
    }
    if let Some(k) = kbd {
        w += 7.0 + kbd_width(ui, k);
    }
    (w, label_w)
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
    let (content, _) = button_content(ui, icon, label, kbd);
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0 + content, 32.0), Sense::click());
    paint_button(ui, t, rect, &resp, kind, icon, label, kbd);
    resp
}

/// `button` stretched to `width` (never narrower than its content), the content centred.
pub fn wide_button(
    ui: &mut Ui,
    t: &Tokens,
    kind: BtnKind,
    icon: Option<Ico>,
    label: &str,
    kbd: Option<&str>,
    width: f32,
) -> Response {
    let (content, _) = button_content(ui, icon, label, kbd);
    let (rect, resp) =
        ui.allocate_exact_size(vec2(width.max(24.0 + content), 32.0), Sense::click());
    paint_button(ui, t, rect, &resp, kind, icon, label, kbd);
    resp
}

#[allow(clippy::too_many_arguments)]
fn paint_button(
    ui: &Ui,
    t: &Tokens,
    rect: Rect,
    resp: &Response,
    kind: BtnKind,
    icon: Option<Ico>,
    label: &str,
    kbd: Option<&str>,
) {
    let (content, label_w) = button_content(ui, icon, label, kbd);
    let h = hover_mix(ui, resp);
    let pressed = resp.is_pointer_button_down_on();
    let (rest, over, down, fg) = match kind {
        BtnKind::Primary => (t.primary, t.primary_hover, t.primary_press, t.on_primary),
        BtnKind::Secondary => (t.raised, t.hover, t.press, t.text),
        BtnKind::Ghost => (t.surf, t.raised, t.hover, t.text),
        BtnKind::Danger => (t.danger, t.danger, t.danger, Color32::WHITE),
        BtnKind::DangerGhost => (t.surf, t.raised, t.hover, t.danger),
    };
    let bg = if pressed { down } else { mix(rest, over, h) };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(t.radius), dim(bg, ui));
    if matches!(kind, BtnKind::Ghost | BtnKind::DangerGhost) {
        p.rect_stroke(
            rect,
            CornerRadius::same(t.radius),
            Stroke::new(1.0, t.border),
            StrokeKind::Inside,
        );
    }
    let fg = dim(fg, ui);
    let mut x = rect.center().x - content / 2.0;
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

/// A 44 × 44 background choice: the fill over the whole tile with a 1 px border on top, a 2 px
/// primary ring when selected; hover only colours the border.
pub fn swatch(ui: &mut Ui, t: &Tokens, fill: Swatch, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let Swatch::Gradient(stops, angle) = fill;
    paint_gradient(p, rect, stops, angle, 10.0, t.surf);
    let ring = if selected {
        Stroke::new(2.0, t.primary)
    } else {
        Stroke::new(1.0, mix(t.border, t.muted, h))
    };
    p.rect_stroke(rect, CornerRadius::same(10), ring, StrokeKind::Inside);
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
    // Measures from "Ajustes · escuro": 20 px in, 63 px tall with a hint and 60 without, 24 px
    // between the texts and the control.
    let w = ui.available_width().max(240.0);
    let h = if hint.is_some() { 63.0 } else { 60.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let left = rect.left() + 20.0;
    let used = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(left, rect.top()),
                    pos2(rect.right() - 20.0, rect.bottom()),
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            control,
        )
        .response
        .rect;
    let room = (used.left() - 24.0 - left).max(0.0);
    let line = |text: &str, size: f32, weight: Weight, color: Color32| {
        let mut job =
            egui::text::LayoutJob::simple_singleline(text.to_string(), font(size, weight), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room);
        ui.painter().layout_job(job)
    };
    let color = dim(t.text, ui);
    let g = line(label, 14.0, Weight::Medium, color);
    let top = if hint.is_some() {
        rect.top() + 14.0
    } else {
        rect.center().y - g.size().y / 2.0
    };
    ui.painter().galley(pos2(left, top), g, color);
    if let Some(hint) = hint {
        let g = line(hint, 12.0, Weight::Regular, t.muted);
        ui.painter()
            .galley(pos2(left, rect.top() + 34.0), g, t.muted);
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
/// edit, show in folder and trash over the picture; selected takes the theme's mark (a ring in
/// TakeUI, corner ticks in Coral UI). The size is the caller's (the grid's column width) and never
/// depends on hover.
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
    paint_thumb(&p, t, pic, thumb, 6, 22.0);
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
        // The theme's mark (ring or corner ticks), drawn on the card's own edge so a scroll
        // area's clip never cuts it.
        selection_marker(&p, t, rect.shrink(5.0));
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

/// A picture fitted in `pic` on the input colour (`radius` corners), or the broken-image icon.
fn paint_thumb(p: &Painter, t: &Tokens, pic: Rect, thumb: CardThumb, radius: u8, icon: f32) {
    p.rect_filled(pic, CornerRadius::same(radius), t.input);
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
        CardThumb::Failed => draw_icon(p, pic.center(), Ico::Images, icon, t.faint),
        _ => {}
    }
}

/// A picture fitted in `pic` on the input colour, 8 px corners (the detail panel's preview).
pub fn thumb(p: &Painter, t: &Tokens, pic: Rect, thumb: CardThumb) {
    paint_thumb(p, t, pic, thumb, 8, 24.0);
}

/// A list row's height.
pub const ROW_H: f32 = 48.0;

/// Where a list row's parts start from its left edge. A column that does not fit is `None`:
/// dimensions go first, then size, then time, so the name keeps 160 px.
#[derive(Clone, Copy, Debug)]
pub struct ListCols {
    pub thumb: f32,
    pub name: f32,
    pub name_w: f32,
    /// Time, dimensions and size.
    pub cells: [Option<f32>; 3],
    pub buttons: f32,
}

/// The columns of a list row `width` wide.
pub fn list_columns(width: f32) -> ListCols {
    let (pad, gap) = (12.0, 12.0);
    let widths = [70.0, 110.0, 80.0];
    let buttons = width - pad - (4.0 * 32.0 + 3.0 * 2.0);
    let name = pad + 56.0 + gap;
    let mut show = [true; 3];
    let room = |show: &[bool; 3]| {
        let cells: f32 = show
            .iter()
            .zip(widths)
            .filter(|(s, _)| **s)
            .map(|(_, w)| w + gap)
            .sum();
        buttons - gap - cells - name
    };
    for drop in [1, 2, 0] {
        if room(&show) >= 160.0 {
            break;
        }
        show[drop] = false;
    }
    let mut x = buttons - gap;
    let mut cells = [None; 3];
    for i in [2, 1, 0] {
        if show[i] {
            x -= widths[i];
            cells[i] = Some(x);
            x -= gap;
        }
    }
    ListCols {
        thumb: pad,
        name,
        name_w: (x - name).max(0.0),
        cells,
        buttons,
    }
}

/// A capture as a list row, `width` × 48: picture, name (cut with "…" to its column), then time,
/// dimensions and size (`cols`) at `list_columns`; on hover or when chosen, the copy, edit, show
/// in folder and trash buttons. A chosen row is tinted. The size never depends on hover.
pub fn capture_row(
    ui: &mut Ui,
    t: &Tokens,
    width: f32,
    thumb: CardThumb,
    name: &str,
    cols: [&str; 3],
    selected: bool,
) -> CardResponse {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter().clone();
    let bg = if selected {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(6), bg);
    let c = list_columns(width);
    let pic = Rect::from_min_size(
        pos2(rect.left() + c.thumb, rect.center().y - 18.0),
        vec2(56.0, 36.0),
    );
    paint_thumb(&p, t, pic, thumb, 4, 16.0);
    let cell = |s: &str, size: f32, w: Weight, c: Color32, room: f32| {
        let mut job = egui::text::LayoutJob::simple_singleline(s.to_string(), font(size, w), c);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room.max(1.0));
        ui.painter().layout_job(job)
    };
    let name_g = cell(name, 13.0, Weight::Medium, dim(t.text, ui), c.name_w);
    p.galley(
        pos2(
            rect.left() + c.name,
            rect.center().y - name_g.size().y / 2.0,
        ),
        name_g,
        t.text,
    );
    for (i, s) in cols.iter().enumerate() {
        let Some(at) = c.cells[i] else {
            continue;
        };
        let g = cell(
            s,
            12.0,
            Weight::Regular,
            dim(t.muted, ui),
            [70.0, 110.0, 80.0][i],
        );
        p.galley(
            pos2(rect.left() + at, rect.center().y - g.size().y / 2.0),
            g,
            t.muted,
        );
    }
    let mut action = None;
    if resp.contains_pointer() || selected {
        for (i, a) in [
            CardAction::Copy,
            CardAction::Edit,
            CardAction::Folder,
            CardAction::Trash,
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::from_min_size(
                pos2(
                    rect.left() + c.buttons + i as f32 * 34.0,
                    rect.center().y - 16.0,
                ),
                vec2(32.0, 32.0),
            );
            let b = ui.interact(r, resp.id.with(("row-action", a as u8)), Sense::click());
            let bh = hover_mix(ui, &b);
            p.rect_filled(
                r,
                CornerRadius::same(t.radius),
                mix(Color32::TRANSPARENT, t.hover, bh),
            );
            let (icon, tip) = match a {
                CardAction::Copy => (Ico::Copy, Msg::Copy),
                CardAction::Edit => (Ico::Edit, Msg::EditInEditor),
                CardAction::Folder => (Ico::FolderOpen, Msg::ShowInFolder),
                CardAction::Trash => (Ico::Trash, Msg::Delete),
            };
            draw_icon(&p, r.center(), icon, 16.0, mix(t.muted, t.text, bh));
            if b.on_hover_text(tr(tip)).clicked() {
                action = Some(a);
            }
        }
    }
    CardResponse { resp, action }
}

/// A choice between a few icons, 32 px tall: 30 × 28 parts 2 px apart inside, the chosen one on
/// `seg_sel`; each answers with its tip on hover.
pub fn icon_segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    t: &Tokens,
    value: &mut T,
    items: &[(T, Ico, &str)],
) -> Response {
    let n = items.len() as f32;
    let (rect, mut resp) = ui.allocate_exact_size(
        vec2(4.0 + 30.0 * n + 2.0 * (n - 1.0).max(0.0), 32.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, CornerRadius::same(t.radius), t.raised);
    for (i, (v, icon, tip)) in items.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + 2.0 + i as f32 * 32.0, rect.top() + 2.0),
            vec2(30.0, 28.0),
        );
        let item = ui.interact(r, resp.id.with(i), Sense::click());
        let h = hover_mix(ui, &item);
        let item = item.on_hover_text(*tip);
        resp = resp.union(item.clone());
        if item.clicked() && *value != *v {
            *value = *v;
            resp.mark_changed();
        }
        let sel = *value == *v;
        let p = ui.painter();
        if sel {
            p.rect_filled(r, CornerRadius::same(6), t.seg_sel);
        }
        let fg = if sel { t.text } else { mix(t.muted, t.text, h) };
        draw_icon(p, r.center(), *icon, 15.0, dim(fg, ui));
    }
    resp
}

/// A window button of the theme's title bar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

/// 32 × 32 window button, rounded like the bar's other buttons. Hover fills `hover`; Close fills
/// `danger` with a white glyph. With the window maximized, Maximize shows the restore glyph.
pub fn window_control(ui: &mut Ui, t: &Tokens, c: WindowControl, maximized: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
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
        16.0,
        dim(mix(t.muted, hover_fg, h), ui),
    );
    resp.on_hover_text(tr(tip))
}

/// 34 × 34 tool. The active tool is filled in primary in Coral UI and tinted in TakeUI.
/// `None` draws the numbered-step tool: a circle with "1".
pub fn tool_button(
    ui: &mut Ui,
    t: &Tokens,
    icon: Option<Ico>,
    active: bool,
    tip: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let (bg, fg) = match (active, t.theme == ThemeId::Coral) {
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
        None => step_glyph(p, rect.center(), 18.0, fg),
    }
    resp.on_hover_text(tip)
}

/// The numbered step's icon, `size` px: a circle with "1".
pub fn step_glyph(p: &Painter, center: egui::Pos2, size: f32, fg: Color32) {
    p.circle_stroke(center, size / 2.0, Stroke::new(size / 11.0, fg));
    draw_text(
        p,
        center,
        Align2::CENTER_CENTER,
        "1",
        size * 0.55,
        Weight::Bold,
        fg,
    );
}

/// Underlined tabs across the full width, 40 px: the labels in equal parts, the open one in the
/// text colour over a 2 px line in the accent, the others muted; a 1 px line runs under them all.
pub fn underline_tabs<T: PartialEq + Copy>(
    ui: &mut Ui,
    t: &Tokens,
    value: &mut T,
    items: &[(T, &str)],
) -> Response {
    let w = ui.available_width();
    let (rect, mut resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - 1.0), rect.max),
        0.0,
        t.border,
    );
    let part = w / items.len().max(1) as f32;
    for (i, (v, s)) in items.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + i as f32 * part, rect.top()),
            vec2(part, 40.0),
        );
        let item = ui.interact(r, resp.id.with(i), Sense::click());
        let h = hover_mix(ui, &item);
        // The bar answers for its parts: hovered or clicked when one of them is.
        resp = resp.union(item.clone());
        if item.clicked() && *value != *v {
            *value = *v;
            resp.mark_changed();
        }
        let sel = *value == *v;
        let p = ui.painter();
        if sel {
            p.rect_filled(
                Rect::from_min_max(pos2(r.left(), r.bottom() - 2.0), r.max),
                0.0,
                t.accent,
            );
        }
        let fg = if sel { t.text } else { mix(t.muted, t.text, h) };
        draw_text(
            p,
            pos2(r.center().x, r.center().y - 1.0),
            Align2::CENTER_CENTER,
            s,
            13.0,
            Weight::Medium,
            dim(fg, ui),
        );
    }
    resp
}

/// What a floating bar's button shows: an icon, or a colour (the object's).
#[derive(Clone, Copy, Debug)]
pub enum BarItem {
    Icon(Ico),
    Color(Color32),
}

/// A 28 x 28 button of a floating bar (radius 6): a 16 px icon, or a 16 px colour dot ringed in
/// the border colour; hover only colours the background.
/// A floating bar's button, 28 px tall: an icon (28 wide) or a colour with the chevron of its menu
/// (40 wide).
pub fn bar_button(ui: &mut Ui, t: &Tokens, item: BarItem, tip: &str) -> Response {
    let w = match item {
        BarItem::Icon(_) => 28.0,
        BarItem::Color(_) => 40.0,
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    p.rect_filled(
        rect,
        CornerRadius::same(6),
        mix(Color32::TRANSPARENT, t.raised, h),
    );
    match item {
        BarItem::Icon(i) => draw_icon(p, rect.center(), i, 16.0, dim(mix(t.muted, t.text, h), ui)),
        BarItem::Color(c) => {
            let dot = pos2(rect.left() + 14.0, rect.center().y);
            p.circle_filled(dot, 8.0, c);
            p.circle_stroke(dot, 7.5, Stroke::new(1.0, t.border));
            draw_icon(
                p,
                pos2(rect.right() - 10.0, rect.center().y),
                Ico::ChevronDown,
                12.0,
                dim(mix(t.muted, t.text, h), ui),
            );
        }
    }
    resp.on_hover_text(tip)
}

/// A floating bar's button with an icon and a short text, 28 px tall: 8 px in, the icon, the text,
/// and the chevron of its menu when it opens one. `on` tints it (a switch that is on).
pub fn bar_text(
    ui: &mut Ui,
    t: &Tokens,
    icon: Ico,
    text: &str,
    chevron: bool,
    on: bool,
    tip: &str,
) -> Response {
    let tw = text_size(ui, text, 12.0, Weight::Medium).x;
    let w = 8.0 + 16.0 + 6.0 + tw + if chevron { 4.0 + 12.0 } else { 0.0 } + 8.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if on {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(6), bg);
    let fg = dim(
        if on {
            t.accent
        } else {
            mix(t.muted, t.text, h)
        },
        ui,
    );
    let mut x = rect.left() + 8.0;
    draw_icon(p, pos2(x + 8.0, rect.center().y), icon, 16.0, fg);
    x += 16.0 + 6.0;
    let text_fg = dim(if on { t.accent } else { t.text }, ui);
    draw_text(
        p,
        pos2(x, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        12.0,
        Weight::Medium,
        text_fg,
    );
    if chevron {
        draw_icon(
            p,
            pos2(x + tw + 4.0 + 6.0, rect.center().y),
            Ico::ChevronDown,
            12.0,
            fg,
        );
    }
    resp.on_hover_text(tip)
}

/// What a row of the object list answered: the row, and the eye that hides or shows its object.
pub struct ObjectRow {
    pub row: Response,
    pub eye: Response,
}

/// A row of the editor's object list: full width, 34 px, a 16 px icon 10 px in (`None` draws the
/// numbered step), the label at 36 px (13 medium) and the 15 px eye 10 px from the right. The
/// selected row is tinted, in the accent colour; a hidden object is faint, its eye crossed out.
pub fn object_row(
    ui: &mut Ui,
    t: &Tokens,
    icon: Option<Ico>,
    label: &str,
    selected: bool,
    hidden: bool,
) -> ObjectRow {
    let w = ui.available_width();
    let (rect, row) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
    let eye_at = pos2(rect.right() - 10.0 - 7.5, rect.center().y);
    let eye = ui.interact(
        Rect::from_center_size(eye_at, vec2(24.0, 24.0)),
        row.id.with("eye"),
        Sense::click(),
    );
    let h = hover_mix(ui, &row).max(hover_mix(ui, &eye));
    let p = ui.painter();
    let bg = if selected {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(8), bg);
    let (ic, fg) = match (selected, hidden) {
        (true, _) => (t.accent, t.accent),
        (false, true) => (t.faint, t.faint),
        (false, false) => (t.muted, t.text),
    };
    let icon_at = pos2(rect.left() + 10.0 + 8.0, rect.center().y);
    match icon {
        Some(i) => draw_icon(p, icon_at, i, 16.0, dim(ic, ui)),
        None => step_glyph(p, icon_at, 15.0, dim(ic, ui)),
    }
    let room = (eye_at.x - 7.5 - 10.0 - (rect.left() + 36.0)).max(1.0);
    let mut job = egui::text::LayoutJob::simple_singleline(
        label.to_owned(),
        font(13.0, Weight::Medium),
        dim(fg, ui),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let g = p.layout_job(job);
    p.galley(
        pos2(rect.left() + 36.0, rect.center().y - g.size().y / 2.0),
        g,
        fg,
    );
    let eye_fg = mix(t.faint, t.text, hover_mix(ui, &eye));
    draw_icon(
        p,
        eye_at,
        if hidden { Ico::EyeOff } else { Ico::Eye },
        15.0,
        dim(eye_fg, ui),
    );
    ObjectRow { row, eye }
}

/// An 18 × 18 colour to pick, as in the editor's tool bar: the colour in a circle, a 1 px ring
/// around dark colours, a 2 px ring in the text colour when chosen; hover only rings it in muted.
pub fn color_dot(ui: &mut Ui, t: &Tokens, color: Color32, selected: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let c = rect.center();
    if selected {
        p.circle_filled(c, 9.0, t.text);
        p.circle_filled(c, 7.0, t.surf);
        p.circle_filled(c, 5.5, color);
    } else {
        p.circle_filled(c, 7.5, color);
        let ring = mix(t.border, t.muted, h);
        p.circle_stroke(c, 8.5, Stroke::new(1.0, ring));
    }
    resp.on_hover_text(tip)
}

/// A 28 × 28 stroke width to pick: a dot as wide as the stroke (2 to 12 px drawn 3 to 11 px),
/// on the raised colour when chosen; hover only colours the background.
pub fn width_dot(ui: &mut Ui, t: &Tokens, width: f32, selected: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if selected {
        t.raised
    } else {
        mix(Color32::TRANSPARENT, t.hover, h)
    };
    p.rect_filled(rect, CornerRadius::same(t.radius), bg);
    let r = (width * 0.5 + 1.0).clamp(1.5, 5.5);
    let fg = if selected { t.text } else { t.muted };
    p.circle_filled(rect.center(), r, fg);
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

/// Label (84 px), track, and a value box such as "7 %". The answer carries the track's drag, so
/// a caller can tell when the slider is grabbed and let go.
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
    resp.union(drag)
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

/// Drop-down list on egui's combo box: 32 px tall, the input fill with a border (as the field in
/// "Ajustes · escuro"); hover only colours the border.
pub fn select<T: PartialEq + Copy>(
    ui: &mut Ui,
    t: &Tokens,
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
    let mut resp = combo(ui, t, id, current, width, 32.0, |ui| {
        for (v, s) in items {
            if ui.selectable_label(*value == *v, *s).clicked() && *value != *v {
                *value = *v;
                changed = true;
            }
        }
    });
    if changed {
        resp.mark_changed();
    }
    resp
}

/// A drop-down `height` px tall showing `text`, in the select's look; `contents` fills the list.
pub fn combo(
    ui: &mut Ui,
    t: &Tokens,
    id: &str,
    text: &str,
    width: f32,
    height: f32,
    contents: impl FnOnce(&mut Ui),
) -> Response {
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = height;
        ui.spacing_mut().button_padding = vec2(10.0, ((height - 16.0) / 2.0).max(2.0));
        let w = &mut ui.visuals_mut().widgets;
        for (st, stroke) in [
            (&mut w.inactive, t.border),
            (&mut w.hovered, t.muted),
            (&mut w.active, t.muted),
            (&mut w.open, t.muted),
        ] {
            st.weak_bg_fill = t.input;
            st.bg_stroke = Stroke::new(1.0, stroke);
        }
        let chevron = t.muted;
        egui::ComboBox::from_id_salt(id)
            .selected_text(text)
            .width(width)
            .icon(move |ui, rect, _, _| {
                draw_icon(ui.painter(), rect.center(), Ico::ChevronDown, 15.0, chevron)
            })
            .show_ui(ui, contents)
            .response
    })
    .inner
}

/// An 18 × 18 "+" in the colour row that opens a custom colour; hover only colours its ring.
pub fn color_add(ui: &mut Ui, t: &Tokens, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    p.circle_stroke(
        rect.center(),
        8.5,
        Stroke::new(1.0, mix(t.border, t.muted, h)),
    );
    draw_icon(p, rect.center(), Ico::Plus, 12.0, mix(t.muted, t.text, h));
    resp.on_hover_text(tip)
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

/// TakeUI's Settings section: full width, height 36, a 16 px icon 12 px in and the label after it.
/// The open section is tinted with the label and icon in the accent colour.
pub fn section_item(ui: &mut Ui, t: &Tokens, icon: Ico, label: &str, active: bool) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 36.0), Sense::click());
    let h = hover_mix(ui, &resp);
    let p = ui.painter();
    let bg = if active {
        t.tint
    } else {
        mix(Color32::TRANSPARENT, t.raised, h)
    };
    p.rect_filled(rect, CornerRadius::same(t.radius), bg);
    let (ic, fg) = if active {
        (t.accent, t.accent)
    } else {
        (t.muted, t.text)
    };
    draw_icon(
        p,
        pos2(rect.left() + 12.0 + 8.0, rect.center().y),
        icon,
        16.0,
        dim(ic, ui),
    );
    draw_text(
        p,
        pos2(rect.left() + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        13.0,
        Weight::Medium,
        dim(fg, ui),
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

/// What the shortcut status chip shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChipState {
    /// The shortcuts work: a green dot.
    Ready,
    /// Another app holds one: the warning colour.
    Busy,
    /// Switched off or refused by the system: a faint dot.
    Off,
}

/// The shortcut status in a pill: a dot in the state's colour and `label` ("PrtSc ready").
pub fn status_chip(ui: &mut Ui, t: &Tokens, state: ChipState, label: &str) -> Response {
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
    let (dot, fg) = match state {
        ChipState::Ready => (t.green, t.muted),
        ChipState::Busy => (t.warn, t.warn),
        ChipState::Off => (t.faint, t.muted),
    };
    p.circle_filled(pos2(rect.left() + 13.5, rect.center().y), 3.5, dot);
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

/// How long a toast stays on screen: long enough to read (about 15 characters a second), from
/// 2.6 s to 8 s.
pub fn toast_seconds(text: &str) -> f32 {
    (text.chars().count() as f32 / 15.0).clamp(2.6, 8.0)
}

/// The short notice at the bottom of a window ("Copied"), in the theme: a card with a dot in the
/// primary colour and the text.
pub fn toast(ctx: &egui::Context, t: &Tokens, text: &str) {
    egui::Area::new(egui::Id::unique("toast"))
        .order(egui::Order::Tooltip)
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -28.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(t.surf)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(14, 9))
                .shadow(egui::Shadow {
                    offset: [0, 6],
                    blur: 22,
                    spread: 0,
                    color: Color32::from_black_alpha(90),
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, t.primary);
                        ui.label(
                            egui::RichText::new(text)
                                .font(font(13.0, Weight::Regular))
                                .color(t.text),
                        );
                    });
                });
        });
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

    /// Texts, rectangle fills, rectangle strokes and circle fills one frame painted.
    #[derive(Default)]
    struct Shapes {
        texts: Vec<(String, egui::Rect)>,
        fills: Vec<egui::Color32>,
        strokes: Vec<egui::Color32>,
        dots: Vec<egui::Color32>,
    }

    fn shapes(clipped: &[egui::epaint::ClippedShape]) -> Shapes {
        fn walk(s: &egui::Shape, out: &mut Shapes) {
            match s {
                egui::Shape::Text(t) => out.texts.push((
                    t.galley.text().to_owned(),
                    t.galley.rect.translate(t.pos.to_vec2()),
                )),
                egui::Shape::Rect(r) => {
                    out.fills.push(r.fill);
                    out.strokes.push(r.stroke.color);
                }
                egui::Shape::Circle(c) => out.dots.push(c.fill),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = Shapes::default();
        clipped.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    /// Frames of `draw` at 800 × 600 in `t`'s look, one per event list after a first empty one;
    /// returns what the last frame painted.
    fn frames(
        t: &crate::ui::tokens::Tokens,
        events: Vec<Vec<egui::Event>>,
        mut draw: impl FnMut(&mut egui::Ui),
    ) -> Shapes {
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, t);
        let mut last = Shapes::default();
        for (i, ev) in std::iter::once(Vec::new()).chain(events).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                time: Some(i as f64 * 0.05),
                events: ev,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| draw(ui));
            full.textures_delta.clear();
            last = shapes(&full.shapes);
        }
        last
    }

    #[test]
    fn a_long_toast_stays_long_enough_to_read() {
        assert_eq!(toast_seconds("Copied"), 2.6);
        let long = "PrtSc is in use by another app. Close that app, or turn on Takeshot priority.";
        assert!(toast_seconds(long) > 4.5, "{}", toast_seconds(long));
        assert_eq!(toast_seconds(&"x".repeat(400)), 8.0);
    }

    #[test]
    fn the_open_tab_is_underlined_in_the_accent() {
        for theme in [ThemeId::TakeUi, ThemeId::Coral] {
            let t = tokens(theme, true);
            let mut blocks = Vec::new();
            let ctx = egui::Context::default();
            crate::system::fonts::FontLib::load().install(&ctx);
            crate::ui::style::apply(&ctx, &t);
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0))),
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                let mut tab = 1;
                underline_tabs(ui, &t, &mut tab, &[(0, "A"), (1, "B"), (2, "C")]);
            });
            full.textures_delta.clear();
            fn walk(s: &egui::Shape, out: &mut Vec<(Rect, Color32)>) {
                match s {
                    egui::Shape::Rect(r) => out.push((r.rect, r.fill)),
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                    _ => {}
                }
            }
            full.shapes.iter().for_each(|c| walk(&c.shape, &mut blocks));
            let line = blocks
                .iter()
                .find(|(r, c)| *c == t.accent && r.height() == 2.0)
                .map(|(r, _)| *r)
                .expect("the underline");
            let third = blocks
                .iter()
                .filter(|(r, _)| r.height() == 1.0)
                .map(|(r, _)| r.width())
                .fold(0.0, f32::max)
                / 3.0;
            assert!(
                (line.center().x - 1.5 * third).abs() <= 1.0,
                "{theme:?}: under the middle tab: {line:?}"
            );
        }
    }

    #[test]
    fn a_chosen_card_is_marked_the_theme_s_way() {
        // Coral UI: four corner ticks in coral; TakeUI: a ring in purple.
        for (theme, ticks) in [(ThemeId::Coral, true), (ThemeId::TakeUi, false)] {
            let t = tokens(theme, true);
            let ctx = egui::Context::default();
            crate::system::fonts::FontLib::load().install(&ctx);
            crate::ui::style::apply(&ctx, &t);
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))),
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                ui.add_space(20.0);
                capture_card(
                    ui,
                    &t,
                    vec2(220.0, 180.0),
                    CardThumb::Loading,
                    "a.png",
                    "1 KB",
                    true,
                );
            });
            full.textures_delta.clear();
            let mut filled = Vec::new();
            let mut stroked = Vec::new();
            fn walk(s: &egui::Shape, f: &mut Vec<(Rect, Color32)>, st: &mut Vec<Color32>) {
                match s {
                    egui::Shape::Rect(r) => {
                        f.push((r.rect, r.fill));
                        st.push(r.stroke.color);
                    }
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, f, st)),
                    _ => {}
                }
            }
            full.shapes
                .iter()
                .for_each(|c| walk(&c.shape, &mut filled, &mut stroked));
            let tick_count = filled
                .iter()
                .filter(|(r, c)| *c == t.primary && r.width().min(r.height()) == 2.0)
                .count();
            if ticks {
                assert_eq!(tick_count, 8, "{theme:?}: eight tick bars");
                assert!(!stroked.contains(&t.primary), "{theme:?}: no ring");
            } else {
                assert_eq!(tick_count, 0, "{theme:?}: no ticks");
                assert!(stroked.contains(&t.primary), "{theme:?}: the ring");
            }
        }
    }

    #[test]
    fn the_toast_follows_the_theme() {
        for theme in [ThemeId::TakeUi, ThemeId::Coral] {
            for dark in [true, false] {
                let t = tokens(theme, dark);
                let painted = frames(&t, vec![vec![]; 8], |ui| toast(ui.ctx(), &t, "Copied"));
                assert!(painted.fills.contains(&t.surf), "{theme:?} {dark}: card");
                assert!(painted.dots.contains(&t.primary), "{theme:?} {dark}: dot");
            }
        }
    }
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
    fn object_rows_keep_their_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        let cases: Vec<(&str, Widget, f32)> = vec![
            (
                "object_row",
                Box::new(|ui| object_row(ui, &t, Some(Ico::Rect), "Rectangle", false, false).row),
                34.0,
            ),
            (
                "object_row selected and hidden",
                Box::new(|ui| object_row(ui, &t, None, "Step 1", true, true).row),
                34.0,
            ),
            (
                "underline_tabs",
                Box::new(|ui| {
                    let mut tab = 0;
                    underline_tabs(ui, &t, &mut tab, &[(0, "Object"), (1, "Style")])
                }),
                40.0,
            ),
            (
                "bar_button icon",
                Box::new(|ui| bar_button(ui, &t, BarItem::Icon(Ico::Trash), "Delete")),
                28.0,
            ),
            (
                "bar_button colour",
                Box::new(|ui| bar_button(ui, &t, BarItem::Color(Color32::RED), "Colour")),
                28.0,
            ),
            (
                "wide_button",
                Box::new(|ui| {
                    wide_button(
                        ui,
                        &t,
                        BtnKind::DangerGhost,
                        Some(Ico::Trash),
                        "Delete",
                        Some("Del"),
                        236.0,
                    )
                }),
                32.0,
            ),
            (
                "capture_row",
                Box::new(|ui| {
                    capture_row(
                        ui,
                        &t,
                        600.0,
                        CardThumb::Loading,
                        "a.png",
                        ["14:32", "1920 × 1080", "412 KB"],
                        false,
                    )
                    .resp
                }),
                48.0,
            ),
            (
                "icon_segmented",
                Box::new(|ui| {
                    let mut v = 0;
                    icon_segmented(
                        ui,
                        &t,
                        &mut v,
                        &[(0, Ico::Grid, "Grid"), (1, Ico::List, "List")],
                    )
                }),
                32.0,
            ),
            (
                "bar_text",
                Box::new(|ui| bar_text(ui, &t, Ico::Line, "3 px", true, false, "Width")),
                28.0,
            ),
            (
                "bar_text on",
                Box::new(|ui| bar_text(ui, &t, Ico::PaintBucket, "Fill", false, true, "Fill")),
                28.0,
            ),
        ];
        for (name, f, height) in &cases {
            let still = rect_of(false, f.as_ref());
            let hovered = rect_of(true, f.as_ref());
            assert!(!still.hovered && hovered.hovered, "{name}: pointer missed");
            assert_eq!(still.alloc, hovered.alloc, "{name} moves on hover");
            assert_eq!(still.alloc.height(), *height, "{name}");
            for run in [&still, &hovered] {
                assert!(
                    run.alloc.contains_rect(run.painted),
                    "{name} paints outside"
                );
            }
        }
    }

    #[test]
    fn swatches_keep_their_size_on_hover() {
        let t = tokens(ThemeId::TakeUi, true);
        let stops = crate::engine::presets::PRESETS[0].stops;
        let cases: Vec<(&str, Widget)> = vec![
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
            if name.starts_with("swatch") {
                // "Estilo · escuro": six 44 px squares, 8 px apart, across the column.
                assert_eq!(still.alloc.size(), egui::vec2(44.0, 44.0), "{name}");
            } else {
                // A text row in the design (17 px), with room for the watermark's 20 px switch.
                assert_eq!(still.alloc.height(), 20.0, "{name}");
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
                // The full height of the 52 px bar, as in the design.
                assert_eq!(still.alloc.size(), egui::vec2(32.0, 32.0), "{c:?}");
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
                Box::new(|ui| tool_button(ui, &t, Some(Ico::Arrow), false, "Arrow")),
            ),
            (
                "nav_pill",
                Box::new(|ui| nav_pill(ui, &t, Ico::Home, "Home", false)),
            ),
            (
                "section_item",
                Box::new(|ui| section_item(ui, &t, Ico::Sliders, "General", false)),
            ),
            (
                "color_dot",
                Box::new(|ui| color_dot(ui, &t, Color32::RED, false, "Red")),
            ),
            (
                "color_dot selected",
                Box::new(|ui| color_dot(ui, &t, Color32::RED, true, "Red")),
            ),
            (
                "width_dot",
                Box::new(|ui| width_dot(ui, &t, 5.0, false, "5 px")),
            ),
            ("color_add", Box::new(|ui| color_add(ui, &t, "Custom"))),
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

    /// The fills of the rectangles one frame of `draw` paints.
    fn fills(t: &Tokens, draw: impl Fn(&mut egui::Ui)) -> Vec<Color32> {
        fn walk(s: &egui::Shape, out: &mut Vec<Color32>) {
            match s {
                egui::Shape::Rect(r) => out.push(r.fill),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, t);
        let mut full = ctx.run_ui(egui::RawInput::default(), |ui| draw(ui));
        full.textures_delta.clear();
        let mut out = Vec::new();
        full.shapes.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    #[test]
    fn the_active_tool_is_filled_in_coral_and_tinted_in_takeui() {
        for (theme, want) in [(ThemeId::Coral, true), (ThemeId::TakeUi, false)] {
            let t = tokens(theme, true);
            let painted = fills(&t, |ui| {
                tool_button(ui, &t, Some(Ico::Rect), true, "Rect");
            });
            let bg = if want { t.primary } else { t.tint };
            assert!(painted.contains(&bg), "{theme:?}: {painted:?}");
            let ctx = egui::Context::default();
            crate::system::fonts::FontLib::load().install(&ctx);
            crate::ui::style::apply(&ctx, &t);
            let mut full = ctx.run_ui(egui::RawInput::default(), |ui| {
                tool_button(ui, &t, Some(Ico::Rect), true, "Rect");
            });
            full.textures_delta.clear();
            let icon = full
                .shapes
                .iter()
                .find_map(|c| match &c.shape {
                    egui::Shape::Text(s) => s.galley.job.sections.first().map(|x| x.format.color),
                    _ => None,
                })
                .expect("the icon");
            let fg = if want { t.on_primary } else { t.accent };
            assert_eq!(icon, fg, "{theme:?}: the icon reads on its fill");
        }
    }
}
