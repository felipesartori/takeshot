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
use egui::{Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Stroke, Ui, vec2};

/// Width of the controls column.
const COLUMN: f32 = 300.0;

pub fn style_page(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
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
    ui.add_space(16.0);
    let avail = ui.available_size();
    ui.horizontal_top(|ui| {
        let left = (avail.x - COLUMN - 20.0).max(240.0);
        ui.vertical(|ui| {
            ui.set_width(left);
            preview(ui, t, home, shared, vec2(left, (avail.y - 44.0).max(160.0)));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(Msg::DirectStyleLabel))
                        .font(font(12.0, Weight::Regular))
                        .color(t.muted),
                );
                let before = shared.settings.quick_style;
                kit::segmented(
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
            });
        });
        ui.add_space(20.0);
        ui.vertical(|ui| {
            ui.set_width(COLUMN);
            egui::ScrollArea::vertical()
                .id_salt("takeui-style-column")
                .max_height(avail.y)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if controls(ui, t, shared, acts) {
                        shared.settings_dirty = true;
                    }
                });
        });
    });
}

/// The preview on the canvas colour, the picture fitted with a margin.
fn preview(ui: &mut Ui, t: &Tokens, home: &mut Home, shared: &Shared, size: egui::Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(10), t.canvas);
    p.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, t.border),
        egui::StrokeKind::Inside,
    );
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
    ui.spacing_mut().item_spacing = vec2(6.0, 8.0);

    let (open, _) = section(ui, t, "background", tr(Msg::Background));
    if open {
        let s = &mut shared.settings.style;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
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

    ui.add_space(4.0);
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

    ui.add_space(4.0);
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
    use crate::skins::takeui::testing::{click, painted_at, run};
    use crate::system::instance::Page;
    use crate::system::settings::QuickStyle;
    use crate::ui::i18n::{Msg, every};

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
