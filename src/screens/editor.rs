//! Editor: the captured picture on a styled background, with drawing tools and export. The state,
//! what the user asks for (`EditorAction`, `Editor::apply`) and the canvas (zoom, pan, drawing,
//! keys) live here; `views::editor` draws the bars and panels in either theme.

use crate::engine::annotator::{Annotator, View, text_measure};
use crate::engine::cursor::{self, Area, CursorCtx};
use crate::engine::doc::{Doc, ObjId};
use crate::engine::model::*;
use crate::engine::render::{self, Layout};
use crate::screens::overlay::color_image;
use crate::system::settings::{Background, DrawOpts};
use crate::ui::i18n::{Msg, t, t as tr};
use crate::ui::kit::{self, BtnKind, WindowControl};
use crate::ui::tokens::Tokens;
use crate::{Shared, Switch};
use egui::{Color32, Key, Modifiers, Rect, Sense, Ui, pos2, vec2};
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

/// The smallest zoom: a large capture can fit below the first step.
const ZOOM_MIN: f32 = 0.1;

/// The scale a zoom request leads to from `current`; `fit` is the scale that shows it all.
/// Zooming out below the first step keeps the zoom (it never enlarges).
pub fn zoom_scale(z: Zoom, current: f32, fit: f32) -> f32 {
    let hi = ZOOM_STEPS[ZOOM_STEPS.len() - 1];
    match z {
        Zoom::Fit => fit,
        Zoom::Percent(p) => (p as f32 / 100.0).clamp(ZOOM_MIN, hi),
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
            .unwrap_or(current.max(ZOOM_MIN)),
    }
}

/// How much of the Ctrl+wheel or pinch factor the zoom follows: egui's notch (e^0.25) is about
/// 13 %.
const WHEEL_GAIN: f32 = 0.5;

/// The scale after this frame's Ctrl+wheel or pinch factor `z`, in proportion to it (a notch
/// spread over frames adds up to the same), from 10 to 400 %; below 10 % (a big picture's fit)
/// zooming out never enlarges.
pub fn wheel_zoom(current: f32, z: f32) -> f32 {
    (current * z.powf(WHEEL_GAIN)).clamp(ZOOM_MIN.min(current), ZOOM_STEPS[ZOOM_STEPS.len() - 1])
}

/// After the last Ctrl+wheel zoom, how long the stretched picture shows before it is rendered
/// again at the new scale.
const ZOOM_REST: f64 = 0.15;

/// The tabs of the editor's right panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PanelTab {
    #[default]
    Object,
    Style,
}

/// What the user asked for in the editor; the editor view returns these and `Editor::apply` carries them out.
#[derive(Clone, PartialEq, Debug)]
pub enum EditorAction {
    Undo,
    Redo,
    Copy,
    Save,
    SaveAs,
    Pin,
    SetTool(Tool),
    SetColor([u8; 4]),
    SetWidth(f32),
    /// A rectangle's corner radius, in picture px.
    SetRadius(f32),
    /// The alpha of the colour, 0 to 255.
    SetOpacity(u8),
    SetFill(bool),
    SetFont(usize),
    SetFontSize(f32),
    SetTextBox(bool),
    SetBlurMode(BlurMode),
    /// A slider was grabbed (true) or let go: the changes in between undo in one step.
    Gesture(bool),
    Zoom(Zoom),
    /// Pick an object from the list: switches to Select and shows its style; true adds it to the
    /// selection.
    Select(ObjId, bool),
    Hide(ObjId, bool),
    Duplicate,
    Delete,
    ToFront,
    ToBack,
    Tab(PanelTab),
    /// The style (background, frame, watermark) changed in the panel.
    StyleChanged,
    /// Pick the watermark's logo file.
    ChooseLogo,
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

/// An object's name in the panel and the list: the tool that draws it, a text with its first
/// line, a numbered step with its number ("Text · Peak in October", "Step 2").
pub fn object_label(kind: Tool, text: Option<&str>) -> String {
    let name = t(tool_name(kind));
    match (kind, text) {
        (Tool::Counter, Some(n)) => format!("{name} {n}"),
        (_, Some(s)) if !s.trim().is_empty() => {
            format!("{name} · {}", s.lines().next().unwrap_or("").trim())
        }
        _ => name.to_string(),
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
    /// When Ctrl+wheel last zoomed (egui time); the preview waits `ZOOM_REST` after it.
    zoomed_at: f64,
    /// The plain wheel's part of a notch still to come (`wheel_notches`).
    wheel_acc: f32,
    tab: PanelTab,
    preview: Option<egui::TextureHandle>,
    preview_dirty: bool,
    preview_scale: f32,
    /// The scale the preview was last rendered for.
    preview_target: f32,
    /// The file name the title bar shows.
    name: String,
    /// How picture pixels map to the screen, and the canvas's rect, from the last frame drawn.
    view: Option<View>,
    canvas_area: Rect,
    preview_src: Option<(u32, RgbaImage)>,
    lay: Option<Layout>,
    canvas_px: egui::Vec2,
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
            zoomed_at: f64::NEG_INFINITY,
            wheel_acc: 0.0,
            tab: PanelTab::Object,
            preview: None,
            preview_dirty: true,
            preview_scale: 1.0,
            preview_target: 0.0,
            name: format!(
                "Takeshot {}.png",
                chrono::Local::now().format("%Y-%m-%d %H-%M-%S")
            ),
            view: None,
            canvas_area: Rect::NOTHING,
            preview_src: None,
            lay: None,
            canvas_px: vec2(1.0, 1.0),
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
            EditorAction::Undo => {
                self.an.undo();
                self.an.sync_options(&mut shared.settings.draw);
            }
            EditorAction::Redo => {
                self.an.redo();
                self.an.sync_options(&mut shared.settings.draw);
            }
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
            EditorAction::SetTool(tool) => {
                self.an.set_tool(tool, &mut shared.settings.draw);
                shared.settings_dirty = true;
            }
            EditorAction::SetColor(c) => restyle(self, shared, &|o| {
                o.color = [c[0], c[1], c[2], o.color[3]];
            }),
            EditorAction::SetWidth(w) => restyle(self, shared, &|o| o.width = w),
            EditorAction::SetRadius(r) => restyle(self, shared, &|o| o.radius = r),
            EditorAction::SetOpacity(alpha) => restyle(self, shared, &|o| o.color[3] = alpha),
            EditorAction::SetFill(on) => restyle(self, shared, &|o| o.filled = on),
            EditorAction::SetFont(f) => restyle(self, shared, &|o| o.font = f),
            EditorAction::SetFontSize(s) => restyle(self, shared, &|o| o.font_size = s),
            EditorAction::SetTextBox(on) => restyle(self, shared, &|o| o.text_box = on),
            EditorAction::SetBlurMode(m) => restyle(self, shared, &|o| o.blur_mode = m),
            EditorAction::Gesture(true) => self.an.doc.begin_gesture(),
            EditorAction::Gesture(false) => self.an.doc.end_gesture(),
            EditorAction::Zoom(z) => self.zoom_to(z, egui::Vec2::ZERO),
            EditorAction::Select(id, add) => {
                self.an.pick(id, add, &mut shared.settings.draw);
                shared.settings_dirty = true;
            }
            EditorAction::Hide(id, hidden) => self.an.doc.set_hidden(id, hidden),
            EditorAction::Duplicate => {
                self.an.doc.duplicate_selected(egui::vec2(12.0, 12.0));
            }
            EditorAction::Delete => self.an.doc.delete_selected(),
            EditorAction::ToFront => self.an.doc.bring_to_front(),
            EditorAction::ToBack => self.an.doc.send_to_back(),
            EditorAction::Tab(tab) => self.tab = tab,
            EditorAction::StyleChanged => {
                self.preview_dirty = true;
                shared.settings_dirty = true;
            }
            EditorAction::ChooseLogo => {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Logo", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
                {
                    shared.settings.watermark.logo_path = Some(p.to_string_lossy().into_owned());
                    shared.settings_dirty = true;
                    self.preview_dirty = true;
                }
            }
            EditorAction::Window(WindowControl::Close) => return self.request_close(),
            EditorAction::Window(c) => crate::ui::chrome::act(ctx, c),
        }
        Switch::None
    }

    /// Zooms keeping the picture's point at `anchor` (from the canvas centre) where it is on
    /// screen: the pointer for Ctrl+wheel, the centre for the keys and the zoom menu. Fit centres.
    fn zoom_to(&mut self, z: Zoom, anchor: egui::Vec2) {
        match z {
            Zoom::Fit => {
                self.zoom = None;
                self.pan = egui::Vec2::ZERO;
            }
            _ => self.zoom_at(zoom_scale(z, self.shown_scale(), self.fit), anchor),
        }
        self.preview_dirty = true;
    }

    /// Sets the scale to `scale`, keeping the picture's point at `anchor` (from the canvas
    /// centre) where it is on screen.
    fn zoom_at(&mut self, scale: f32, anchor: egui::Vec2) {
        let ratio = scale / self.shown_scale();
        self.zoom = Some(scale);
        self.pan = anchor - (anchor - self.pan) * ratio;
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

    /// Physical pixels per picture pixel that show the whole styled picture in `avail` points.
    fn fit_scale(&self, avail: egui::Vec2, ppp: f32, shared: &Shared) -> f32 {
        let full = render::layout(
            self.annotated.width() as f32,
            self.annotated.height() as f32,
            &shared.settings.style,
            &shared.settings.watermark,
        );
        ((avail.x * ppp) / full.w)
            .min((avail.y * ppp) / full.h)
            .clamp(0.02, 1.0)
    }

    /// Renders the styled preview at `s` physical pixels per picture pixel (at most 1; a larger
    /// zoom magnifies this texture).
    fn rebuild_preview(&mut self, ctx: &egui::Context, s: f32, shared: &Shared) {
        let (iw, ih) = (
            self.annotated.width() as f32,
            self.annotated.height() as f32,
        );
        let want = s.clamp(0.02, 1.0);
        // The styled preview stays within the GPU's texture side; a larger zoom magnifies it.
        let full = render::layout(iw, ih, &shared.settings.style, &shared.settings.watermark);
        let max_side = ctx.input(|i| i.max_texture_side) as f32;
        // A pixel of margin and rounding down: the composed picture rounds its own size up.
        let s = want.min((max_side - 1.0) / full.w.max(full.h));
        let tw = ((iw * s).floor() as u32).max(1);
        let th = ((ih * s).floor() as u32).max(1);
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
        self.preview_target = want;
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

    /// The file name the title bar shows.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    /// The picture's size in pixels.
    pub fn size(&self) -> [u32; 2] {
        [self.base.width(), self.base.height()]
    }

    /// The scale on screen: the zoom chosen, or the one that fits.
    pub fn shown_scale(&self) -> f32 {
        self.zoom.unwrap_or(self.fit)
    }

    pub fn pan(&self) -> egui::Vec2 {
        self.pan
    }

    /// How picture pixels map to the screen in the last frame drawn.
    pub fn view(&self) -> Option<View> {
        self.view
    }

    /// The canvas's rect in the last frame drawn.
    pub fn canvas_area(&self) -> Rect {
        self.canvas_area
    }

    /// The selection's bounds on screen, while nothing is being drawn or dragged.
    pub fn selection_on_screen(&self, shared: &Shared) -> Option<Rect> {
        if self.an.busy() {
            return None;
        }
        let view = self.view?;
        let b = self.an.selection_bounds(&text_measure(&shared.fonts))?;
        Some(Rect::from_min_max(
            view.to_screen(b.min),
            view.to_screen(b.max),
        ))
    }

    /// One frame of the editor: the view draws the bars and panels and calls `canvas` for the
    /// picture; what the user asked for is carried out here, and the discard question shows when
    /// a close waits on it.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        frame: &eframe::Frame,
        t: &Tokens,
        shared: &mut Shared,
    ) -> Switch {
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
        crate::ui::chrome::adopt_window(EDITOR_TITLE);
        let acts = crate::views::editor(ui, t, self, shared);
        let mut switch = Switch::None;
        for a in acts {
            if let s @ Switch::Close = self.apply(a, &ctx, Some(frame), shared) {
                switch = s;
            }
        }
        if self.asking_close {
            let answer = kit::confirm_dialog(
                &ctx,
                t,
                "editor-discard",
                tr(Msg::DiscardTitle),
                tr(Msg::DiscardBody),
                &mut |_| {},
                &[
                    (tr(Msg::Discard), BtnKind::Danger, CloseChoice::Discard),
                    (tr(Msg::Cancel), BtnKind::Secondary, CloseChoice::Cancel),
                    (
                        tr(Msg::CopyAndClose),
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

    /// The picture on its canvas, filling `ui`'s rect: zoom (Ctrl+wheel, Ctrl+0 fits, Ctrl+1 is
    /// 100 %), pan (Space+drag or the middle button), drawing and selecting, the pointer, and the
    /// editor's keys. Returns what the keys asked for.
    pub fn canvas(&mut self, ui: &mut Ui, t: &Tokens, shared: &mut Shared) -> Vec<EditorAction> {
        let ctx = ui.ctx().clone();
        let mut acts = Vec::new();
        let area = ui.max_rect();
        self.canvas_area = area;
        self.view = None;
        // A slider that stopped being drawn mid-drag never reports its release.
        if !ui.input(|i| i.pointer.any_down()) {
            self.an.doc.end_gesture();
        }
        ui.painter().rect_filled(area, 0.0, t.canvas);
        let room = area.shrink(28.0);
        let avail = room.size();
        if avail.x <= 10.0 || avail.y <= 10.0 {
            return acts;
        }
        let ppp = ctx.pixels_per_point();
        let fit = self.fit_scale(avail, ppp, shared);
        if (fit - self.fit).abs() > 1e-4 {
            self.fit = fit;
            if self.zoom.is_none() {
                self.preview_dirty = true;
            }
        }
        let shown = self.shown_scale();
        // While Ctrl+wheel zooms, the picture is stretched; it is rendered again at the new scale
        // once the zoom rests, so a 4K capture never renders every frame.
        let now = ui.input(|i| i.time);
        let since = now - self.zoomed_at;
        let behind = (shown.min(1.0) - self.preview_target).abs() > 1e-4;
        if self.preview_dirty || (behind && since >= ZOOM_REST) {
            self.rebuild_preview(&ctx, shown, shared);
        } else if behind {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(ZOOM_REST - since));
        }
        let size = self.canvas_px * (shown / self.preview_scale) / ppp;
        // A picture larger than the canvas moves within its edges; a smaller one stays centred.
        let slack = ((size - avail) * 0.5).max(egui::Vec2::ZERO);
        self.pan = self.pan.clamp(-slack, slack);
        let d = Rect::from_center_size(room.center() + self.pan, size);
        let painter = ui.painter_at(area);
        if matches!(shared.settings.style.background, Background::None)
            || shared.settings.style.radius > 0.0
        {
            checker(&painter, d);
        }
        if let Some(tex) = &self.preview {
            painter.image(
                tex.id(),
                d,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        let Some(lay) = self.lay else { return acts };
        let m = shown / self.preview_scale;
        let view = View {
            origin: d.min + vec2(lay.img[0], lay.img[1]) * m / ppp,
            k: shown / ppp,
        };
        self.view = Some(view);
        let img_rect = Rect::from_min_max(
            view.to_screen(pos2(0.0, 0.0)),
            view.to_screen(pos2(self.base.width() as f32, self.base.height() as f32)),
        );
        let resp = ui.interact(
            img_rect.expand(40.0).intersect(area),
            ui.scope_id().with("ed_canvas"),
            Sense::click_and_drag(),
        );
        let typing = self.an.handle_typing(
            ui,
            &view,
            &shared.fonts,
            &shared.settings.draw,
            tr(Msg::TypeHere),
        );
        // Space (not while typing) or the middle button drags the picture instead of drawing.
        let space = !typing && self.an.draft.is_none() && ui.input(|i| i.key_down(Key::Space));
        let middle = resp.dragged_by(egui::PointerButton::Middle);
        let panning = space || middle;
        if panning && resp.dragged() {
            self.pan += resp.drag_delta();
        }
        // The plain wheel changes a drawing tool's width (or the text size), a notch a step.
        if self.an.tool != Tool::Select && resp.hovered() {
            let events = ui.input(|i| i.events.clone());
            let n = crate::engine::annotator::wheel_notches(&mut self.wheel_acc, &events);
            if n != 0 {
                let before = shared.settings.draw.clone();
                crate::engine::annotator::wheel_restyle(self.an.tool, &mut shared.settings.draw, n);
                self.an.restyle_from_opts(&before, &shared.settings.draw);
                shared.settings_dirty = true;
            }
        }
        // Ctrl+wheel and the pinch zoom in proportion, around the pointer.
        let z = ui.input(|i| i.zoom_delta());
        if (z - 1.0).abs() > 1e-4
            && resp.hovered()
            && let Some(p) = ui.input(|i| i.pointer.hover_pos())
        {
            self.zoom_at(wheel_zoom(self.shown_scale(), z), p - room.center());
            self.zoomed_at = now;
        }
        let clip = ui.painter_at(img_rect.expand(40.0).intersect(area));
        {
            // The measure borrows the fonts; the shortcuts below need `shared` mutably.
            let measure = text_measure(&shared.fonts);
            if !panning {
                self.an
                    .handle_canvas(ui, &resp, &view, &measure, &mut shared.settings.draw);
            }
            if (resp.hovered() || resp.dragged() || self.an.dragging.is_some())
                && let Some(p) = ui.input(|i| i.pointer.hover_pos())
            {
                // While a new shape is drawn or the picture panned, handles and objects underneath
                // keep the pointer.
                let (handle, over_object, over_text) = if self.an.live.is_some() || panning {
                    (None, false, false)
                } else {
                    self.an.hover(view.to_img(p), &view, &measure)
                };
                ctx.set_cursor_icon(cursor::pick(&CursorCtx {
                    tool: self.an.tool,
                    over_image: resp.rect.contains(p),
                    over_chrome: false,
                    handle,
                    over_object,
                    over_text,
                    typing: self.an.draft.is_some(),
                    dragging: if panning {
                        resp.dragged()
                    } else {
                        self.an.dragging.is_some()
                    },
                    space_pan: panning,
                    area: Area::NotApplicable,
                }));
            }
            self.an
                .paint_selection(&clip, &view, t, &shared.fonts, &measure);
        }
        self.an.paint_live(&clip, &view, &shared.fonts);
        if !typing && !self.asking_close && !ctx.egui_wants_keyboard_input() {
            self.an.handle_object_keys(ui);
            if self.an.handle_history_keys(ui) {
                self.an.sync_options(&mut shared.settings.draw);
            }
            acts.extend(keys(ui, self.an.tool));
        }
        acts
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

/// The window title the editor's own bar adopts.
pub const EDITOR_TITLE: &str = "Takeshot · editor";

/// The editor's keys besides the canvas behaviour: copy, save, pin, close, zoom, Esc back to
/// Select, and the tool letters (only without Ctrl or Alt).
fn keys(ui: &Ui, tool: Tool) -> Vec<EditorAction> {
    let mut acts = Vec::new();
    let cmd = Modifiers::COMMAND;
    ui.input_mut(|i| {
        if i.consume_key(cmd, Key::C) {
            acts.push(EditorAction::Copy);
        }
        if i.consume_key(cmd | Modifiers::SHIFT, Key::S) {
            acts.push(EditorAction::SaveAs);
        } else if i.consume_key(cmd, Key::S) {
            acts.push(EditorAction::Save);
        }
        if i.consume_key(cmd, Key::T) {
            acts.push(EditorAction::Pin);
        }
        if i.consume_key(cmd, Key::W) {
            acts.push(EditorAction::Window(WindowControl::Close));
        }
        if i.consume_key(cmd, Key::Num0) {
            acts.push(EditorAction::Zoom(Zoom::Fit));
        }
        if i.consume_key(cmd, Key::Num1) {
            acts.push(EditorAction::Zoom(Zoom::Percent(100)));
        }
        if i.consume_key(cmd, Key::Plus) || i.consume_key(cmd, Key::Equals) {
            acts.push(EditorAction::Zoom(Zoom::In));
        }
        if i.consume_key(cmd, Key::Minus) {
            acts.push(EditorAction::Zoom(Zoom::Out));
        }
        if tool != Tool::Select && i.consume_key(Modifiers::NONE, Key::Escape) {
            acts.push(EditorAction::SetTool(Tool::Select));
        }
        let m = i.modifiers;
        if !(m.ctrl || m.command || m.alt) {
            for t in [Tool::Select].into_iter().chain(Tool::DRAWING) {
                if i.key_pressed(t.key()) {
                    acts.push(EditorAction::SetTool(t));
                }
            }
        }
    });
    acts
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

    #[test]
    fn zooming_out_never_enlarges() {
        // A 4K capture fits a small window at 20 %: the step "below" must not jump up to 25 %.
        assert_eq!(zoom_scale(Zoom::Out, 0.2, 0.2), 0.2);
        assert_eq!(
            zoom_scale(Zoom::Out, 1.0, 0.5),
            0.75,
            "from a step, the one below"
        );
        assert_eq!(zoom_scale(Zoom::Percent(10), 1.0, 0.5), 0.1, "down to 10 %");
        assert_eq!(zoom_scale(Zoom::Percent(5), 1.0, 0.5), 0.1);
        assert_eq!(zoom_scale(Zoom::In, 0.2, 0.2), 0.25);
    }

    #[test]
    fn the_wheel_zooms_in_proportion() {
        // A notch of Ctrl+wheel (e^0.25 in egui, spread over a few frames) is about 13 %, the
        // same whether it comes in one frame or several; a touchpad pinch goes on smoothly.
        let notch = 0.25f32.exp();
        let whole = wheel_zoom(1.0, notch);
        assert!((whole - 1.133).abs() < 0.01, "{whole}");
        let spread = [
            1.07,
            1.07,
            1.05,
            1.016,
            notch / (1.07 * 1.07 * 1.05 * 1.016),
        ]
        .iter()
        .fold(1.0, |s, z| wheel_zoom(s, *z));
        assert!((spread - whole).abs() < 1e-4, "{spread} vs {whole}");
        let back = wheel_zoom(whole, 1.0 / notch);
        assert!((back - 1.0).abs() < 1e-4, "a notch back undoes it");
        assert_eq!(wheel_zoom(3.9, 4.0), 4.0, "at most 400 %");
        assert_eq!(wheel_zoom(0.11, 0.1), 0.1, "at least 10 %");
        // A big picture fits below 10 %: zooming out there never enlarges it.
        assert!(wheel_zoom(0.068, 0.8) <= 0.068);
        assert!(wheel_zoom(0.068, 1.25) > 0.068);
    }

    #[test]
    fn the_preview_stays_under_the_gpu_texture_limit() {
        // Wide pictures at 200 % against a 2048 px texture limit; 1843 px used to round 1 px past.
        for (w, h) in [(3000, 500), (1843, 500)] {
            let shared = crate::Shared::for_tests();
            let base = RgbaImage::from_pixel(w, h, image::Rgba([240, 240, 240, 255]));
            let mut ed = Editor::new(&egui::Context::default(), base, Doc::new(), &shared);
            let mut shared = shared;
            let ctx = egui::Context::default();
            shared.fonts.install(&ctx);
            let t = crate::ui::tokens::tokens(crate::ui::tokens::ThemeId::TakeUi, true);
            ed.apply(
                EditorAction::Zoom(Zoom::Percent(200)),
                &ctx,
                None,
                &mut shared,
            );
            for i in 0..3 {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
                    max_texture_side: Some(2048),
                    time: Some(i as f64 * 0.05),
                    ..Default::default()
                };
                ctx.run_ui(input, |ui| {
                    ed.canvas(ui, &t, &mut shared);
                })
                .textures_delta
                .clear();
            }
            let side = ed.preview.as_ref().map(|t| t.size()).expect("a preview");
            assert!(side[0] <= 2048 && side[1] <= 2048, "{w}x{h}: {side:?}");
            assert!(!ed.preview_dirty, "rendered once, not every frame");
            assert_eq!(
                ed.preview_target,
                ed.shown_scale().min(1.0),
                "no rebuild loop"
            );
        }
    }

    #[test]
    fn duplicate_copies_12_px_away() {
        // The button and Ctrl+D give the same copy.
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        let id = ed.an.doc.objs()[0].id;
        ed.apply(EditorAction::Select(id, false), &ctx, None, &mut shared);
        ed.apply(EditorAction::Duplicate, &ctx, None, &mut shared);
        let shape = |i: usize| ed.an.doc.objs()[i].ann.shape.clone();
        match (shape(0), shape(1)) {
            (Shape::Rect(a, _, _), Shape::Rect(b, _, _)) => assert_eq!(b - a, vec2(12.0, 12.0)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_preview_waits_for_the_zoom_to_rest() {
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        shared.fonts.install(&ctx);
        let t = crate::ui::tokens::tokens(crate::ui::tokens::ThemeId::TakeUi, true);
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let frame = |ed: &mut Editor, shared: &mut crate::Shared, events, time| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events,
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                ed.canvas(ui, &t, shared);
            })
            .textures_delta
            .clear();
        };
        let over = egui::Event::PointerMoved(pos2(400.0, 300.0));
        frame(&mut ed, &mut shared, vec![over.clone()], 0.0);
        frame(&mut ed, &mut shared, vec![over], 0.016);
        let before = ed.preview_target;
        frame(&mut ed, &mut shared, vec![egui::Event::Zoom(0.8)], 0.032);
        frame(&mut ed, &mut shared, vec![egui::Event::Zoom(0.8)], 0.048);
        assert!(ed.shown_scale() < before, "zoomed out");
        assert_eq!(ed.preview_target, before, "no new picture mid-gesture");
        frame(&mut ed, &mut shared, vec![], 0.30);
        assert!(
            (ed.preview_target - ed.shown_scale().min(1.0)).abs() < 1e-4,
            "rendered again once the zoom rests"
        );
    }

    fn editor() -> (Editor, crate::Shared) {
        let shared = crate::Shared::for_tests();
        let base = RgbaImage::from_pixel(200, 100, image::Rgba([240, 240, 240, 255]));
        let doc = Doc::from_anns(vec![Ann {
            shape: Shape::Rect(pos2(10.0, 10.0), pos2(60.0, 50.0), 0.0),
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: false,
        }]);
        let ed = Editor::new(&egui::Context::default(), base, doc, &shared);
        (ed, shared)
    }

    #[test]
    fn a_corners_drag_is_one_undo_step() {
        let ctx = egui::Context::default();
        let (mut ed, mut shared) = editor();
        let id = ed.an.doc.objs()[0].id;
        ed.apply(EditorAction::Select(id, false), &ctx, None, &mut shared);
        ed.apply(EditorAction::Gesture(true), &ctx, None, &mut shared);
        for r in [6.0, 12.0, 20.0] {
            ed.apply(EditorAction::SetRadius(r), &ctx, None, &mut shared);
        }
        ed.apply(EditorAction::Gesture(false), &ctx, None, &mut shared);
        let radius = |ed: &Editor| match ed.an.doc.objs()[0].ann.shape {
            Shape::Rect(_, _, r) => r,
            _ => unreachable!(),
        };
        assert_eq!(radius(&ed), 20.0);
        assert_eq!(shared.settings.draw.radius, 20.0, "the next rectangle too");
        ed.apply(EditorAction::Undo, &ctx, None, &mut shared);
        assert_eq!(radius(&ed), 0.0, "one step back to square");
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
            caret: None,
        });
        assert!(unsaved_edits(&an, saved), "text being typed");
        an.draft = None;
        an.commit(Ann {
            shape: Shape::Rect(pos2(0.0, 0.0), pos2(50.0, 50.0), 0.0),
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
