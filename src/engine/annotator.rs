//! Drawing on a picture: the same logic runs on the capture overlay and in the editor.

use crate::engine::model::*;
use crate::system::fonts::FontLib;
use crate::system::settings::DrawOpts;
use egui::{Color32, Painter, Pos2, Response, Shape as EShape, Stroke, Ui, pos2, vec2};

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

pub struct Annotator {
    pub anns: Vec<Ann>,
    pub redo: Vec<Ann>,
    pub tool: Tool,
    pub live: Option<Ann>,
    start: Option<Pos2>,
    pub draft: Option<TextDraft>,
    /// Set whenever `anns` changes, so the owner re-renders the baked picture.
    pub changed: bool,
}

impl Annotator {
    pub fn new(tool: Tool) -> Self {
        Self {
            anns: Vec::new(),
            redo: Vec::new(),
            tool,
            live: None,
            start: None,
            draft: None,
            changed: false,
        }
    }

    pub fn busy(&self) -> bool {
        self.live.is_some()
    }

    pub fn next_counter(&self) -> u32 {
        self.anns
            .iter()
            .filter(|a| matches!(a.shape, Shape::Counter { .. }))
            .count() as u32
            + 1
    }

    pub fn commit(&mut self, a: Ann) {
        if !a.is_degenerate() {
            self.anns.push(a);
            self.redo.clear();
            self.changed = true;
        }
    }

    pub fn undo(&mut self) {
        if self.draft.take().is_some() {
            return;
        }
        if let Some(a) = self.anns.pop() {
            self.redo.push(a);
            self.changed = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(a) = self.redo.pop() {
            self.anns.push(a);
            self.changed = true;
        }
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
            paint_ann(painter, view, a);
        }
        if let Some(d) = &self.draft {
            let size = opts.font_size * view.k;
            let fam = fonts.get(opts.font).family.clone();
            let shown = if d.text.is_empty() {
                "Digite…"
            } else {
                d.text.as_str()
            };
            let mut col = c32(opts.color);
            if d.text.is_empty() {
                col = col.gamma_multiply(0.5);
            }
            let pos = view.to_screen(d.pos);
            let galley = painter.layout(
                shown.to_string(),
                egui::FontId::new(size.max(4.0), fam),
                col,
                f32::INFINITY,
            );
            let rect = egui::Rect::from_min_size(pos, galley.size());
            if opts.text_box {
                let pad = crate::engine::render::text_box_padding(size);
                painter.rect_filled(rect.expand2(pad), size * 0.25, c32(opts.color));
            }
            let text_col = if opts.text_box {
                if luminance(opts.color) > 0.6 {
                    Color32::from_rgb(17, 17, 17)
                } else {
                    Color32::WHITE
                }
            } else {
                col
            };
            let galley = painter.layout(
                shown.to_string(),
                egui::FontId::new(size.max(4.0), fonts.get(opts.font).family.clone()),
                text_col,
                f32::INFINITY,
            );
            painter.galley(pos, galley, text_col);
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
}

/// Approximate on-screen drawing of one annotation (used while it is being drawn).
pub fn paint_ann(p: &Painter, v: &View, a: &Ann) {
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
        Shape::Text { .. } => {}
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
    let _ = vec2(0.0, 0.0);
}
