//! Editor: the captured picture on a styled background, with drawing tools and export.

use crate::engine::annotator::{Annotator, View, text_measure};
use crate::engine::cursor::{self, Area, CursorCtx};
use crate::engine::doc::{Doc, ObjId};
use crate::engine::model::*;
use crate::engine::render::{self, Layout};
use crate::screens::icons::Icon;
use crate::screens::overlay::color_image;
use crate::screens::widgets::{self, ACCENT, BG, MUTED, TEXT};
use crate::system::settings::Background;
use crate::system::settings::DrawOpts;
use crate::ui::i18n::{Msg, t};
use crate::ui::kit::WindowControl;
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

/// A zoom request: fit the window, a percentage, or the next step in or out.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Zoom {
    Fit,
    Percent(u32),
    In,
    Out,
}

/// The zoom steps, as scales.
const ZOOM_STEPS: [f32; 8] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0];

/// The scale a zoom request leads to from `current`; `fit` is the scale that shows it all.
pub fn zoom_scale(z: Zoom, current: f32, fit: f32) -> f32 {
    let (lo, hi) = (ZOOM_STEPS[0], ZOOM_STEPS[ZOOM_STEPS.len() - 1]);
    match z {
        Zoom::Fit => fit,
        Zoom::Percent(p) => (p as f32 / 100.0).clamp(lo, hi),
        Zoom::In => ZOOM_STEPS
            .iter()
            .copied()
            .find(|s| *s > current + 1e-3)
            .unwrap_or(hi),
        Zoom::Out => ZOOM_STEPS
            .iter()
            .rev()
            .copied()
            .find(|s| *s < current - 1e-3)
            .unwrap_or(lo),
    }
}

/// The tabs of the editor's right panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PanelTab {
    #[default]
    Object,
    Style,
}

/// What the user asked for in the editor; a skin returns these and `Editor::apply` carries them out.
#[derive(Clone, PartialEq, Debug)]
pub enum EditorAction {
    Undo,
    Redo,
    Copy,
    Save,
    SaveAs,
    Pin,
    NewCapture,
    SetTool(Tool),
    SetColor([u8; 4]),
    SetWidth(f32),
    /// The alpha of the colour, 0 to 255.
    SetOpacity(u8),
    SetFill(bool),
    SetFont(usize),
    SetTextBox(bool),
    SetBlurMode(BlurMode),
    Zoom(Zoom),
    /// Select an object; true adds it to the selection.
    Select(ObjId, bool),
    Hide(ObjId, bool),
    Duplicate,
    Delete,
    ToFront,
    ToBack,
    Tab(PanelTab),
    /// The style (background, frame, watermark) changed in the panel.
    StyleChanged,
    Window(WindowControl),
}

/// The tool's name, as the bar's tooltip and the object list show it.
pub fn tool_name(t: Tool) -> Msg {
    match t {
        Tool::Select => Msg::ToolSelect,
        Tool::Pen => Msg::ToolPen,
        Tool::Line => Msg::ToolLine,
        Tool::Arrow => Msg::ToolArrow,
        Tool::Rect => Msg::ToolRect,
        Tool::Ellipse => Msg::ToolEllipse,
        Tool::Marker => Msg::ToolMarker,
        Tool::Text => Msg::ToolText,
        Tool::Blur => Msg::ToolBlur,
        Tool::Counter => Msg::ToolStep,
    }
}

/// The status bar's hint for a tool.
pub fn tool_hint(t: Tool) -> Msg {
    match t {
        Tool::Select => Msg::HintSelect,
        Tool::Pen | Tool::Marker => Msg::HintStraight,
        Tool::Line | Tool::Arrow => Msg::HintAngles,
        Tool::Rect => Msg::HintSquare,
        Tool::Ellipse => Msg::HintCircle,
        Tool::Text => Msg::HintText,
        Tool::Blur => Msg::HintBlur,
        Tool::Counter => Msg::HintStep,
    }
}

pub struct Editor {
    base: RgbaImage,
    annotated: RgbaImage,
    pub(crate) an: Annotator,
    /// The zoom chosen; `None` fits the picture to the canvas.
    zoom: Option<f32>,
    /// The scale that fits the canvas, from the last layout.
    fit: f32,
    /// How far the picture is moved from the centre when it is larger than the canvas.
    pan: egui::Vec2,
    tab: PanelTab,
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
            zoom: None,
            fit: 1.0,
            pan: egui::Vec2::ZERO,
            tab: PanelTab::Object,
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

    /// The zoom chosen, or `None` when the picture fits the canvas.
    pub fn zoom(&self) -> Option<f32> {
        self.zoom
    }

    pub fn tab(&self) -> PanelTab {
        self.tab
    }

    /// Carries out what the user asked for. `frame` parents the save dialog when there is one.
    pub fn apply(
        &mut self,
        a: EditorAction,
        ctx: &egui::Context,
        frame: Option<&eframe::Frame>,
        shared: &mut Shared,
    ) -> Switch {
        let restyle = |ed: &mut Self, shared: &mut Shared, f: &dyn Fn(&mut DrawOpts)| {
            let before = shared.settings.draw.clone();
            f(&mut shared.settings.draw);
            ed.an.restyle_from_opts(&before, &shared.settings.draw);
            shared.settings_dirty = true;
        };
        match a {
            EditorAction::Undo => self.an.undo(),
            EditorAction::Redo => self.an.redo(),
            EditorAction::Copy => {
                self.copy(shared);
            }
            EditorAction::Save => self.save(frame, shared, false),
            EditorAction::SaveAs => self.save(frame, shared, true),
            EditorAction::Pin => {
                let img = self.final_image(shared);
                match crate::system::actions::temp_png(&img) {
                    Some(p) => {
                        let _ =
                            crate::system::actions::spawn_self(&["--pin", &p.to_string_lossy()]);
                    }
                    None => shared.toast(crate::ui::i18n::tf(Msg::CannotPin, &[""])),
                }
            }
            EditorAction::NewCapture => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                let _ = crate::system::actions::spawn_self(&["--capture", "--wait"]);
            }
            EditorAction::SetTool(tool) => {
                self.an.set_tool(tool, &mut shared.settings.draw);
                shared.settings_dirty = true;
            }
            EditorAction::SetColor(c) => restyle(self, shared, &|o| {
                o.color = [c[0], c[1], c[2], o.color[3]];
            }),
            EditorAction::SetWidth(w) => restyle(self, shared, &|o| o.width = w),
            EditorAction::SetOpacity(alpha) => restyle(self, shared, &|o| o.color[3] = alpha),
            EditorAction::SetFill(on) => restyle(self, shared, &|o| o.filled = on),
            EditorAction::SetFont(f) => restyle(self, shared, &|o| o.font = f),
            EditorAction::SetTextBox(on) => restyle(self, shared, &|o| o.text_box = on),
            EditorAction::SetBlurMode(m) => restyle(self, shared, &|o| o.blur_mode = m),
            EditorAction::Zoom(z) => {
                self.zoom = match z {
                    Zoom::Fit => None,
                    _ => Some(zoom_scale(z, self.zoom.unwrap_or(self.fit), self.fit)),
                };
                self.pan = egui::Vec2::ZERO;
                self.preview_dirty = true;
            }
            EditorAction::Select(id, add) => self.an.doc.select(Some(id), add),
            EditorAction::Hide(id, hidden) => self.an.doc.set_hidden(id, hidden),
            EditorAction::Duplicate => {
                self.an.doc.duplicate_selected(egui::vec2(16.0, 16.0));
            }
            EditorAction::Delete => self.an.doc.delete_selected(),
            EditorAction::ToFront => self.an.doc.bring_to_front(),
            EditorAction::ToBack => self.an.doc.send_to_back(),
            EditorAction::Tab(tab) => self.tab = tab,
            EditorAction::StyleChanged => {
                self.preview_dirty = true;
                shared.settings_dirty = true;
            }
            EditorAction::Window(WindowControl::Close) => return self.request_close(),
            EditorAction::Window(c) => crate::ui::chrome::act(ctx, c),
        }
        Switch::None
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
        let mut acts: Vec<EditorAction> = Vec::new();

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
                    if widgets::icon_button(ui, Icon::Undo, false, t(Msg::Undo)).clicked() {
                        acts.push(EditorAction::Undo);
                    }
                    if widgets::icon_button(ui, Icon::Redo, false, t(Msg::Redo)).clicked() {
                        acts.push(EditorAction::Redo);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let copy = egui::Button::new(
                            RichText::new(t(Msg::Copy)).color(Color32::WHITE).strong(),
                        )
                        .fill(ACCENT)
                        .corner_radius(8)
                        .min_size(vec2(84.0, 32.0));
                        if ui.add(copy).clicked() {
                            acts.push(EditorAction::Copy);
                        }
                        if ui
                            .add(egui::Button::new(t(Msg::Save)).min_size(vec2(72.0, 32.0)))
                            .clicked()
                        {
                            acts.push(EditorAction::Save);
                        }
                        if widgets::icon_button(ui, Icon::Folder, false, t(Msg::SaveAs)).clicked() {
                            acts.push(EditorAction::SaveAs);
                        }
                        if widgets::icon_button(ui, Icon::Pin, false, t(Msg::PinOnScreen)).clicked()
                        {
                            acts.push(EditorAction::Pin);
                        }
                        if widgets::icon_button(ui, Icon::Full, false, t(Msg::NewCapture)).clicked()
                        {
                            acts.push(EditorAction::NewCapture);
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
                        acts.push(EditorAction::Copy);
                    }
                    if ui.input_mut(|i| i.consume_key(cmd | Modifiers::SHIFT, Key::S)) {
                        acts.push(EditorAction::SaveAs);
                    } else if ui.input_mut(|i| i.consume_key(cmd, Key::S)) {
                        acts.push(EditorAction::Save);
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
        for a in acts {
            if let s @ Switch::Close = self.apply(a, &ctx, Some(frame), shared) {
                switch = s;
            }
        }
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
                &mut |_| {},
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
                shared.toast(t(Msg::ImageCopied).to_string());
                true
            }
            Err(e) => {
                shared.toast(crate::ui::i18n::tf(Msg::CouldNotCopy, &[&e.to_string()]));
                false
            }
        }
    }

    fn save(&mut self, frame: Option<&eframe::Frame>, shared: &mut Shared, ask: bool) {
        self.an.commit_draft(&shared.settings.draw);
        self.sync_now(shared);
        let path = if shared.settings.auto_save && !ask {
            Some(crate::system::actions::auto_path(&shared.settings.save_dir))
        } else {
            crate::system::actions::ask_save_path(&shared.settings.save_dir, frame)
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
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                shared.toast(crate::ui::i18n::tf(Msg::SavedFile, &[&name]));
            }
            Err(e) => shared.toast(crate::ui::i18n::tf(Msg::CouldNotSave, &[&e.to_string()])),
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
    fn zoom_steps_and_fit() {
        let fit = 0.37;
        assert_eq!(zoom_scale(Zoom::Percent(100), 0.5, fit), 1.0);
        assert_eq!(zoom_scale(Zoom::Fit, 2.0, fit), fit);
        assert_eq!(zoom_scale(Zoom::In, 1.0, fit), 1.5);
        assert_eq!(zoom_scale(Zoom::In, 0.6, fit), 0.75, "to the next step");
        assert_eq!(zoom_scale(Zoom::Out, 0.6, fit), 0.5, "to the step below");
        assert_eq!(zoom_scale(Zoom::Out, 0.25, fit), 0.25, "the smallest step");
        assert_eq!(zoom_scale(Zoom::In, 4.0, fit), 4.0, "the largest step");
    }

    fn editor() -> (Editor, crate::Shared) {
        let shared = crate::Shared::for_tests();
        let base = RgbaImage::from_pixel(200, 100, image::Rgba([240, 240, 240, 255]));
        let doc = Doc::from_anns(vec![Ann {
            shape: Shape::Rect(pos2(10.0, 10.0), pos2(60.0, 50.0)),
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: false,
        }]);
        let ed = Editor::new(&egui::Context::default(), base, doc, &shared);
        (ed, shared)
    }

    #[test]
    fn apply_routes_history_and_tools() {
        let ctx = egui::Context::default();
        let (mut ed, mut shared) = editor();
        ed.apply(EditorAction::SetTool(Tool::Rect), &ctx, None, &mut shared);
        assert_eq!(ed.an.tool, Tool::Rect);
        assert_eq!(shared.settings.draw.last_tool, Tool::Rect);
        ed.an.commit(Ann {
            shape: Shape::Ellipse(pos2(80.0, 10.0), pos2(120.0, 50.0)),
            color: [0, 0, 255, 255],
            width: 3.0,
            filled: false,
        });
        assert_eq!(ed.an.doc.objs().len(), 2);
        ed.apply(EditorAction::Undo, &ctx, None, &mut shared);
        assert_eq!(ed.an.doc.objs().len(), 1);
        ed.apply(EditorAction::Redo, &ctx, None, &mut shared);
        assert_eq!(ed.an.doc.objs().len(), 2);
        ed.apply(
            EditorAction::Zoom(Zoom::Percent(200)),
            &ctx,
            None,
            &mut shared,
        );
        assert_eq!(ed.zoom(), Some(2.0));
        ed.apply(EditorAction::Zoom(Zoom::Fit), &ctx, None, &mut shared);
        assert_eq!(ed.zoom(), None, "fit follows the window");
    }

    #[test]
    fn tool_names_follow_the_language() {
        let _lang = crate::ui::i18n::lang_lock();
        let before = crate::ui::i18n::lang();
        crate::ui::i18n::set_lang(crate::ui::i18n::Lang::En);
        assert_eq!(t(tool_name(Tool::Ellipse)), "Ellipse");
        crate::ui::i18n::set_lang(crate::ui::i18n::Lang::Pt);
        assert_eq!(t(tool_name(Tool::Ellipse)), "Elipse");
        assert_eq!(t(tool_name(Tool::Counter)), "Passo");
        crate::ui::i18n::set_lang(before);
    }

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
