//! Drawing on a picture: the same logic runs on the capture overlay and in the editor.
//! Annotations live in a [`Doc`] of objects that can be selected, moved, resized and restyled.

use crate::engine::doc::{Doc, Handle, ObjId, TextMeasure, ann_bounds};
use crate::engine::model::*;
use crate::engine::render::text_box_padding;
use crate::system::fonts::{self, FontLib};
use crate::system::settings::DrawOpts;
use crate::ui::kit::selection_marker;
use crate::ui::tokens::Tokens;
use egui::{Color32, Painter, Pos2, Response, Shape as EShape, Stroke, Ui, pos2, vec2};

/// How far from a handle, in screen points, a press still grabs it.
const HANDLE_REACH: f32 = 7.0;
/// How far from a stroke, in screen points, a click still picks its object.
const HIT_REACH: f32 = 4.0;

/// Maps picture pixels to screen points: screen = origin + px * k.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub origin: Pos2,
    pub k: f32,
}

impl View {
    pub fn to_screen(self, p: Pos2) -> Pos2 {
        self.origin + p.to_vec2() * self.k
    }
    pub fn to_img(self, s: Pos2) -> Pos2 {
        ((s - self.origin) / self.k).to_pos2()
    }
}

pub struct TextDraft {
    pub pos: Pos2,
    pub text: String,
    pub started: f64,
}

/// An object drag in progress: the whole selection, or one handle of one object.
#[derive(Clone, Copy, Debug)]
pub enum Drag {
    Move { last: Pos2 },
    Resize { id: ObjId, handle: Handle },
}

/// Whether the selection took this pointer interaction, or left it to drawing or the capture area.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectOutcome {
    Consumed,
    Missed,
}

pub struct Annotator {
    pub doc: Doc,
    pub tool: Tool,
    pub live: Option<Ann>,
    start: Option<Pos2>,
    pub draft: Option<TextDraft>,
    pub dragging: Option<Drag>,
    /// Revision and left-out objects of the last baked picture.
    baked: Option<(u64, Vec<ObjId>)>,
}

impl Annotator {
    pub fn new(tool: Tool) -> Self {
        Self {
            doc: Doc::new(),
            tool,
            live: None,
            start: None,
            draft: None,
            dragging: None,
            baked: None,
        }
    }

    pub fn busy(&self) -> bool {
        self.live.is_some() || self.dragging.is_some()
    }

    pub fn next_counter(&self) -> u32 {
        self.doc.next_counter()
    }

    pub fn commit(&mut self, a: Ann) {
        self.doc.add(a);
    }

    pub fn undo(&mut self) {
        if self.draft.take().is_some() {
            return;
        }
        self.doc.undo();
    }

    pub fn redo(&mut self) {
        self.doc.redo();
    }

    /// Objects left out of the baked picture: the selection while it is dragged, painted live instead.
    pub fn excluded_from_bake(&self) -> Vec<ObjId> {
        if self.dragging.is_some() {
            self.doc.selected().to_vec()
        } else {
            Vec::new()
        }
    }

    /// The annotations to render when the baked picture is out of date, once per change.
    /// During a drag only its start re-bakes, to take the dragged objects out; moves are painted live.
    pub fn bake(&mut self) -> Option<Vec<Ann>> {
        let excluded = self.excluded_from_bake();
        let rev = self.doc.revision();
        let fresh = self
            .baked
            .as_ref()
            .is_some_and(|(r, ex)| *ex == excluded && (self.dragging.is_some() || *r == rev));
        if fresh {
            return None;
        }
        let anns = self.doc.visible_anns(&excluded);
        self.baked = Some((rev, excluded));
        Some(anns)
    }

    pub fn commit_draft(&mut self, opts: &DrawOpts) {
        if let Some(d) = self.draft.take() {
            self.commit(Ann {
                shape: Shape::Text {
                    pos: d.pos,
                    text: d.text,
                    font: opts.font,
                    size: opts.font_size,
                    boxed: opts.text_box,
                },
                color: opts.color,
                width: opts.width,
                filled: false,
            });
        }
    }

    pub fn set_tool(&mut self, t: Tool, opts: &mut DrawOpts) {
        self.commit_draft(opts);
        self.tool = t;
        if t != Tool::Select {
            opts.last_tool = t;
        }
    }

    fn new_shape(&self, p: Pos2, opts: &DrawOpts) -> Shape {
        match self.tool {
            Tool::Pen => Shape::Pen(vec![p]),
            Tool::Marker => Shape::Marker(vec![p]),
            Tool::Line => Shape::Line(p, p),
            Tool::Arrow => Shape::Arrow(p, p),
            Tool::Rect => Shape::Rect(p, p),
            Tool::Ellipse => Shape::Ellipse(p, p),
            Tool::Blur => Shape::Blur(p, p, opts.blur_mode),
            _ => Shape::Line(p, p),
        }
    }

    /// Pointer handling for drawing tools on a canvas response.
    pub fn handle_pointer(&mut self, ui: &Ui, resp: &Response, view: &View, opts: &DrawOpts) {
        let mods = ui.input(|i| i.modifiers);
        let (shift, alt) = (mods.shift, mods.alt);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        match self.tool {
            Tool::Select => {}
            Tool::Text => {
                if (resp.clicked() || resp.drag_started())
                    && let Some(p) = pointer
                {
                    self.commit_draft(opts);
                    let now = ui.input(|i| i.time);
                    self.draft = Some(TextDraft {
                        pos: view.to_img(p),
                        text: String::new(),
                        started: now,
                    });
                }
            }
            Tool::Counter => {
                if resp.clicked()
                    && let Some(p) = pointer
                {
                    let n = self.next_counter();
                    self.commit(Ann {
                        shape: Shape::Counter {
                            center: view.to_img(p),
                            n,
                        },
                        color: opts.color,
                        width: opts.width,
                        filled: true,
                    });
                }
            }
            _ => {
                if resp.drag_started() {
                    let origin = ui.input(|i| i.pointer.press_origin()).or(pointer);
                    if let Some(o) = origin {
                        let p = view.to_img(o);
                        self.start = Some(p);
                        self.live = Some(Ann {
                            shape: self.new_shape(p, opts),
                            color: opts.color,
                            width: opts.width,
                            filled: opts.filled,
                        });
                    }
                }
                if resp.dragged()
                    && let (Some(start), Some(cur), Some(live)) =
                        (self.start, pointer, self.live.as_mut())
                {
                    let cur = view.to_img(cur);
                    let (a, b) = constrain(self.tool, start, cur, shift, alt);
                    match &mut live.shape {
                        Shape::Pen(pts) | Shape::Marker(pts) => {
                            if shift {
                                *pts = vec![a, b];
                            } else {
                                let last = *pts.last().unwrap();
                                if last.distance(cur) * view.k >= 1.5 {
                                    pts.push(cur);
                                }
                            }
                        }
                        Shape::Line(p0, p1)
                        | Shape::Arrow(p0, p1)
                        | Shape::Rect(p0, p1)
                        | Shape::Ellipse(p0, p1)
                        | Shape::Blur(p0, p1, _) => {
                            *p0 = a;
                            *p1 = b;
                        }
                        _ => {}
                    }
                }
                if resp.drag_stopped() {
                    if let Some(a) = self.live.take() {
                        self.commit(a);
                    }
                    self.start = None;
                }
            }
        }
    }

    /// Selection, moving and resizing on a canvas response. Handles of selected objects answer with
    /// any tool; picking and moving by the body only with Select. `Missed` leaves the interaction to
    /// drawing (`handle_pointer`) or to the capture area.
    #[allow(dead_code)] // The editor and the capture call it from plan 3, Tasks 4 and 5.
    pub fn handle_select_pointer(
        &mut self,
        ui: &Ui,
        resp: &Response,
        view: &View,
        m: TextMeasure,
        opts: &mut DrawOpts,
    ) -> SelectOutcome {
        let (shift, pointer, origin) = ui.input(|i| {
            (
                i.modifiers.shift,
                i.pointer.interact_pos(),
                i.pointer.press_origin(),
            )
        });
        if let Some(drag) = self.dragging {
            if resp.dragged()
                && let Some(p) = pointer
            {
                self.drag_to(drag, view.to_img(p), shift, m);
            } else {
                self.dragging = None;
            }
            return SelectOutcome::Consumed;
        }
        let started = resp.drag_started();
        let at = if started {
            origin.or(pointer)
        } else if resp.clicked() {
            pointer
        } else {
            None
        };
        let Some(at) = at else {
            return SelectOutcome::Missed;
        };
        let p = view.to_img(at);
        let drag = if let Some((id, handle)) = self.doc.handle_at(p, HANDLE_REACH / view.k, m) {
            Drag::Resize { id, handle }
        } else if self.tool != Tool::Select {
            return SelectOutcome::Missed;
        } else if let Some(id) = self.doc.hit(p, HIT_REACH / view.k, m) {
            if shift {
                self.doc.select(Some(id), true);
            } else if !started || !self.doc.selected().contains(&id) {
                // Dragging an object of a multi-selection moves them all; a click picks just it.
                self.doc.select(Some(id), false);
            }
            self.copy_style(id, opts);
            Drag::Move { last: p }
        } else {
            if !shift {
                self.doc.select(None, false);
            }
            return SelectOutcome::Missed;
        };
        if started {
            self.doc.begin_change();
            self.dragging = Some(drag);
            if let Some(cur) = pointer {
                self.drag_to(drag, view.to_img(cur), shift, m);
            }
        }
        SelectOutcome::Consumed
    }

    fn drag_to(&mut self, drag: Drag, p: Pos2, keep_ratio: bool, m: TextMeasure) {
        match drag {
            Drag::Move { last } => {
                self.doc.translate_selected(p - last);
                self.dragging = Some(Drag::Move { last: p });
            }
            Drag::Resize { id, handle } => self.doc.resize(id, handle, p, keep_ratio, m),
        }
    }

    /// The options bar shows the style of what was just picked, so the next change applies to it.
    fn copy_style(&self, id: ObjId, opts: &mut DrawOpts) {
        let Some(o) = self.doc.objs().iter().find(|o| o.id == id) else {
            return;
        };
        opts.color = o.ann.color;
        match o.ann.shape {
            Shape::Text { .. } => {}
            Shape::Rect(..) | Shape::Ellipse(..) => {
                opts.width = o.ann.width;
                opts.filled = o.ann.filled;
            }
            _ => opts.width = o.ann.width,
        }
    }

    /// Applies to the selection what the options bar changed between `before` and `after`.
    #[allow(dead_code)] // The editor and the capture call it from plan 3, Tasks 4 and 5.
    pub fn restyle_from_opts(&mut self, before: &DrawOpts, after: &DrawOpts) {
        let color = (after.color != before.color).then_some(after.color);
        let width = (after.width != before.width).then_some(after.width);
        let filled = (after.filled != before.filled).then_some(after.filled);
        if color.is_some() || width.is_some() || filled.is_some() {
            self.doc.restyle_selected(color, width, filled);
        }
    }

    /// Double click on a text object: takes it out of the picture and reopens it for typing,
    /// with its font, size, background and colour back in the options.
    #[allow(dead_code)] // The editor and the capture call it from plan 3, Tasks 4 and 5.
    pub fn edit_text_at(&mut self, p_img: Pos2, m: TextMeasure, opts: &mut DrawOpts) -> bool {
        self.commit_draft(opts);
        let Some(id) = self.doc.hit(p_img, 0.0, m) else {
            return false;
        };
        let is_text = self
            .doc
            .objs()
            .iter()
            .any(|o| o.id == id && matches!(o.ann.shape, Shape::Text { .. }));
        if !is_text {
            return false;
        }
        let Some(Ann {
            shape:
                Shape::Text {
                    pos,
                    text,
                    font,
                    size,
                    boxed,
                },
            color,
            ..
        }) = self.doc.take_for_edit(id)
        else {
            return false;
        };
        opts.font = font;
        opts.font_size = size;
        opts.text_box = boxed;
        opts.color = color;
        self.draft = Some(TextDraft {
            pos,
            text,
            started: 0.0,
        });
        true
    }

    /// Keyboard input while typing text. Returns true when keys were used for the draft.
    pub fn handle_typing(&mut self, ui: &Ui, opts: &DrawOpts) -> bool {
        let Some(_) = &self.draft else { return false };
        let events = ui.input(|i| i.events.clone());
        let mut commit = false;
        let mut cancel = false;
        if let Some(d) = self.draft.as_mut() {
            for e in events {
                match e {
                    egui::Event::Text(t) => d.text.push_str(&t),
                    egui::Event::Paste(t) => d.text.push_str(&t),
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => {
                        d.text.pop();
                    }
                    egui::Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        if modifiers.shift {
                            d.text.push('\n');
                        } else {
                            commit = true;
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Escape,
                        pressed: true,
                        ..
                    } => {
                        if d.text.is_empty() {
                            cancel = true;
                        } else {
                            commit = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        if cancel {
            self.draft = None;
        } else if commit {
            self.commit_draft(opts);
        }
        // swallow the keys so shortcuts don't fire while typing
        ui.input_mut(|i| {
            i.events
                .retain(|e| !matches!(e, egui::Event::Text(_) | egui::Event::Key { .. }));
        });
        true
    }

    /// Paint the shape being drawn and the text being typed.
    pub fn paint_live(
        &self,
        painter: &Painter,
        view: &View,
        fonts: &FontLib,
        opts: &DrawOpts,
        time: f64,
    ) {
        if let Some(a) = &self.live {
            paint_ann(painter, view, a, fonts);
        }
        if let Some(d) = &self.draft {
            let size = opts.font_size * view.k;
            let shown = if d.text.is_empty() {
                "Digite…"
            } else {
                d.text.as_str()
            };
            let rect = paint_text(
                painter,
                view.to_screen(d.pos),
                shown,
                fonts.get(opts.font).family.clone(),
                size,
                opts.color,
                opts.text_box,
                if d.text.is_empty() { 0.5 } else { 1.0 },
            );
            // dashed frame + caret
            painter.rect_stroke(
                rect.expand(4.0),
                3.0,
                Stroke::new(1.0, Color32::from_white_alpha(140)),
                egui::StrokeKind::Outside,
            );
            if ((time - d.started) * 2.0) as i64 % 2 == 0 {
                let x = if d.text.is_empty() {
                    rect.min.x
                } else {
                    rect.max.x + 1.0
                };
                let last_line_top = rect.max.y - size;
                painter.line_segment(
                    [pos2(x, last_line_top.max(rect.min.y)), pos2(x, rect.max.y)],
                    Stroke::new(1.5, Color32::WHITE),
                );
            }
        }
    }

    /// Paints the dragged objects live (they are out of the baked picture) and, on every selected
    /// object, the theme's selection marker and its handles: white squares with a 2 px primary
    /// border, round on line and arrow ends.
    #[allow(dead_code)] // The editor and the capture call it from plan 3, Tasks 4 and 5.
    pub fn paint_selection(
        &self,
        painter: &Painter,
        view: &View,
        t: &Tokens,
        fonts: &FontLib,
        m: TextMeasure,
    ) {
        let excluded = self.excluded_from_bake();
        for o in self.doc.objs().iter().filter(|o| excluded.contains(&o.id)) {
            paint_ann(painter, view, &o.ann, fonts);
        }
        let ring = Stroke::new(2.0, t.primary);
        for o in self
            .doc
            .objs()
            .iter()
            .filter(|o| !o.hidden && self.doc.selected().contains(&o.id))
        {
            let b = ann_bounds(&o.ann, m);
            let screen = egui::Rect::from_min_max(view.to_screen(b.min), view.to_screen(b.max));
            selection_marker(painter, t, screen);
            for (h, at) in self.doc.handles(o.id, m) {
                let c = view.to_screen(at);
                if matches!(h, Handle::Start | Handle::End) {
                    painter.circle(c, 5.0, Color32::WHITE, ring);
                } else {
                    painter.rect(
                        egui::Rect::from_center_size(c, vec2(8.0, 8.0)),
                        1.0,
                        Color32::WHITE,
                        ring,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
    }
}

/// Measures text as the final render draws it, background spacing included on both sides.
#[allow(dead_code)] // The editor and the capture call it from plan 3, Tasks 4 and 5.
pub fn text_measure(fonts: &FontLib) -> impl Fn(&str, usize, f32, bool) -> egui::Vec2 + '_ {
    move |text, font, size, boxed| {
        let (w, h) = fonts::measure(&fonts.get(font).font, size, text);
        let pad = if boxed {
            text_box_padding(size) * 2.0
        } else {
            egui::Vec2::ZERO
        };
        vec2(w, h) + pad
    }
}

/// Draws text at `pos` (screen) with its optional background, as the editor shows it; returns the text rect.
#[allow(clippy::too_many_arguments)]
fn paint_text(
    p: &Painter,
    pos: Pos2,
    text: &str,
    family: egui::FontFamily,
    size: f32,
    color: [u8; 4],
    boxed: bool,
    fade: f32,
) -> egui::Rect {
    let size = size.max(4.0);
    let ink = if !boxed {
        c32(color)
    } else if luminance(color) > 0.6 {
        Color32::from_rgb(17, 17, 17)
    } else {
        Color32::WHITE
    }
    .gamma_multiply(fade);
    let galley = p.layout(
        text.to_owned(),
        egui::FontId::new(size, family),
        ink,
        f32::INFINITY,
    );
    let rect = egui::Rect::from_min_size(pos, galley.size());
    if boxed {
        p.rect_filled(
            rect.expand2(text_box_padding(size)),
            size * 0.25,
            c32(color),
        );
    }
    p.galley(pos, galley, ink);
    rect
}

/// Approximate on-screen drawing of one annotation (while it is drawn or dragged).
pub fn paint_ann(p: &Painter, v: &View, a: &Ann, fonts: &FontLib) {
    let col = c32(a.color);
    let w = (a.width * v.k).max(1.0);
    let st = Stroke::new(w, col);
    match &a.shape {
        Shape::Pen(pts) => {
            let pts: Vec<Pos2> = pts.iter().map(|q| v.to_screen(*q)).collect();
            if pts.len() > 1 {
                p.add(EShape::line(pts.clone(), st));
            }
            for q in [pts.first(), pts.last()].into_iter().flatten() {
                p.circle_filled(*q, w / 2.0, col);
            }
        }
        Shape::Marker(pts) => {
            let c =
                Color32::from_rgba_unmultiplied(a.color[0], a.color[1], a.color[2], MARKER_ALPHA);
            let pts: Vec<Pos2> = pts.iter().map(|q| v.to_screen(*q)).collect();
            if pts.len() > 1 {
                p.add(EShape::line(
                    pts,
                    Stroke::new(a.width * MARKER_WIDTH_FACTOR * v.k, c),
                ));
            }
        }
        Shape::Line(a0, b0) => {
            p.line_segment([v.to_screen(*a0), v.to_screen(*b0)], st);
        }
        Shape::Arrow(a0, b0) => {
            let (end, head) = arrow_geom(*a0, *b0, a.width);
            p.line_segment([v.to_screen(*a0), v.to_screen(end)], st);
            p.add(EShape::convex_polygon(
                head.iter().map(|q| v.to_screen(*q)).collect(),
                col,
                Stroke::NONE,
            ));
        }
        Shape::Rect(a0, b0) => {
            let r = egui::Rect::from_two_pos(v.to_screen(*a0), v.to_screen(*b0));
            if a.filled {
                p.rect_filled(r, 0.0, col);
            } else {
                p.rect_stroke(r, 0.0, st, egui::StrokeKind::Middle);
            }
        }
        Shape::Ellipse(a0, b0) => {
            let r = egui::Rect::from_two_pos(v.to_screen(*a0), v.to_screen(*b0));
            p.add(EShape::Ellipse(egui::epaint::EllipseShape {
                center: r.center(),
                radius: r.size() / 2.0,
                fill: if a.filled { col } else { Color32::TRANSPARENT },
                stroke: if a.filled { Stroke::NONE } else { st },
                angle: 0.0,
            }));
        }
        Shape::Blur(a0, b0, _) => {
            let r = egui::Rect::from_two_pos(v.to_screen(*a0), v.to_screen(*b0));
            p.rect_filled(r, 2.0, Color32::from_white_alpha(40));
            dashed_rect(p, r, Color32::WHITE);
        }
        Shape::Counter { center, n } => {
            let c = v.to_screen(*center);
            let r = counter_radius(a.width) * v.k;
            p.circle_filled(c, r, col);
            p.text(
                c,
                egui::Align2::CENTER_CENTER,
                n.to_string(),
                egui::FontId::new(r * 1.2, egui::FontFamily::Name("bold".into())),
                Color32::WHITE,
            );
        }
        Shape::Text {
            pos,
            text,
            font,
            size,
            boxed,
        } => {
            paint_text(
                p,
                v.to_screen(*pos),
                text,
                fonts.get(*font).family.clone(),
                size * v.k,
                a.color,
                *boxed,
                1.0,
            );
        }
    }
}

pub fn dashed_rect(p: &Painter, r: egui::Rect, col: Color32) {
    let pts = vec![
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
        r.left_top(),
    ];
    p.extend(EShape::dashed_line(&pts, Stroke::new(1.0, col), 5.0, 4.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::doc::{Doc, ObjId, ann_bounds};
    use crate::ui::tokens::{ThemeId, tokens};
    use egui::{Event, Modifiers, PointerButton, Rect, Sense, Vec2};

    fn m(t: &str, _f: usize, size: f32, boxed: bool) -> Vec2 {
        let w = t.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * size * 0.5;
        let h = t.lines().count() as f32 * size * 1.2;
        let pad = if boxed { 8.0 } else { 0.0 };
        vec2(w + pad, h + pad)
    }

    const VIEW: View = View {
        origin: Pos2::ZERO,
        k: 1.0,
    };

    fn rect_ann(a: (f32, f32), b: (f32, f32), filled: bool) -> Ann {
        Ann {
            shape: Shape::Rect(pos2(a.0, a.1), pos2(b.0, b.1)),
            color: [255, 0, 0, 255],
            width: 4.0,
            filled,
        }
    }

    fn press(p: Pos2) -> Vec<Event> {
        vec![
            Event::PointerMoved(p),
            Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]
    }

    fn release(p: Pos2) -> Vec<Event> {
        vec![Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]
    }

    fn to(p: Pos2) -> Vec<Event> {
        vec![Event::PointerMoved(p)]
    }

    fn click(p: Pos2) -> Vec<Vec<Event>> {
        vec![press(p), release(p), vec![]]
    }

    fn drag(from: Pos2, path: &[Pos2]) -> Vec<Vec<Event>> {
        let mut f = vec![press(from)];
        f.extend(path.iter().map(|p| to(*p)));
        f.push(release(*path.last().unwrap()));
        f.push(vec![]);
        f
    }

    /// What one frame of `handle_select_pointer` did, as the screens call it.
    struct Frame {
        outcome: SelectOutcome,
        excluded: Vec<ObjId>,
        baked: bool,
    }

    fn drive(
        an: &mut Annotator,
        opts: &mut DrawOpts,
        frames: Vec<Vec<Event>>,
        shift: bool,
    ) -> Vec<Frame> {
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
        let canvas = |ui: &mut Ui| {
            ui.interact(
                ui.max_rect(),
                ui.scope_id().with("canvas"),
                Sense::click_and_drag(),
            )
        };
        // egui hit-tests against the previous frame's widgets, so the canvas is laid out once first.
        let warm = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        ctx.run_ui(warm, |ui| {
            canvas(ui);
        })
        .textures_delta
        .clear();
        let mut out = Vec::new();
        let held = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        for (i, events) in frames.into_iter().enumerate() {
            let mut all = vec![Event::ModifiersChanged(held)];
            all.extend(events);
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(0.016 + i as f64 * 0.016),
                events: all,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                let resp = canvas(ui);
                let outcome = an.handle_select_pointer(ui, &resp, &VIEW, &m, opts);
                out.push(Frame {
                    outcome,
                    excluded: an.excluded_from_bake(),
                    baked: an.bake().is_some(),
                });
            });
            full.textures_delta.clear();
        }
        out
    }

    fn with(tool: Tool, anns: Vec<Ann>) -> (Annotator, Vec<ObjId>) {
        let mut an = Annotator::new(tool);
        an.doc = Doc::from_anns(anns);
        let ids = an.doc.objs().iter().map(|o| o.id).collect();
        (an, ids)
    }

    fn bounds(an: &Annotator, id: ObjId) -> Rect {
        let o = an.doc.objs().iter().find(|o| o.id == id).unwrap();
        ann_bounds(&o.ann, &m)
    }

    #[test]
    fn dragging_an_object_moves_it_in_one_undo_step_and_bakes_only_at_the_ends() {
        let (mut an, ids) = with(
            Tool::Select,
            vec![rect_ann((100.0, 100.0), (200.0, 200.0), true)],
        );
        let mut opts = DrawOpts::default();
        let before = bounds(&an, ids[0]);
        let path = [
            pos2(160.0, 150.0),
            pos2(250.0, 150.0),
            pos2(350.0, 150.0),
            pos2(450.0, 150.0),
        ];
        let frames = drive(&mut an, &mut opts, drag(pos2(150.0, 150.0), &path), false);
        // Frames: press, drag start, three moves, release, idle.
        assert!(
            frames[1..=5]
                .iter()
                .all(|f| f.outcome == SelectOutcome::Consumed)
        );
        assert_eq!(
            frames[3].excluded, ids,
            "the dragged object is painted live"
        );
        assert!(frames[6].excluded.is_empty());
        let baked: Vec<bool> = frames.iter().map(|f| f.baked).collect();
        assert_eq!(baked, vec![true, true, false, false, false, true, false]);
        assert!((bounds(&an, ids[0]).min.x - (before.min.x + 300.0)).abs() < 0.01);
        assert!(an.dragging.is_none());
        an.undo();
        assert!((bounds(&an, ids[0]).min.x - before.min.x).abs() < 0.01);
    }

    #[test]
    fn handles_resize_with_any_tool_and_drawing_goes_on_elsewhere() {
        for tool in [Tool::Select, Tool::Rect] {
            let (mut an, ids) = with(tool, vec![rect_ann((100.0, 100.0), (200.0, 200.0), false)]);
            an.doc.select(Some(ids[0]), false);
            let mut opts = DrawOpts::default();
            let path = [pos2(220.0, 220.0), pos2(300.0, 300.0)];
            let frames = drive(&mut an, &mut opts, drag(pos2(200.0, 200.0), &path), false);
            assert!(
                frames[1..=3]
                    .iter()
                    .all(|f| f.outcome == SelectOutcome::Consumed),
                "{tool:?}"
            );
            match an.doc.objs()[0].ann.shape {
                Shape::Rect(a, b) => {
                    assert!(a.distance(pos2(100.0, 100.0)) < 0.01, "{tool:?}");
                    assert!(b.distance(pos2(300.0, 300.0)) < 0.01, "{tool:?}");
                }
                _ => unreachable!(),
            }
            assert_eq!(an.doc.objs().len(), 1);
        }
        // Away from the handles a drawing tool is left to draw.
        let (mut an, ids) = with(
            Tool::Rect,
            vec![rect_ann((100.0, 100.0), (200.0, 200.0), false)],
        );
        an.doc.select(Some(ids[0]), false);
        let mut opts = DrawOpts::default();
        let frames = drive(
            &mut an,
            &mut opts,
            drag(pos2(400.0, 400.0), &[pos2(500.0, 450.0)]),
            false,
        );
        assert!(frames.iter().all(|f| f.outcome == SelectOutcome::Missed));
        assert!(an.dragging.is_none());
    }

    #[test]
    fn click_selects_shift_click_adds_and_empty_click_clears() {
        let (mut an, ids) = with(
            Tool::Select,
            vec![
                rect_ann((0.0, 0.0), (100.0, 100.0), true),
                rect_ann((200.0, 0.0), (300.0, 100.0), true),
            ],
        );
        let mut opts = DrawOpts::default();
        let f = drive(&mut an, &mut opts, click(pos2(50.0, 50.0)), false);
        assert!(f.iter().any(|f| f.outcome == SelectOutcome::Consumed));
        assert_eq!(an.doc.selected(), &[ids[0]]);
        drive(&mut an, &mut opts, click(pos2(250.0, 50.0)), true);
        assert_eq!(an.doc.selected(), &[ids[0], ids[1]]);
        let f = drive(&mut an, &mut opts, click(pos2(500.0, 500.0)), false);
        assert!(f.iter().all(|f| f.outcome == SelectOutcome::Missed));
        assert!(an.doc.selected().is_empty());
    }

    #[test]
    fn selecting_copies_the_object_style_into_the_options() {
        let mut blue = rect_ann((0.0, 0.0), (100.0, 100.0), true);
        blue.color = [0, 0, 255, 255];
        blue.width = 8.0;
        let (mut an, _) = with(Tool::Select, vec![blue]);
        let mut opts = DrawOpts::default();
        drive(&mut an, &mut opts, click(pos2(50.0, 50.0)), false);
        assert_eq!(opts.color, [0, 0, 255, 255]);
        assert_eq!(opts.width, 8.0);
        assert!(opts.filled);
    }

    #[test]
    fn option_bar_changes_restyle_the_selection() {
        let (mut an, ids) = with(
            Tool::Select,
            vec![rect_ann((0.0, 0.0), (100.0, 100.0), false)],
        );
        let before = DrawOpts::default();
        let mut after = before.clone();
        after.color = [0, 200, 0, 255];
        let rev = an.doc.revision();
        an.restyle_from_opts(&before, &after);
        assert_eq!(an.doc.revision(), rev, "nothing selected, nothing restyled");
        an.doc.select(Some(ids[0]), false);
        an.restyle_from_opts(&before, &after);
        assert_eq!(an.doc.objs()[0].ann.color, [0, 200, 0, 255]);
        assert_eq!(an.doc.objs()[0].ann.width, 4.0, "only what the bar changed");
        an.undo();
        assert_eq!(an.doc.objs()[0].ann.color, [255, 0, 0, 255]);
    }

    #[test]
    fn double_click_on_text_reopens_it_for_typing() {
        let text = Ann {
            shape: Shape::Text {
                pos: pos2(100.0, 100.0),
                text: "hi".into(),
                font: 0,
                size: 20.0,
                boxed: true,
            },
            color: [0, 0, 255, 255],
            width: 3.0,
            filled: false,
        };
        let (mut an, _) = with(Tool::Select, vec![text]);
        let mut opts = DrawOpts::default();
        assert!(!an.edit_text_at(pos2(500.0, 500.0), &m, &mut opts));
        assert!(an.edit_text_at(pos2(105.0, 105.0), &m, &mut opts));
        let d = an.draft.as_ref().unwrap();
        assert_eq!((d.pos, d.text.as_str()), (pos2(100.0, 100.0), "hi"));
        assert!(an.doc.objs().is_empty());
        assert_eq!(
            (opts.font_size, opts.text_box, opts.color),
            (20.0, true, [0, 0, 255, 255])
        );
        an.commit_draft(&opts);
        assert_eq!(an.doc.objs().len(), 1);
    }

    #[test]
    fn text_measure_adds_the_background_on_both_sides() {
        let fonts = FontLib::load();
        let f = text_measure(&fonts);
        let plain = f("Ab\nc", 0, 20.0, false);
        assert!(plain.x > 0.0 && plain.y > 20.0, "{plain:?}");
        let boxed = f("Ab\nc", 0, 20.0, true);
        let pad = crate::engine::render::text_box_padding(20.0) * 2.0;
        assert!(
            (boxed - plain - pad).length() < 0.01,
            "{boxed:?} vs {plain:?}"
        );
    }

    #[test]
    fn selection_paints_handles_and_the_dragged_text_live() {
        let line = Ann {
            shape: Shape::Line(pos2(10.0, 10.0), pos2(100.0, 10.0)),
            color: [255, 0, 0, 255],
            width: 4.0,
            filled: false,
        };
        let text = Ann {
            shape: Shape::Text {
                pos: pos2(100.0, 100.0),
                text: "hi".into(),
                font: 0,
                size: 20.0,
                boxed: false,
            },
            color: [0, 0, 255, 255],
            width: 3.0,
            filled: false,
        };
        let (mut an, ids) = with(Tool::Select, vec![line, text]);
        let fonts = FontLib::load();
        let t = tokens(ThemeId::TakeUi, true);
        let ctx = egui::Context::default();
        fonts.install(&ctx);
        let paint = |an: &Annotator| {
            let mut full = ctx.run_ui(egui::RawInput::default(), |ui| {
                an.paint_selection(ui.painter(), &VIEW, &t, &fonts, &m);
            });
            full.textures_delta.clear();
            let mut circles = 0;
            let mut texts = Vec::new();
            for c in &full.shapes {
                match &c.shape {
                    egui::Shape::Circle(_) => circles += 1,
                    egui::Shape::Text(tx) => texts.push(tx.galley.text().to_owned()),
                    _ => {}
                }
            }
            (circles, texts)
        };
        an.doc.select(Some(ids[0]), false);
        let (circles, texts) = paint(&an);
        assert_eq!(circles, 2, "line ends get round handles");
        assert!(
            texts.is_empty(),
            "nothing is dragged, so nothing is painted live"
        );
        an.doc.select(Some(ids[1]), false);
        an.dragging = Some(Drag::Move { last: Pos2::ZERO });
        let (_, texts) = paint(&an);
        assert_eq!(texts, vec!["hi".to_owned()]);
    }
}
