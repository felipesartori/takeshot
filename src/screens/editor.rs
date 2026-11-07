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
use crate::ui::i18n::{Msg, t};
use crate::ui::kit::{self, BtnKind};
use crate::{Shared, Switch};
use egui::{Color32, Key, Modifiers, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use image::RgbaImage;

/// The answers of the "Discard changes?" dialog.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CloseChoice {
    Discard,
    Cancel,
    CopyAndClose,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CloseStep {
    Close,
    Stay,
}

/// Whether a close request from the window must be held back to ask first. Once the user chose to
/// close (`closing`), the final request goes through.
fn hold_close(requested: bool, closing: bool, unsaved: bool) -> bool {
    requested && !closing && unsaved
}

/// Edits that were not copied or saved: object changes since the last export, or text being typed.
fn unsaved_edits(an: &Annotator, saved_rev: Option<u64>) -> bool {
    saved_rev != Some(an.doc.revision()) || an.draft.as_ref().is_some_and(|d| !d.text.is_empty())
}

/// What an answer does; `copy` runs only for Copy and close, which closes only when it worked.
fn after_answer(answer: CloseChoice, copy: impl FnOnce() -> bool) -> CloseStep {
    match answer {
        CloseChoice::Discard => CloseStep::Close,
        CloseChoice::Cancel => CloseStep::Stay,
        CloseChoice::CopyAndClose if copy() => CloseStep::Close,
        CloseChoice::CopyAndClose => CloseStep::Stay,
    }
}

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
    /// Document revision at the last successful copy or save; `None` until the first one when the
    /// picture arrived with objects (they were never exported either).
    saved_rev: Option<u64>,
    asking_close: bool,
    /// Set once the user chose to close, so the window's own close request goes through.
    closing: bool,
}

impl Editor {
    pub fn new(_ctx: &egui::Context, base: RgbaImage, doc: Doc, shared: &Shared) -> Self {
        let mut an = Annotator::with_doc(shared.settings.draw.last_tool, doc);
        let first = an.bake().unwrap_or_default();
        let annotated = render::render_annotations(&base, &first, &shared.fonts);
        let saved_rev = an.doc.objs().is_empty().then(|| an.doc.revision());
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
            saved_rev,
            asking_close: false,
            closing: false,
        }
    }

    fn unsaved(&self) -> bool {
        unsaved_edits(&self.an, self.saved_rev)
    }

    /// Lets the next close through without asking (the screenshot self-check closes on its own).
    pub fn force_close(&mut self) {
        self.closing = true;
    }

    /// Closing asks first when there are unsaved edits; otherwise it closes.
    fn request_close(&mut self) -> Switch {
        if self.unsaved() {
            self.asking_close = true;
            Switch::None
        } else {
            self.closing = true;
            Switch::Close
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
        // The window X: keep the window open and ask when there are unsaved edits.
        if hold_close(
            ctx.input(|i| i.viewport().close_requested()),
            self.closing,
            self.unsaved(),
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.asking_close = true;
        }
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
                if let Some(tool) = self.an.bar_tool() {
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
                if !typing && !self.asking_close && !ctx.egui_wants_keyboard_input() {
                    self.an.handle_object_keys(ui);
                    widgets::tool_shortcuts(ui, &mut self.an, &mut shared.settings.draw, true);
                    let cmd = Modifiers::COMMAND;
                    self.an.handle_history_keys(ui);
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
                        switch = self.request_close();
                    }
                }
            });
        if self.asking_close {
            let theme = crate::ui::tokens::tokens(
                shared.settings.ui_theme,
                crate::ui::tokens::effective_dark(shared.settings.ui_mode, ctx.system_theme()),
            );
            let answer = kit::confirm_dialog(
                &ctx,
                &theme,
                "editor-discard",
                t(Msg::DiscardTitle),
                t(Msg::DiscardBody),
                &[
                    (t(Msg::Discard), BtnKind::Danger, CloseChoice::Discard),
                    (t(Msg::Cancel), BtnKind::Secondary, CloseChoice::Cancel),
                    (
                        t(Msg::CopyAndClose),
                        BtnKind::Primary,
                        CloseChoice::CopyAndClose,
                    ),
                ],
                CloseChoice::Cancel,
            );
            if let Some(choice) = answer {
                self.asking_close = false;
                if after_answer(choice, || self.copy(shared)) == CloseStep::Close {
                    self.closing = true;
                    switch = Switch::Close;
                }
            }
        }
        switch
    }

    /// Copies the final picture; true when it reached the clipboard.
    fn copy(&mut self, shared: &mut Shared) -> bool {
        self.an.commit_draft(&shared.settings.draw);
        self.sync_now(shared);
        let img = self.final_image(shared);
        match crate::system::actions::copy_image(&img) {
            Ok(()) => {
                self.saved_rev = Some(self.an.doc.revision());
                shared.toast("Imagem copiada".into());
                true
            }
            Err(e) => {
                shared.toast(format!("Não consegui copiar: {e}"));
                false
            }
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
                self.saved_rev = Some(self.an.doc.revision());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::annotator::TextDraft;

    #[test]
    fn the_window_x_is_held_only_for_unsaved_edits_and_not_when_closing() {
        assert!(hold_close(true, false, true));
        assert!(
            !hold_close(true, true, true),
            "the final close must go through"
        );
        assert!(!hold_close(true, false, false));
        assert!(!hold_close(false, false, true));
    }

    #[test]
    fn typed_text_and_object_changes_count_as_unsaved() {
        let mut an = Annotator::with_doc(Tool::Select, Doc::new());
        let saved = Some(an.doc.revision());
        assert!(!unsaved_edits(&an, saved));
        assert!(unsaved_edits(&an, None), "never exported");
        an.draft = Some(TextDraft {
            pos: pos2(0.0, 0.0),
            text: "hi".into(),
            started: 0.0,
        });
        assert!(unsaved_edits(&an, saved), "text being typed");
        an.draft = None;
        an.commit(Ann {
            shape: Shape::Rect(pos2(0.0, 0.0), pos2(50.0, 50.0)),
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: false,
        });
        assert!(unsaved_edits(&an, saved), "an object was added");
    }

    #[test]
    fn copy_and_close_closes_only_when_the_copy_worked() {
        let copies = std::cell::Cell::new(0);
        let copy = |ok: bool| {
            let copies = &copies;
            move || {
                copies.set(copies.get() + 1);
                ok
            }
        };
        assert_eq!(
            after_answer(CloseChoice::Discard, copy(true)),
            CloseStep::Close
        );
        assert_eq!(
            after_answer(CloseChoice::Cancel, copy(true)),
            CloseStep::Stay
        );
        assert_eq!(
            after_answer(CloseChoice::CopyAndClose, copy(true)),
            CloseStep::Close
        );
        assert_eq!(
            after_answer(CloseChoice::CopyAndClose, copy(false)),
            CloseStep::Stay
        );
        assert_eq!(copies.get(), 2, "only Copy and close copies");
    }
}
