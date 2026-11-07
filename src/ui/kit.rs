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
