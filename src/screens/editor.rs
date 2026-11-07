//! Editor: the captured picture on a styled background, with drawing tools and export.

use crate::engine::annotator::{Annotator, View, text_measure};
use crate::engine::cursor::{self, Area, CursorCtx};
use crate::engine::doc::Doc;
use crate::engine::model::*;
use crate::engine::render::{self, Layout};
use crate::screens::icons::Icon;
use crate::screens::overlay::color_image;
use crate::screens::widgets::{self, ACCENT, BG, MUTED, TEXT};
use crate::system::settings::Background;
use crate::{Shared, Switch};
use egui::{Color32, Key, Modifiers, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use image::RgbaImage;

pub struct Editor {
    base: RgbaImage,
    annotated: RgbaImage,
    an: Annotator,
    preview: Option<egui::TextureHandle>,
    preview_dirty: bool,
    preview_scale: f32,
    preview_src: Option<(u32, RgbaImage)>,
    lay: Option<Layout>,
    canvas_px: egui::Vec2,
    last_avail: egui::Vec2,
}

impl Editor {
    pub fn new(_ctx: &egui::Context, base: RgbaImage, anns: Vec<Ann>, shared: &Shared) -> Self {
        let mut an = Annotator::with_doc(shared.settings.draw.last_tool, Doc::from_anns(anns));
        let first = an.bake().unwrap_or_default();
        let annotated = render::render_annotations(&base, &first, &shared.fonts);
        Self {
            base,
            annotated,
            an,
            preview: None,
            preview_dirty: true,
            preview_scale: 1.0,
            preview_src: None,
            lay: None,
            canvas_px: vec2(1.0, 1.0),
            last_avail: vec2(0.0, 0.0),
        }
    }

    fn final_image(&self, shared: &Shared) -> RgbaImage {
        render::compose(
            &self.annotated,
            &shared.settings.style,
            &shared.settings.watermark,
            shared.logo.image.as_ref(),
            &shared.fonts,
            1.0,
        )
        .0
    }

    fn rebuild_preview(&mut self, ctx: &egui::Context, avail: egui::Vec2, shared: &Shared) {
        let ppp = ctx.pixels_per_point();
        let (iw, ih) = (
            self.annotated.width() as f32,
            self.annotated.height() as f32,
        );
        let full = render::layout(iw, ih, &shared.settings.style, &shared.settings.watermark);
        let s = ((avail.x * ppp) / full.w)
            .min((avail.y * ppp) / full.h)
            .clamp(0.02, 1.0);
        let tw = ((iw * s).round() as u32).max(1);
        let th = ((ih * s).round() as u32).max(1);
        let need = self
            .preview_src
            .as_ref()
            .map(|(w, _)| *w != tw)
            .unwrap_or(true);
        if need {
            let img = if tw == self.annotated.width() {
                self.annotated.clone()
            } else {
                image::imageops::resize(
                    &self.annotated,
                    tw,
                    th,
                    image::imageops::FilterType::CatmullRom,
                )
            };
            self.preview_src = Some((tw, img));
        }
        let src = &self.preview_src.as_ref().unwrap().1;
        let unit = src.width() as f32 / iw;
        let (img, lay) = render::compose(
            src,
            &shared.settings.style,
            &shared.settings.watermark,
            shared.logo.image.as_ref(),
            &shared.fonts,
            unit,
        );
        self.canvas_px = vec2(img.width() as f32, img.height() as f32);
        self.lay = Some(lay);
        self.preview_scale = unit;
        match &mut self.preview {
            Some(t) => t.set(color_image(&img), egui::TextureOptions::LINEAR),
            None => {
                self.preview = Some(ctx.load_texture(
                    "preview",
                    color_image(&img),
                    egui::TextureOptions::LINEAR,
                ))
            }
        }
        self.preview_dirty = false;
    }

    pub fn ui(&mut self, ui: &mut Ui, frame: &eframe::Frame, shared: &mut Shared) -> Switch {
        let ctx = ui.ctx().clone();
        if let Some(anns) = self.an.bake() {
            self.annotated = render::render_annotations(&self.base, &anns, &shared.fonts);
            self.preview_src = None;
            self.preview_dirty = true;
        }
        shared.logo.sync(&ctx, &shared.settings.watermark);

        // ------------------------------------------------ top bar
        egui::Panel::top("editor_top")
            .frame(
                egui::Frame::new()
                    .fill(widgets::PANEL)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(Stroke::new(1.0, widgets::BORDER)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    widgets::tool_buttons(ui, &mut self.an, &mut shared.settings.draw, true);
                    widgets::separator(ui);
                    if widgets::icon_button(ui, Icon::Undo, false, "Desfazer (Ctrl+Z)").clicked() {
                        self.an.undo();
                    }
                    if widgets::icon_button(ui, Icon::Redo, false, "Refazer (Ctrl+Y)").clicked() {
                        self.an.redo();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let copy = egui::Button::new(
                            RichText::new("Copiar").color(Color32::WHITE).strong(),
                        )
                        .fill(ACCENT)
                        .corner_radius(8)
                        .min_size(vec2(84.0, 32.0));
                        if ui
                            .add(copy)
                            .on_hover_text("Copiar a imagem final (Ctrl+C)")
                            .clicked()
                        {
                            self.copy(shared);
                        }
                        if ui
                            .add(egui::Button::new("Salvar").min_size(vec2(72.0, 32.0)))
                            .on_hover_text("Salvar PNG sem perdas (Ctrl+S)")
                            .clicked()
                        {
                            self.save(frame, shared, false);
                        }
                        if widgets::icon_button(
                            ui,
                            Icon::Folder,
                            false,
                            "Salvar como… (Ctrl+Shift+S)",
                        )
                        .clicked()
                        {
                            self.save(frame, shared, true);
                        }
                        if widgets::icon_button(ui, Icon::Pin, false, "Fixar na tela").clicked() {
                            let img = self.final_image(shared);
                            if let Some(p) = crate::system::actions::temp_png(&img) {
                                let _ = crate::system::actions::spawn_self(&[
                                    "--pin",
                                    &p.to_string_lossy(),
                                ]);
                            }
                        }
                        if widgets::icon_button(ui, Icon::Full, false, "Nova captura").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            let _ = crate::system::actions::spawn_self(&["--capture", "--wait"]);
                        }
                    });
                });
                // With Select the bar shows the picked object's options, and changing them restyles it.
                let bar_tool = if self.an.tool != Tool::Select {
                    Some(self.an.tool)
                } else {
                    self.an.doc.selected().first().and_then(|id| {
                        self.an
                            .doc
                            .objs()
                            .iter()
                            .find(|o| o.id == *id)
                            .map(|o| o.ann.shape.tool())
                    })
                };
                if let Some(tool) = bar_tool {
                    ui.add_space(4.0);
                    let before = shared.settings.draw.clone();
                    ui.horizontal(|ui| {
                        widgets::options_row(ui, tool, &mut shared.settings.draw, &shared.fonts)
                    });
                    self.an.restyle_from_opts(&before, &shared.settings.draw);
                }
            });

        // ------------------------------------------------ style sidebar
        egui::Panel::right("editor_style")
            .exact_size(318.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(widgets::PANEL)
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .stroke(Stroke::new(1.0, widgets::BORDER)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if widgets::style_panel(
                            ui,
                            &mut shared.settings.style,
                            &mut shared.settings.watermark,
                            &mut shared.logo,
                        ) {
                            self.preview_dirty = true;
                            shared.settings_dirty = true;
                        }
                        shared.logo.sync(&ctx, &shared.settings.watermark);
                        widgets::section(ui, "Exportar");
                        let (iw, ih) = (
                            self.annotated.width() as f32,
                            self.annotated.height() as f32,
                        );
                        let full = render::layout(
                            iw,
                            ih,
                            &shared.settings.style,
                            &shared.settings.watermark,
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} × {} px  ·  PNG sem perdas",
                                full.w as u32, full.h as u32
                            ))
                            .color(TEXT),
                        );
                        ui.label(
                            RichText::new("Resolução original, sem compressão com perda.")
                                .color(MUTED)
                                .size(12.0),
                        );
                        if ui.button("Restaurar estilo padrão").clicked() {
                            shared.settings.style = Default::default();
                            self.preview_dirty = true;
                            shared.settings_dirty = true;
                        }
                    });
            });

        // ------------------------------------------------ canvas
        let mut switch = Switch::None;
        egui::CentralPanel::no_frame()
            .frame(egui::Frame::new().fill(BG))
            .show(ui, |ui| {
                let avail_rect = ui.max_rect().shrink(28.0);
                let avail = avail_rect.size();
                if (avail - self.last_avail).length() > 1.0 {
                    self.last_avail = avail;
                    self.preview_dirty = true;
                }
                if self.preview_dirty && avail.x > 10.0 && avail.y > 10.0 {
                    self.rebuild_preview(&ctx, avail, shared);
                }
                let ppp = ctx.pixels_per_point();
                let size = self.canvas_px / ppp;
                let d = Rect::from_center_size(avail_rect.center(), size);
                let painter = ui.painter_at(ui.max_rect());
                if matches!(shared.settings.style.background, Background::None)
                    || shared.settings.style.radius > 0.0
                {
                    checker(&painter, d);
                }
                if let Some(t) = &self.preview {
                    painter.image(
                        t.id(),
                        d,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                let Some(lay) = self.lay else { return };
                let view = View {
                    origin: d.min + vec2(lay.img[0], lay.img[1]) / ppp,
                    k: self.preview_scale / ppp,
                };
                let img_rect = Rect::from_min_max(
                    view.to_screen(pos2(0.0, 0.0)),
                    view.to_screen(pos2(self.base.width() as f32, self.base.height() as f32)),
                );
                let resp = ui.interact(
                    img_rect.expand(40.0),
                    ui.scope_id().with("ed_canvas"),
                    Sense::click_and_drag(),
                );
                let typing = self.an.handle_typing(ui, &shared.settings.draw);
                let clip = ui.painter_at(img_rect.expand(40.0));
                {
                    // The measure borrows the fonts; the shortcuts below need `shared` mutably.
                    let measure = text_measure(&shared.fonts);
                    self.an
                        .handle_canvas(ui, &resp, &view, &measure, &mut shared.settings.draw);
                    if (resp.hovered() || self.an.dragging.is_some())
                        && let Some(p) = ui.input(|i| i.pointer.hover_pos())
                    {
                        // While a new shape is drawn, handles and objects underneath keep the pointer.
                        let (handle, over_object) = if self.an.live.is_some() {
                            (None, false)
                        } else {
                            self.an.hover(view.to_img(p), &view, &measure)
                        };
                        ctx.set_cursor_icon(cursor::pick(&CursorCtx {
                            tool: self.an.tool,
                            over_image: resp.rect.contains(p),
                            over_chrome: false,
                            handle,
                            over_object,
                            typing: self.an.draft.is_some(),
                            dragging: self.an.dragging.is_some(),
                            space_pan: false,
                            area: Area::NotApplicable,
                        }));
                    }
                    let theme = crate::ui::tokens::tokens(
                        shared.settings.ui_theme,
                        crate::ui::tokens::effective_dark(
                            shared.settings.ui_mode,
                            ctx.system_theme(),
                        ),
                    );
                    self.an
                        .paint_selection(&clip, &view, &theme, &shared.fonts, &measure);
                }
                self.an.paint_live(
                    &clip,
                    &view,
                    &shared.fonts,
                    &shared.settings.draw,
                    ui.input(|i| i.time),
                );
                if !typing && !ctx.egui_wants_keyboard_input() {
                    self.an.handle_object_keys(ui);
                    widgets::tool_shortcuts(ui, &mut self.an, &mut shared.settings.draw, true);
                    let cmd = Modifiers::COMMAND;
                    // Redo first: egui also matches Ctrl+Shift+Z against the plain Ctrl+Z pattern.
                    if ui.input_mut(|i| {
                        i.consume_key(cmd, Key::Y) || i.consume_key(cmd | Modifiers::SHIFT, Key::Z)
                    }) {
                        self.an.redo();
                    }
                    if ui.input_mut(|i| i.consume_key(cmd, Key::Z)) {
                        self.an.undo();
                    }
                    if ui.input_mut(|i| i.consume_key(cmd, Key::C)) {
                        self.copy(shared);
                    }
                    if ui.input_mut(|i| i.consume_key(cmd | Modifiers::SHIFT, Key::S)) {
                        self.save(frame, shared, true);
                    } else if ui.input_mut(|i| i.consume_key(cmd, Key::S)) {
                        self.save(frame, shared, false);
                    }
                    if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
                        && self.an.tool != Tool::Select
                    {
                        self.an.set_tool(Tool::Select, &mut shared.settings.draw);
                    }
                    if ui.input_mut(|i| i.consume_key(cmd, Key::W)) {
                        switch = Switch::Close;
                    }
                }
            });
        switch
    }

    fn copy(&mut self, shared: &mut Shared) {
        self.an.commit_draft(&shared.settings.draw);
        self.sync_now(shared);
        let img = self.final_image(shared);
        match crate::system::actions::copy_image(&img) {
            Ok(()) => shared.toast("Imagem copiada".into()),
            Err(e) => shared.toast(format!("Não consegui copiar: {e}")),
        }
    }

    fn save(&mut self, frame: &eframe::Frame, shared: &mut Shared, ask: bool) {
        self.an.commit_draft(&shared.settings.draw);
        self.sync_now(shared);
        let path = if shared.settings.auto_save && !ask {
            Some(crate::system::actions::auto_path(&shared.settings.save_dir))
        } else {
            crate::system::actions::ask_save_path(&shared.settings.save_dir, Some(frame))
        };
        let Some(path) = path else { return };
        let img = self.final_image(shared);
        match crate::system::actions::save_png(&img, &path) {
            Ok(()) => {
                if let Some(dir) = path.parent() {
                    shared.settings.save_dir = dir.to_string_lossy().into_owned();
                    shared.settings_dirty = true;
                }
                shared.toast(format!(
                    "Salvo: {}",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                ));
            }
            Err(e) => shared.toast(format!("Não consegui salvar: {e}")),
        }
    }

    fn sync_now(&mut self, shared: &Shared) {
        if let Some(anns) = self.an.bake() {
            self.annotated = render::render_annotations(&self.base, &anns, &shared.fonts);
            self.preview_src = None;
            self.preview_dirty = true;
        }
    }
}

fn checker(p: &egui::Painter, r: Rect) {
    let s = 14.0;
    p.rect_filled(r, 0.0, Color32::from_gray(52));
    let cols = (r.width() / s).ceil() as i32;
    let rows = (r.height() / s).ceil() as i32;
    if cols * rows > 12000 {
        return;
    }
    for i in 0..rows {
        for j in 0..cols {
            if (i + j) % 2 == 0 {
                let cell = Rect::from_min_size(
                    pos2(r.left() + j as f32 * s, r.top() + i as f32 * s),
                    vec2(s, s),
                )
                .intersect(r);
                p.rect_filled(cell, 0.0, Color32::from_gray(66));
            }
        }
    }
}
