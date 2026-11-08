//! The old look the windows not yet skinned still use (pin, notice): colours, the floating frame,
//! the toast; and the shared bits: tool shortcuts, hex colours, the watermark logo cache.

use crate::engine::annotator::Annotator;
use crate::engine::model::*;
use crate::system::settings::{DrawOpts, Watermark};
use egui::{Color32, CornerRadius, Margin, RichText, Sense, Stroke, Ui, vec2};

pub const BG: Color32 = Color32::from_rgb(14, 14, 16);
pub const PANEL: Color32 = Color32::from_rgb(21, 21, 24);
pub const RAISED: Color32 = Color32::from_rgb(30, 30, 34);
pub const HOVER: Color32 = Color32::from_rgb(40, 40, 46);
pub const BORDER: Color32 = Color32::from_rgb(44, 44, 51);
pub const TEXT: Color32 = Color32::from_rgb(232, 232, 234);
pub const MUTED: Color32 = Color32::from_rgb(140, 140, 150);
pub const ACCENT: Color32 = Color32::from_rgb(255, 90, 54);

pub fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = BG;
    v.faint_bg_color = RAISED;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    let w = &mut v.widgets;
    for (st, fill) in [
        (&mut w.noninteractive, PANEL),
        (&mut w.inactive, RAISED),
        (&mut w.hovered, HOVER),
        (&mut w.active, HOVER),
        (&mut w.open, HOVER),
    ] {
        st.bg_fill = fill;
        st.weak_bg_fill = fill;
        st.corner_radius = CornerRadius::same(8);
    }
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.bg_stroke = Stroke::NONE;
    w.hovered.fg_stroke = Stroke::new(1.2, Color32::WHITE);
    w.hovered.bg_stroke = Stroke::new(1.0, BORDER);
    w.active.fg_stroke = Stroke::new(1.2, Color32::WHITE);
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(10.0, 5.0);
        s.spacing.slider_width = 150.0;
        s.spacing.interact_size.y = 26.0;
        use egui::{FontFamily, FontId, TextStyle};
        s.text_styles
            .insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
        s.text_styles.insert(
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        );
        s.text_styles.insert(
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        );
        s.text_styles.insert(
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Name("bold".into())),
        );
    });
}

/// Frame used for floating bars over the screenshot.
pub fn floating_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(22, 22, 25, 245))
        .stroke(Stroke::new(1.0, Color32::from_rgb(52, 52, 60)))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(6, 6))
        .shadow(egui::Shadow {
            offset: [0, 6],
            blur: 22,
            spread: 0,
            color: Color32::from_black_alpha(110),
        })
}

/// Pick a tool by its key (P, L, A, R, O, M, T, B, N, V).
pub fn tool_shortcuts(ui: &mut Ui, an: &mut Annotator, opts: &mut DrawOpts, allow_select: bool) {
    let mods = ui.input(|i| i.modifiers);
    if mods.ctrl || mods.command || mods.alt {
        return;
    }
    for t in [Tool::Select].into_iter().chain(Tool::DRAWING) {
        if t == Tool::Select && !allow_select {
            continue;
        }
        if ui.input(|i| i.key_pressed(t.key())) {
            an.set_tool(t, opts);
        }
    }
}

pub fn hex(c: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub struct LogoCache {
    pub path: Option<String>,
    pub image: Option<image::RgbaImage>,
    pub thumb: Option<egui::TextureHandle>,
}

impl LogoCache {
    pub fn new() -> Self {
        Self {
            path: None,
            image: None,
            thumb: None,
        }
    }
    pub fn sync(&mut self, ctx: &egui::Context, wm: &Watermark) {
        if self.path != wm.logo_path {
            self.path = wm.logo_path.clone();
            self.image = wm
                .logo_path
                .as_deref()
                .and_then(crate::engine::render::load_logo);
            self.thumb = self.image.as_ref().map(|img| {
                let t = image::imageops::thumbnail(
                    img,
                    96,
                    (96.0 * img.height() as f32 / img.width().max(1) as f32).max(1.0) as u32,
                );
                ctx.load_texture(
                    "logo-thumb",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [t.width() as usize, t.height() as usize],
                        t.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        }
    }
}

/// Little toast message at the bottom of the window.
pub fn toast(ctx: &egui::Context, text: &str) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::unique("toast"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_BOTTOM, vec2(0.0, -28.0))
        .interactable(false)
        .show(ctx, |ui| {
            floating_frame()
                .inner_margin(Margin::symmetric(14, 9))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.0, ACCENT);
                        ui.label(RichText::new(text).color(TEXT));
                    });
                });
        });
    let _ = screen;
}
