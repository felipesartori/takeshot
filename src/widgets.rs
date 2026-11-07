//! Look and feel: theme, icon buttons, the tool bar, the options bar and the style panel.

use crate::annotator::Annotator;
use crate::fonts::FontLib;
use crate::icons::{self, Icon};
use crate::model::*;
use crate::presets::{self, PRESETS};
use crate::settings::{Aspect, Background, DrawOpts, Style, Watermark, WmPos};
use egui::{Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2, vec2};

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
    match t.shift_hint() {
        Some(h) => format!("{} ({})\n{}", t.label(), t.key_name(), h),
        None => format!("{} ({})", t.label(), t.key_name()),
    }
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

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .size(11.0)
            .color(MUTED)
            .strong(),
    );
    ui.add_space(2.0);
}

/// Paint a gradient into a rect (approximated by a vertex-coloured mesh).
pub fn paint_gradient(
    p: &egui::Painter,
    rect: Rect,
    stops: &[(f32, [u8; 3])],
    angle: f32,
    rounding: f32,
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
            let c = presets::sample(stops, t_of(q));
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
        // mask the corners with the panel colour
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
            p.add(egui::Shape::convex_polygon(pts, PANEL, Stroke::NONE));
        }
    }
}

fn swatch(
    ui: &mut Ui,
    selected: bool,
    tip: &str,
    draw: impl FnOnce(&egui::Painter, Rect),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 30.0), Sense::click());
    let p = ui.painter();
    draw(p, rect);
    if selected {
        p.rect_stroke(
            rect.expand(2.5),
            9.0,
            Stroke::new(2.0, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
    } else if resp.hovered() {
        p.rect_stroke(
            rect.expand(2.0),
            9.0,
            Stroke::new(1.0, MUTED),
            egui::StrokeKind::Middle,
        );
    }
    resp.on_hover_text(tip)
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
            self.image = wm.logo_path.as_deref().and_then(crate::render::load_logo);
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

/// Background, frame and watermark controls. Returns true when anything changed.
pub fn style_panel(
    ui: &mut Ui,
    style: &mut Style,
    wm: &mut Watermark,
    logo: &mut LogoCache,
) -> bool {
    let before = (
        serde_json::to_string(style).ok(),
        serde_json::to_string(wm).ok(),
    );
    ui.spacing_mut().slider_width = 118.0;
    ui.set_max_width(ui.available_width());

    section(ui, "Fundo");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(9.0, 9.0);
        for (i, pr) in PRESETS.iter().enumerate() {
            let sel = style.background == Background::Preset(i);
            if swatch(ui, sel, pr.name, |p, r| {
                paint_gradient(p, r, pr.stops, pr.angle, 7.0)
            })
            .clicked()
            {
                style.background = Background::Preset(i);
            }
        }
        let (a, b) = (style.custom_a, style.custom_b);
        let stops = [(0.0, [a[0], a[1], a[2]]), (1.0, [b[0], b[1], b[2]])];
        if swatch(
            ui,
            style.background == Background::Custom,
            "Degradê personalizado",
            |p, r| {
                paint_gradient(p, r, &stops, style.custom_angle, 7.0);
                p.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    egui::FontId::proportional(16.0),
                    Color32::WHITE,
                );
            },
        )
        .clicked()
        {
            style.background = Background::Custom;
        }
        if swatch(
            ui,
            style.background == Background::Solid,
            "Cor sólida",
            |p, r| {
                p.rect_filled(r, 7.0, c32(style.solid));
            },
        )
        .clicked()
        {
            style.background = Background::Solid;
        }
        if swatch(
            ui,
            style.background == Background::Blurred,
            "Fundo com a própria captura borrada",
            |p, r| {
                paint_gradient(
                    p,
                    r,
                    &[
                        (0.0, [90, 98, 120]),
                        (0.5, [150, 140, 130]),
                        (1.0, [80, 110, 120]),
                    ],
                    120.0,
                    7.0,
                );
                for k in 0..3 {
                    p.circle_filled(
                        pos2(r.left() + 10.0 + k as f32 * 10.0, r.center().y),
                        5.0,
                        Color32::from_white_alpha(40),
                    );
                }
            },
        )
        .clicked()
        {
            style.background = Background::Blurred;
        }
        if swatch(
            ui,
            style.background == Background::None,
            "Sem fundo (transparente)",
            |p, r| {
                let s = 7.5;
                for i in 0..4 {
                    for j in 0..6 {
                        let c = if (i + j) % 2 == 0 {
                            Color32::from_gray(70)
                        } else {
                            Color32::from_gray(45)
                        };
                        let cell = Rect::from_min_size(
                            pos2(r.left() + j as f32 * s, r.top() + i as f32 * s),
                            Vec2::splat(s),
                        )
                        .intersect(r);
                        p.rect_filled(cell, 0.0, c);
                    }
                }
            },
        )
        .clicked()
        {
            style.background = Background::None;
        }
    });
    match style.background {
        Background::Custom => {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let mut a = c32(style.custom_a);
                let mut b = c32(style.custom_b);
                ui.label("Cores");
                if egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut a,
                    egui::color_picker::Alpha::Opaque,
                )
                .changed()
                {
                    style.custom_a = a.to_srgba_unmultiplied();
                }
                if egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut b,
                    egui::color_picker::Alpha::Opaque,
                )
                .changed()
                {
                    style.custom_b = b.to_srgba_unmultiplied();
                }
                if ui.small_button("⇄").on_hover_text("Inverter").clicked() {
                    std::mem::swap(&mut style.custom_a, &mut style.custom_b);
                }
            });
            ui.add(
                egui::Slider::new(&mut style.custom_angle, 0.0..=360.0)
                    .suffix("°")
                    .text("Ângulo"),
            );
        }
        Background::Solid => {
            ui.horizontal(|ui| {
                ui.label("Cor");
                let mut c = c32(style.solid);
                if egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut c,
                    egui::color_picker::Alpha::Opaque,
                )
                .changed()
                {
                    style.solid = c.to_srgba_unmultiplied();
                }
            });
        }
        Background::Preset(i)
            // start a custom gradient from the chosen preset
            if ui.small_button("Personalizar este degradê").clicked() => {
                let p = &PRESETS[i];
                let f = p.stops[0].1;
                let l = p.stops[p.stops.len() - 1].1;
                style.custom_a = [f[0], f[1], f[2], 255];
                style.custom_b = [l[0], l[1], l[2], 255];
                style.custom_angle = p.angle;
                style.background = Background::Custom;
            }
        _ => {}
    }

    section(ui, "Moldura");
    egui::Grid::new("frame_grid")
        .num_columns(2)
        .spacing(vec2(10.0, 8.0))
        .show(ui, |ui| {
            ui.label("Margem");
            ui.add(egui::Slider::new(&mut style.padding, 0.0..=25.0).suffix(" %"));
            ui.end_row();
            ui.label("Cantos");
            ui.add(egui::Slider::new(&mut style.radius, 0.0..=60.0).suffix(" px"));
            ui.end_row();
            ui.label("Sombra");
            ui.add(egui::Slider::new(&mut style.shadow, 0.0..=1.0).show_value(false));
            ui.end_row();
        });
    ui.label(RichText::new("Formato").color(MUTED).size(12.0));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for a in Aspect::ALL {
            if chip(ui, style.aspect == a, a.label()).clicked() {
                style.aspect = a;
            }
        }
        ui.add_space(6.0);
        toggle_chip(ui, &mut style.window_bar, "Barra de janela");
    });

    section(ui, "Marca d'água");
    ui.horizontal(|ui| {
        let label = if wm.enabled { "Ligada" } else { "Desligada" };
        toggle_chip(ui, &mut wm.enabled, label);
        if ui.button("Escolher logo…").clicked()
            && let Some(p) = rfd::FileDialog::new()
                .add_filter("Imagem", &["png", "jpg", "jpeg", "webp", "bmp"])
                .pick_file()
        {
            wm.logo_path = Some(p.to_string_lossy().into_owned());
            wm.enabled = true;
        }
        if wm.logo_path.is_some() && ui.small_button("Remover").clicked() {
            wm.logo_path = None;
        }
    });
    logo.sync(ui.ctx(), wm);
    if let Some(t) = &logo.thumb {
        let sz = t.size_vec2();
        let k = (40.0 / sz.y).min(160.0 / sz.x);
        ui.add(egui::Image::new(t).fit_to_exact_size(sz * k));
    } else if let Some(p) = &wm.logo_path {
        ui.colored_label(ACCENT, format!("Não consegui abrir {p}"));
    }
    ui.horizontal(|ui| {
        ui.label("Texto");
        ui.add(
            egui::TextEdit::singleline(&mut wm.text)
                .hint_text("@seu_nome ou site")
                .desired_width(170.0),
        );
    });
    ui.label(RichText::new("Posição").color(MUTED).size(12.0));
    ui.horizontal(|ui| {
        // 3×3 position picker drawn as a little canvas
        let (rect, _) = ui.allocate_exact_size(vec2(96.0, 66.0), Sense::hover());
        ui.painter().rect_filled(rect, 8.0, BG);
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(1.0, BORDER),
            egui::StrokeKind::Inside,
        );
        for (i, pos) in WmPos::GRID.iter().enumerate() {
            let (cx, cy) = ((i % 3) as f32, (i / 3) as f32);
            let c = pos2(
                rect.left() + 16.0 + cx * 32.0,
                rect.top() + 12.0 + cy * 21.0,
            );
            let r = Rect::from_center_size(c, vec2(22.0, 14.0));
            let resp = ui
                .interact(r, ui.scope_id().with(("wmpos", i)), Sense::click())
                .on_hover_text(pos.label());
            let sel = wm.position == *pos;
            ui.painter().rect_filled(
                r,
                4.0,
                if sel {
                    ACCENT
                } else if resp.hovered() {
                    HOVER
                } else {
                    RAISED
                },
            );
            if resp.clicked() {
                wm.position = *pos;
                wm.enabled = true;
            }
        }
        ui.vertical(|ui| {
            for pos in [WmPos::BandTop, WmPos::BandBottom] {
                if chip(ui, wm.position == pos, pos.label()).clicked() {
                    wm.position = pos;
                    wm.enabled = true;
                }
            }
        });
    });
    egui::Grid::new("wm_grid")
        .num_columns(2)
        .spacing(vec2(10.0, 8.0))
        .show(ui, |ui| {
            if wm.position.is_band() {
                ui.label("Cor da faixa");
                let mut c = c32(wm.band_color);
                if egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut c,
                    egui::color_picker::Alpha::Opaque,
                )
                .changed()
                {
                    wm.band_color = c.to_srgba_unmultiplied();
                }
                ui.end_row();
            } else {
                ui.label("Tamanho");
                ui.add(egui::Slider::new(&mut wm.size, 3.0..=40.0).suffix(" %"));
                ui.end_row();
                ui.label("Distância");
                ui.add(egui::Slider::new(&mut wm.margin, 0.0..=12.0).suffix(" %"));
                ui.end_row();
                ui.label("Cor do texto");
                let mut c = c32(wm.text_color);
                if egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut c,
                    egui::color_picker::Alpha::Opaque,
                )
                .changed()
                {
                    wm.text_color = c.to_srgba_unmultiplied();
                }
                ui.end_row();
            }
            ui.label("Opacidade");
            ui.add(egui::Slider::new(&mut wm.opacity, 0.1..=1.0).show_value(false));
            ui.end_row();
        });
    if !wm.position.is_band() {
        ui.checkbox(
            &mut wm.inside_image,
            "Posicionar sobre a captura (não no fundo)",
        );
    }

    let after = (
        serde_json::to_string(style).ok(),
        serde_json::to_string(wm).ok(),
    );
    before != after
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
