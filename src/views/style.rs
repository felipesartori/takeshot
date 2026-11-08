//! Estilo: the live preview with the direct-copy choice under it, and the style controls
//! in collapsible sections on the right, as in "Estilo · escuro" of the design. `controls` is meant
//! for the editor's Style tab too (plan 5b); the current editor still draws its own panel.

use crate::Shared;
use crate::engine::presets::PRESETS;
use crate::screens::home::{Home, MainAction};
use crate::system::settings::{Aspect, Background, QuickStyle, WmPos};
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, BtnKind, Ico, Swatch};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::Tokens;
use egui::{
    Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Ui, UiBuilder, pos2, vec2,
};

/// Width of the controls column, from its line to the box's outer edge, and its padding. The
/// content is 310 px at the design's size: the padding on both sides and a 10 px lane for the bar.
const COLUMN: f32 = 360.0;
const COLUMN_PAD: f32 = 20.0;

pub fn style_page(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    // Measures from "Estilo · escuro": the controls in a 360 px column behind a line from the
    // box's top to its bottom; title, preview and the direct-copy choice 24 px in on the left.
    let inner = ui.max_rect();
    let b = inner.expand(1.0); // the box, its border included
    let line = Rect::from_min_max(
        pos2(b.right() - COLUMN - 1.0, inner.top()),
        pos2(b.right() - COLUMN, inner.bottom()),
    );
    let column = Rect::from_min_max(pos2(line.right(), inner.top()), inner.max);
    let left = Rect::from_min_max(
        pos2(b.left() + 24.0, b.top() + 24.0),
        pos2((line.left() - 24.0).max(b.left() + 24.0), b.bottom() - 24.0),
    );
    ui.scope_builder(UiBuilder::new().max_rect(left), |ui| {
        preview_side(ui, t, home, shared, acts)
    });
    ui.painter().rect_filled(line, 0.0, t.border);
    // The tabs on top, the direct-copy choice at the foot, the open tab's controls between.
    let tabs_rect = Rect::from_min_size(
        column.min + vec2(COLUMN_PAD - 8.0, 8.0),
        vec2(column.width() - 2.0 * (COLUMN_PAD - 8.0), 40.0),
    );
    let tab = ui
        .scope_builder(UiBuilder::new().max_rect(tabs_rect), |ui| tabs(ui, t))
        .inner;
    let foot_h = 1.0 + COLUMN_PAD + 18.0 + 32.0 + COLUMN_PAD;
    let foot = Rect::from_min_max(pos2(column.left(), column.bottom() - foot_h), column.max);
    ui.painter().rect_filled(
        Rect::from_min_size(foot.min, vec2(foot.width(), 1.0)),
        0.0,
        t.border,
    );
    ui.scope_builder(
        UiBuilder::new().max_rect(
            foot.shrink2(vec2(COLUMN_PAD, 0.0))
                .with_min_y(foot.top() + 1.0 + COLUMN_PAD),
        ),
        |ui| direct_choice(ui, t, shared),
    );
    let body = Rect::from_min_max(
        pos2(column.left(), tabs_rect.bottom()),
        pos2(column.right(), foot.top()),
    );
    ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical()
            .id_salt(("takeshot-style-column", tab))
            .auto_shrink([false, false])
            .scroll_bar_rect(body.shrink2(vec2(0.0, 8.0)))
            .show(ui, |ui| {
                // The scroll bar sits inside the right padding, so nothing moves when it shows.
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: COLUMN_PAD as i8,
                        right: 0,
                        top: COLUMN_PAD as i8,
                        bottom: COLUMN_PAD as i8,
                    })
                    .show(ui, |ui| {
                        ui.set_width((column.width() - 2.0 * COLUMN_PAD - 10.0).max(0.0));
                        if controls(ui, t, shared, acts, tab) {
                            shared.settings_dirty = true;
                        }
                    });
            });
    });
}

/// Title and Restore default over the preview, which takes the rest of the height.
fn preview_side(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    let area = ui.max_rect();
    ui.spacing_mut().item_spacing.y = 4.0;
    // Title and subtitle with Restore default on their right, or the button under them when the
    // row does not fit (as Início's header). The button's width is remembered from the frame before.
    let (title, sub) = (tr(Msg::Style), tr(Msg::StyleSubtitle));
    let texts_w = {
        let p = ui.painter();
        let w = |s: &str, f| p.layout_no_wrap(s.to_string(), f, t.text).size().x;
        w(title, font(22.0, Weight::Bold)).max(w(sub, font(13.0, Weight::Regular)))
    };
    let restore_id = egui::Id::unique("takeui-style-restore");
    let restore_w = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(restore_id))
        .unwrap_or(160.0);
    let titles = |ui: &mut Ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(font(22.0, Weight::Bold))
                    .color(t.text),
            );
            ui.label(
                RichText::new(sub)
                    .font(font(13.0, Weight::Regular))
                    .color(t.muted),
            );
        });
    };
    let mut restore = |ui: &mut Ui| {
        let r = kit::button(
            ui,
            t,
            BtnKind::Secondary,
            Some(Ico::RotateCcw),
            tr(Msg::RestoreDefault),
            None,
        );
        if r.clicked() {
            acts.push(MainAction::RestoreStyle);
        }
        r.rect.width()
    };
    let w = if area.width() >= texts_w + 16.0 + restore_w {
        ui.horizontal(|ui| {
            titles(ui);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| restore(ui))
                .inner
        })
        .inner
    } else {
        titles(ui);
        ui.add_space(8.0);
        restore(ui)
    };
    ui.ctx().data_mut(|d| d.insert_temp(restore_id, w));
    ui.add_space(16.0 - 4.0);
    let preview_h = ui.available_rect_before_wrap().height().max(120.0);
    preview(ui, t, home, shared, vec2(area.width(), preview_h));
}

/// "When copying or saving straight from the capture" over its list, for the panel's foot.
fn direct_choice(ui: &mut Ui, t: &Tokens, shared: &mut Shared) {
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.label(
        RichText::new(tr(Msg::DirectStyleLabel))
            .font(font(12.0, Weight::Regular))
            .color(t.muted),
    );
    let before = shared.settings.quick_style;
    let w = ui.available_width();
    kit::select(
        ui,
        t,
        "takeshot-direct-choice",
        &mut shared.settings.quick_style,
        &[
            (QuickStyle::Full, tr(Msg::StyleFull)),
            (QuickStyle::CornersOnly, tr(Msg::StyleCorners)),
            (QuickStyle::Plain, tr(Msg::StyleNone)),
        ],
        w,
    );
    if shared.settings.quick_style != before {
        shared.settings_dirty = true;
    }
}

/// The preview on the canvas colour (12 px corners, no border), the picture fitted with a margin.
fn preview(ui: &mut Ui, t: &Tokens, home: &mut Home, shared: &Shared, size: egui::Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(12), t.canvas);
    if let Some((id, px)) = home.style_preview(ui.ctx(), shared) {
        let k = ((rect.width() - 48.0) / px.x)
            .min((rect.height() - 48.0) / px.y)
            .max(0.0);
        let r = Rect::from_center_size(rect.center(), px * k);
        p.image(
            id,
            r,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// A part of the style, one per tab: background, frame and watermark.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum StyleTab {
    Background,
    Frame,
    Watermark,
}

/// The style's tabs; remembers the open one (Background at first) and returns it.
pub fn tabs(ui: &mut Ui, t: &Tokens) -> StyleTab {
    let id = egui::Id::unique("takeshot-style-tab");
    let mut tab = ui
        .data(|d| d.get_temp::<StyleTab>(id))
        .unwrap_or(StyleTab::Background);
    kit::underline_tabs(
        ui,
        t,
        &mut tab,
        &[
            (StyleTab::Background, tr(Msg::Background)),
            (StyleTab::Frame, tr(Msg::Frame)),
            (StyleTab::Watermark, tr(Msg::Watermark)),
        ],
    );
    ui.data_mut(|d| d.insert_temp(id, tab));
    tab
}

/// A label on the left and a control on the right.
fn row(ui: &mut Ui, t: &Tokens, label: &str, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .font(font(13.0, Weight::Regular))
                .color(t.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
}

fn color_row(ui: &mut Ui, t: &Tokens, label: &str, c: &mut [u8; 4]) {
    row(ui, t, label, |ui| {
        let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
        if egui::color_picker::color_edit_button_srgba(
            ui,
            &mut col,
            egui::color_picker::Alpha::OnlyBlend,
        )
        .changed()
        {
            *c = col.to_srgba_unmultiplied();
        }
    });
}

/// A text link with its icon: accent colour and the hand pointer.
fn link(ui: &mut Ui, t: &Tokens, icon: Ico, text: &str) -> egui::Response {
    let g = ui
        .painter()
        .layout_no_wrap(text.to_string(), font(12.0, Weight::Medium), t.accent);
    let (rect, resp) = ui.allocate_exact_size(vec2(18.0 + g.size().x, 20.0), Sense::click());
    let p = ui.painter();
    p.text(
        egui::pos2(rect.left() + 7.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        crate::ui::icon::glyph(icon),
        crate::ui::icon::font(13.0),
        t.accent,
    );
    p.galley(
        egui::pos2(rect.left() + 18.0, rect.center().y - g.size().y / 2.0),
        g,
        t.accent,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The open tab's controls (background, frame or watermark); true when one of them changed.
pub fn controls(
    ui: &mut Ui,
    t: &Tokens,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
    tab: StyleTab,
) -> bool {
    let before = serde_json::to_string(&(&shared.settings.style, &shared.settings.watermark)).ok();
    // The design's 22 px between rows.
    ui.spacing_mut().item_spacing = vec2(6.0, 22.0);

    if tab == StyleTab::Background {
        let s = &mut shared.settings.style;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for (i, pr) in PRESETS.iter().enumerate() {
                let on = s.background == Background::Preset(i);
                if kit::swatch(ui, t, Swatch::Gradient(pr.stops, pr.angle), on)
                    .on_hover_text(pr.name)
                    .clicked()
                {
                    s.background = Background::Preset(i);
                }
            }
        });
        kit::segmented(
            ui,
            t,
            &mut s.background,
            &[
                (Background::Solid, tr(Msg::SolidColor)),
                (Background::Blurred, tr(Msg::BlurredCapture)),
                (Background::None, tr(Msg::Transparent)),
            ],
        );
        if s.background == Background::Solid {
            color_row(ui, t, tr(Msg::Color), &mut s.solid);
        }
        if s.background == Background::Custom {
            ui.horizontal(|ui| {
                for c in [&mut s.custom_a, &mut s.custom_b] {
                    let mut col = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                    if egui::color_picker::color_edit_button_srgba(
                        ui,
                        &mut col,
                        egui::color_picker::Alpha::Opaque,
                    )
                    .changed()
                    {
                        *c = col.to_srgba_unmultiplied();
                    }
                }
                if kit::icon_button(ui, t, Ico::RotateCcw, false, tr(Msg::SwapColors)).clicked() {
                    std::mem::swap(&mut s.custom_a, &mut s.custom_b);
                }
            });
            kit::slider_row(ui, t, tr(Msg::Angle), &mut s.custom_angle, 0.0..=360.0, "°");
        } else if link(ui, t, Ico::Sliders, tr(Msg::CustomizeGradient)).clicked() {
            // Starts from the gradient on screen.
            if let Background::Preset(i) = s.background
                && let Some(pr) = PRESETS.get(i)
            {
                let first = pr.stops.first().map(|x| x.1).unwrap_or([255, 255, 255]);
                let last = pr.stops.last().map(|x| x.1).unwrap_or(first);
                s.custom_a = [first[0], first[1], first[2], 255];
                s.custom_b = [last[0], last[1], last[2], 255];
                s.custom_angle = pr.angle;
            }
            s.background = Background::Custom;
        }
    }

    if tab == StyleTab::Frame {
        let s = &mut shared.settings.style;
        kit::slider_row(ui, t, tr(Msg::Margin), &mut s.padding, 0.0..=25.0, "%");
        kit::slider_row(ui, t, tr(Msg::Corners), &mut s.radius, 0.0..=60.0, "px");
        let mut shadow = s.shadow * 100.0;
        kit::slider_row(ui, t, tr(Msg::Shadow), &mut shadow, 0.0..=100.0, "%");
        s.shadow = shadow / 100.0;
        row(ui, t, tr(Msg::AspectRatio), |ui| {
            kit::segmented(
                ui,
                t,
                &mut s.aspect,
                &[
                    (Aspect::Auto, tr(Msg::FreeRatio)),
                    (Aspect::Wide, "16:9"),
                    (Aspect::Classic, "4:3"),
                    (Aspect::Square, "1:1"),
                    (Aspect::Story, "9:16"),
                ],
            );
        });
        row(ui, t, tr(Msg::WindowBar), |ui| {
            kit::switch(ui, t, &mut s.window_bar);
        });
    }

    let wm = &mut shared.settings.watermark;
    if tab == StyleTab::Watermark {
        row(ui, t, tr(Msg::ShowWatermark), |ui| {
            kit::switch(ui, t, &mut wm.enabled);
        });
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), t.input);
            match &shared.logo.thumb {
                Some(tex) => {
                    let s = tex.size_vec2();
                    let k = (28.0 / s.x).min(28.0 / s.y);
                    ui.painter().image(
                        tex.id(),
                        Rect::from_center_size(rect.center(), s * k),
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                None => {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        crate::ui::icon::glyph(Ico::ImagePlus),
                        crate::ui::icon::font(15.0),
                        t.faint,
                    );
                }
            }
            ui.vertical(|ui| {
                let name = wm
                    .logo_path
                    .as_deref()
                    .and_then(|p| std::path::Path::new(p).file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| tr(Msg::ChooseLogo).to_string());
                ui.label(
                    RichText::new(name)
                        .font(font(12.0, Weight::Medium))
                        .color(t.text),
                );
                if let (Some(p), Some(img)) = (&wm.logo_path, &shared.logo.image) {
                    let ext = std::path::Path::new(p)
                        .extension()
                        .map(|e| e.to_string_lossy().to_uppercase())
                        .unwrap_or_default();
                    ui.label(
                        RichText::new(format!("{ext} · {} × {}", img.width(), img.height()))
                            .font(font(11.0, Weight::Regular))
                            .color(t.faint),
                    );
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if wm.logo_path.is_some()
                    && kit::icon_button(ui, t, Ico::Trash, false, tr(Msg::Delete)).clicked()
                {
                    wm.logo_path = None;
                }
                let label = if wm.logo_path.is_some() {
                    Msg::Replace
                } else {
                    Msg::ChooseLogo
                };
                if kit::button(ui, t, BtnKind::Secondary, None, tr(label), None).clicked() {
                    acts.push(MainAction::ChooseLogo);
                }
            });
        });
        let w = ui.available_width();
        kit::text_field(ui, t, &mut wm.text, tr(Msg::WatermarkTextHint), None, w);
        ui.label(
            RichText::new(tr(Msg::Position))
                .font(font(12.0, Weight::Medium))
                .color(t.muted),
        );
        let selected = WmPos::GRID.iter().position(|p| *p == wm.position);
        if let Some(i) = kit::anchor_grid(ui, t, selected) {
            wm.position = WmPos::GRID[i];
        }
        kit::segmented(
            ui,
            t,
            &mut wm.position,
            &[
                (WmPos::BandTop, tr(Msg::BandTop)),
                (WmPos::BandBottom, tr(Msg::BandBottom)),
            ],
        );
        if wm.position.is_band() {
            color_row(ui, t, tr(Msg::BandColor), &mut wm.band_color);
        } else {
            kit::slider_row(ui, t, tr(Msg::Size), &mut wm.size, 3.0..=40.0, "%");
            kit::slider_row(ui, t, tr(Msg::Distance), &mut wm.margin, 0.0..=12.0, "%");
        }
        color_row(ui, t, tr(Msg::TextColor), &mut wm.text_color);
        let mut opacity = wm.opacity * 100.0;
        kit::slider_row(ui, t, tr(Msg::Opacity), &mut opacity, 10.0..=100.0, "%");
        wm.opacity = opacity / 100.0;
        if !wm.position.is_band() {
            row(ui, t, tr(Msg::OnCapture), |ui| {
                kit::switch(ui, t, &mut wm.inside_image);
            });
        }
    }
    serde_json::to_string(&(&shared.settings.style, &shared.settings.watermark)).ok() != before
}

#[cfg(test)]
mod tests {
    use crate::screens::home::Home;
    use crate::system::instance::Page;
    use crate::system::settings::QuickStyle;
    use crate::ui::i18n::{Lang, Msg, every, lang, set_lang};
    use crate::views::testing::{MESH, click, content_box, painted_at, run, run_painted};
    use egui::vec2;

    #[test]
    fn the_controls_column_runs_the_full_height() {
        // "Estilo · escuro": a 360 px column behind a line from the box's top to its bottom;
        // the title sits over the preview only.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let b = content_box(&blocks, &shared.tokens(None));
        let line = blocks
            .iter()
            .map(|(r, _)| *r)
            .find(|r| r.width() == 1.0 && r.height() > 300.0)
            .expect("the line before the column");
        assert!(
            (line.top() - b.top()).abs() <= 1.0 && (line.bottom() - b.bottom()).abs() <= 1.0,
            "{line:?} spans {b:?}"
        );
        assert!(
            (b.right() - line.right() - 360.0).abs() <= 1.0,
            "{line:?} in {b:?}"
        );
        let restore =
            painted_at(&painted, &every(Msg::RestoreDefault), |_| true).expect("Restore default");
        assert!(restore.x < line.left(), "{restore:?} over the preview");
    }

    #[test]
    fn no_text_runs_into_another_at_any_width() {
        // Restore default used to sit over the wrapped subtitle between 900 and ~1100 px.
        let before = lang();
        for l in [Lang::Pt, Lang::En] {
            set_lang(l);
            for w in (900..=1240).step_by(20) {
                let (mut home, mut shared) = Home::for_tests(vec![]);
                home.open(Page::Style);
                let (_, painted, blocks) = run_painted(
                    &mut home,
                    &mut shared,
                    (w as f32, 740.0),
                    true,
                    vec![vec![]],
                );
                let b = content_box(&blocks, &shared.tokens(None));
                let texts: Vec<_> = painted.iter().filter(|(_, r)| r.top() > b.top()).collect();
                for (i, (s, r)) in texts.iter().enumerate() {
                    for (o, q) in &texts[i + 1..] {
                        assert!(
                            !r.shrink(0.5).intersects(q.shrink(0.5)),
                            "{l:?} at {w}: {s:?} {r:?} runs into {o:?} {q:?}"
                        );
                    }
                }
            }
        }
        set_lang(before);
    }

    #[test]
    fn the_swatches_fill_their_tiles() {
        // "Estilo · escuro": each gradient fills its 44 px tile; the border sits on top.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, _, blocks) = run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let tiles: Vec<_> = blocks
            .iter()
            .filter(|(r, c)| *c == MESH && r.width() < 60.0 && (r.width() - r.height()).abs() < 1.0)
            .map(|(r, _)| *r)
            .collect();
        assert_eq!(tiles.len(), 12, "{tiles:?}");
        for r in tiles {
            assert_eq!(r.size(), vec2(44.0, 44.0), "{r:?}");
        }
    }

    #[test]
    fn the_scroll_bar_keeps_off_the_rounded_corners() {
        // The longest tab scrolls at the minimum size; its bar stops short of the box's corners.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, painted) = run(&mut home, &mut shared, (900.0, 600.0), true, vec![]);
        let tab = painted_at(&painted, &every(Msg::Watermark), |_| true).expect("the tab");
        let mut events = click(tab);
        events.push(vec![]);
        let (_, _, blocks) = run_painted(&mut home, &mut shared, (900.0, 600.0), true, events);
        let b = content_box(&blocks, &shared.tokens(None));
        let bars: Vec<_> = blocks
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| r.width() <= 6.5 && r.height() > 40.0 && r.right() > b.right() - 8.0)
            .collect();
        assert!(!bars.is_empty(), "the bar");
        for r in bars {
            assert!(
                r.top() >= b.top() + 12.0 - 0.5 && r.bottom() <= b.bottom() - 12.0 + 0.5,
                "{r:?} in {b:?}"
            );
        }
    }

    #[test]
    fn the_page_fits_in_its_box_at_the_minimum_size() {
        let before = lang();
        for l in [Lang::Pt, Lang::En] {
            set_lang(l);
            let (mut home, mut shared) = Home::for_tests(vec![]);
            home.open(Page::Style);
            let (_, painted, blocks) =
                run_painted(&mut home, &mut shared, (900.0, 600.0), true, vec![vec![]]);
            let b = content_box(&blocks, &shared.tokens(None));
            // A row too wide for its column would push the box past the window's 8 px margin.
            assert!((b.right() - (900.0 - 8.0)).abs() <= 0.5, "{l:?}: {b:?}");
            for (s, r) in painted.iter().filter(|(_, r)| r.top() > b.top()) {
                assert!(
                    r.right() <= b.right() - 25.0 + 1.0,
                    "{l:?}: {s:?} {r:?} outside {b:?}"
                );
            }
        }
        set_lang(before);
    }

    #[test]
    fn the_controls_keep_clear_of_the_scroll_bar() {
        // At the minimum size the column scrolls; its 6 px bar keeps an 8 px gap to the controls
        // instead of floating over them. The second frame knows the column overflows.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (900.0, 600.0), true, vec![vec![]]);
        let b = content_box(&blocks, &shared.tokens(None));
        // The column runs to the inner edge of the box's 1 px border; the bar sits there.
        let right = b.right() - 1.0;
        let column = painted
            .iter()
            .filter(|(_, r)| r.center().x > right - super::COLUMN && r.top() > b.top() + 10.0);
        let mut seen = 0;
        for (s, r) in column {
            seen += 1;
            assert!(r.right() <= right - 14.0 + 1.0, "{s:?} {r:?} under the bar");
        }
        assert!(seen > 5, "the controls column");
    }

    fn texts_after(
        home: &mut Home,
        shared: &mut crate::Shared,
        at: Option<egui::Pos2>,
    ) -> Vec<(String, egui::Rect)> {
        run(
            home,
            shared,
            (1160.0, 740.0),
            true,
            at.map(click).unwrap_or_default(),
        )
        .1
    }

    #[test]
    fn the_style_panel_shows_one_section_per_tab() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let shown =
            |p: &[(String, egui::Rect)], m: Msg| painted_at(p, &every(m), |_| true).is_some();
        let painted = texts_after(&mut home, &mut shared, None);
        assert!(
            shown(&painted, Msg::SolidColor) && !shown(&painted, Msg::Margin),
            "Background first"
        );
        for (tab, inside, outside) in [
            (Msg::Frame, Msg::Margin, Msg::SolidColor),
            (Msg::Watermark, Msg::ShowWatermark, Msg::Margin),
            (Msg::Background, Msg::SolidColor, Msg::ShowWatermark),
        ] {
            let at = painted_at(&painted, &every(tab), |_| true).expect("the tab");
            let painted = texts_after(&mut home, &mut shared, Some(at));
            assert!(shown(&painted, inside), "{tab:?} shows {inside:?}");
            assert!(!shown(&painted, outside), "{tab:?} hides {outside:?}");
        }
    }

    #[test]
    fn the_direct_copy_choice_sits_at_the_panel_foot() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let b = content_box(&blocks, &shared.tokens(None));
        let label =
            painted_at(&painted, &every(Msg::DirectStyleLabel), |_| true).expect("the label");
        let column_left = b.right() - super::COLUMN;
        assert!(label.x > column_left, "in the panel: {label:?}");
        assert!(
            b.bottom() - label.y < 90.0,
            "at its foot: {label:?} in {b:?}"
        );
    }

    #[test]
    fn the_direct_style_choice_is_saved() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let current =
            painted_at(&painted, &every(Msg::StyleFull), |_| true).expect("the full-style choice");
        let mut open = click(current);
        open.push(vec![]);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, open.clone());
        let corners = painted_at(&painted, &every(Msg::StyleCorners), |_| true)
            .expect("the rounded-corners-only choice in the list");
        shared.settings_dirty = false;
        // Each run starts a fresh egui context: open the list again, then pick.
        open.extend(click(corners));
        run(&mut home, &mut shared, (1160.0, 740.0), true, open);
        assert_eq!(shared.settings.quick_style, QuickStyle::CornersOnly);
        assert!(shared.settings_dirty);
    }
}
