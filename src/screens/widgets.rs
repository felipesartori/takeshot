//! The old look (capture overlay until plan 5b replaces it): theme, icon buttons, the tool bar
//! and the options bar.

use crate::engine::annotator::Annotator;
use crate::engine::model::*;
use crate::screens::icons::{self, Icon};
use crate::system::fonts::FontLib;
use crate::system::settings::{DrawOpts, Watermark};
use egui::{Color32, CornerRadius, Margin, RichText, Sense, Stroke, Ui, pos2, vec2};

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

pub fn icon_button(ui: &mut Ui, icon: Icon, selected: bool, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 8.0, ACCENT);
    } else if resp.hovered() {
        p.rect_filled(rect, 8.0, HOVER);
    }
    let col = if selected || resp.hovered() {
        Color32::WHITE
    } else {
        Color32::from_rgb(200, 200, 206)
    };
    icons::paint(p, rect.shrink(6.0), icon, col);
    resp.on_hover_text(tip)
}

pub fn separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, 32.0), Sense::hover());
    ui.painter().line_segment(
        [
            pos2(rect.center().x, rect.top() + 7.0),
            pos2(rect.center().x, rect.bottom() - 7.0),
        ],
        Stroke::new(1.0, BORDER),
    );
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

pub fn tool_tip(t: Tool) -> String {
    use crate::screens::editor::{tool_hint, tool_name};
    use crate::ui::i18n::t as tr;
    format!(
        "{} ({})\n{}",
        tr(tool_name(t)),
        t.key_name(),
        tr(tool_hint(t))
    )
}

/// The tool buttons. Returns true when something changed.
pub fn tool_buttons(ui: &mut Ui, an: &mut Annotator, opts: &mut DrawOpts, allow_select: bool) {
    if allow_select {
        if icon_button(
            ui,
            Icon::Select,
            an.tool == Tool::Select,
            &tool_tip(Tool::Select),
        )
        .clicked()
        {
            an.set_tool(Tool::Select, opts);
        }
        separator(ui);
    }
    for t in Tool::DRAWING {
        if icon_button(ui, icons::for_tool(t), an.tool == t, &tool_tip(t)).clicked() {
            an.set_tool(t, opts);
        }
    }
}

pub fn color_dot(ui: &mut Ui, c: [u8; 4], selected: bool, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let p = ui.painter();
    let center = rect.center();
    let r = size * 0.32;
    if selected {
        p.circle_stroke(center, r + 3.5, Stroke::new(1.5, Color32::WHITE));
    } else if resp.hovered() {
        p.circle_stroke(center, r + 3.0, Stroke::new(1.0, MUTED));
    }
    p.circle_filled(center, r, c32(c));
    if luminance(c) < 0.15 {
        p.circle_stroke(center, r, Stroke::new(1.0, Color32::from_gray(90)));
    }
    resp
}

/// Colour, width and the options of the current tool.
pub fn options_row(ui: &mut Ui, tool: Tool, opts: &mut DrawOpts, fonts: &FontLib) {
    ui.spacing_mut().item_spacing.x = 2.0;
    for c in PALETTE {
        if color_dot(ui, c, opts.color == c, 26.0)
            .on_hover_text(hex(c))
            .clicked()
        {
            opts.color = c;
        }
    }
    let mut c = c32(opts.color);
    ui.add_space(4.0);
    if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque)
        .on_hover_text("Cor personalizada")
        .changed()
    {
        opts.color = c.to_srgba_unmultiplied();
    }
    separator(ui);
    if !matches!(tool, Tool::Text) {
        for w in WIDTHS {
            let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 32.0), Sense::click());
            let sel = (opts.width - w).abs() < 0.01;
            if sel {
                ui.painter()
                    .rect_filled(rect.shrink2(vec2(1.0, 4.0)), 6.0, HOVER);
            }
            let d = (w * 0.9 + 2.0).min(14.0);
            ui.painter().circle_filled(
                rect.center(),
                d / 2.0,
                if sel {
                    Color32::WHITE
                } else {
                    Color32::from_gray(150)
                },
            );
            if resp
                .on_hover_text(format!("Espessura {w:.0} px · roda do mouse ajusta"))
                .clicked()
            {
                opts.width = w;
            }
        }
    }
    ui.spacing_mut().item_spacing.x = 6.0;
    match tool {
        Tool::Rect | Tool::Ellipse => {
            separator(ui);
            toggle_chip(ui, &mut opts.filled, "Preenchido");
        }
        Tool::Blur => {
            separator(ui);
            let mut pix = opts.blur_mode == BlurMode::Pixelate;
            if chip(ui, !pix, "Borrar").clicked() {
                pix = false;
            }
            if chip(ui, pix, "Pixelar").clicked() {
                pix = true;
            }
            opts.blur_mode = if pix {
                BlurMode::Pixelate
            } else {
                BlurMode::Blur
            };
        }
        Tool::Text => {
            separator(ui);
            let cur = fonts.get(opts.font);
            egui::ComboBox::from_id_salt("font_pick")
                .width(150.0)
                .selected_text(RichText::new(&cur.name).family(cur.family.clone()))
                .show_ui(ui, |ui| {
                    for (i, f) in fonts.list.iter().enumerate() {
                        ui.selectable_value(
                            &mut opts.font,
                            i,
                            RichText::new(format!("{}  Aa", f.name))
                                .family(f.family.clone())
                                .size(16.0),
                        );
                    }
                });
            ui.add(
                egui::DragValue::new(&mut opts.font_size)
                    .range(8.0..=240.0)
                    .speed(0.5)
                    .suffix(" px"),
            )
            .on_hover_text("Tamanho do texto");
            toggle_chip(ui, &mut opts.text_box, "Com fundo");
        }
        _ => {}
    }
}

pub fn chip(ui: &mut Ui, selected: bool, label: &str) -> egui::Response {
    let text = RichText::new(label)
        .size(12.5)
        .color(if selected { Color32::WHITE } else { MUTED });
    let b = egui::Button::new(text)
        .fill(if selected {
            ACCENT
        } else {
            Color32::TRANSPARENT
        })
        .stroke(if selected {
            Stroke::NONE
        } else {
            Stroke::new(1.0, BORDER)
        })
        .corner_radius(CornerRadius::same(7))
        .min_size(vec2(0.0, 26.0));
    ui.add(b)
}

pub fn toggle_chip(ui: &mut Ui, v: &mut bool, label: &str) {
    if chip(ui, *v, label).clicked() {
        *v = !*v;
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
