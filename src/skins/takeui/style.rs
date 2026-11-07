//! TakeUI's Estilo: the live preview with the direct-copy choice under it, and the style controls
//! in collapsible sections on the right, as in "Estilo · escuro" of the design. `controls` is also
//! what the editor's Style tab shows.

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

/// Width of the controls column, its line included, and its padding.
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
    ui.scope_builder(UiBuilder::new().max_rect(column), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("takeui-style-column")
            .auto_shrink([false, false])
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
                        ui.set_width((column.width() - 2.0 * COLUMN_PAD).max(0.0));
                        if controls(ui, t, shared, acts) {
                            shared.settings_dirty = true;
                        }
                    });
            });
    });
}

/// Title and Restore default over the preview, and the direct-copy choice under it: on one row
/// when it fits, else its label over it. The choice's width is remembered from the frame before.
fn preview_side(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    let area = ui.max_rect();
    ui.spacing_mut().item_spacing.y = 4.0;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(tr(Msg::Style))
                    .font(font(22.0, Weight::Bold))
                    .color(t.text),
            );
            ui.label(
                RichText::new(tr(Msg::StyleSubtitle))
                    .font(font(13.0, Weight::Regular))
                    .color(t.muted),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if kit::button(
                ui,
                t,
                BtnKind::Secondary,
                Some(Ico::RotateCcw),
                tr(Msg::RestoreDefault),
                None,
            )
            .clicked()
            {
                acts.push(MainAction::RestoreStyle);
            }
        });
    });
    ui.add_space(16.0 - 4.0);
    let label = tr(Msg::DirectStyleLabel);
    let label_w = ui
        .painter()
        .layout_no_wrap(label.to_string(), font(13.0, Weight::Regular), t.muted)
        .size()
        .x;
    let id = egui::Id::unique("takeui-direct-choice");
    let choice_w = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(330.0);
    let one_row = area.width() >= label_w + 12.0 + choice_w;
    let below = if one_row { 28.0 } else { 16.0 + 4.0 + 28.0 };
    let preview_h = (ui.available_rect_before_wrap().height() - 16.0 - below).max(120.0);
    preview(ui, t, home, shared, vec2(area.width(), preview_h));
    ui.add_space(16.0 - 4.0);
    let choice = |ui: &mut Ui, shared: &mut Shared| {
        let before = shared.settings.quick_style;
        let r = kit::segmented(
            ui,
            t,
            &mut shared.settings.quick_style,
            &[
                (QuickStyle::Full, tr(Msg::StyleFull)),
                (QuickStyle::CornersOnly, tr(Msg::StyleCorners)),
                (QuickStyle::Plain, tr(Msg::StyleNone)),
            ],
        );
        if shared.settings.quick_style != before {
            shared.settings_dirty = true;
        }
        r.rect.width()
    };
    let label_text = RichText::new(label)
        .font(font(13.0, Weight::Regular))
        .color(t.muted);
    let w = if one_row {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.label(label_text);
            choice(ui, shared)
        })
        .inner
    } else {
        ui.label(label_text);
        choice(ui, shared)
    };
    ui.ctx().data_mut(|d| d.insert_temp(id, w));
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

/// A collapsible section's header; remembers whether it is open (open at first).
fn section(ui: &mut Ui, t: &Tokens, key: &str, title: &str) -> (bool, egui::Response) {
    let id = egui::Id::unique(("takeui-style-section", key));
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(true);
    let resp = kit::section_header(ui, t, title, &mut open);
    ui.data_mut(|d| d.insert_temp(id, open));
    (open, resp)
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

/// Background, frame and watermark; true when one of them changed.
pub fn controls(ui: &mut Ui, t: &Tokens, shared: &mut Shared, acts: &mut Vec<MainAction>) -> bool {
    let before = serde_json::to_string(&(&shared.settings.style, &shared.settings.watermark)).ok();
    // The design's 22 px between rows, and a line between sections.
    ui.spacing_mut().item_spacing = vec2(6.0, 22.0);
    let rule = |ui: &mut Ui| {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, t.border);
    };

    let (open, _) = section(ui, t, "background", tr(Msg::Background));
    if open {
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

    rule(ui);
    let (open, _) = section(ui, t, "frame", tr(Msg::Frame));
    if open {
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

    rule(ui);
    let (open, header) = section(ui, t, "watermark", tr(Msg::Watermark));
    let wm = &mut shared.settings.watermark;
    let at = Rect::from_center_size(
        egui::pos2(header.rect.right() - 22.0, header.rect.center().y),
        vec2(36.0, 20.0),
    );
    ui.put(at, |ui: &mut Ui| kit::switch(ui, t, &mut wm.enabled));
    if open {
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
    use crate::skins::takeui::testing::{click, content_box, painted_at, run, run_painted};
    use crate::system::instance::Page;
    use crate::system::settings::QuickStyle;
    use crate::ui::i18n::{Lang, Msg, every, lang, lang_lock, set_lang};

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
    fn the_page_fits_in_its_box_at_the_minimum_size() {
        let _lang = lang_lock();
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

    #[test]
    fn the_direct_style_choice_is_saved() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Style);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let corners = painted_at(&painted, &every(Msg::StyleCorners), |_| true)
            .expect("the rounded-corners-only choice");
        shared.settings_dirty = false;
        run(
            &mut home,
            &mut shared,
            (1160.0, 740.0),
            true,
            click(corners),
        );
        assert_eq!(shared.settings.quick_style, QuickStyle::CornersOnly);
        assert!(shared.settings_dirty);
    }
}
