//! Object document: the annotations of one picture as selectable, editable objects with undo history.
//!
//! Objects are stored back to front. History is a stack of snapshots of the object list, one per
//! action (a whole drag is one action); the selection lives outside the history.

use crate::engine::model::{
    Ann, BlurMode, MARKER_WIDTH_FACTOR, Shape, Tool, arrow_geom, constrain, counter_radius,
    norm_rect,
};
use egui::{Pos2, Rect, Vec2, vec2};

pub type ObjId = u64;

#[derive(Clone, Debug)]
pub struct Obj {
    pub id: ObjId,
    pub ann: Ann,
    pub hidden: bool,
}

/// Grab points of a selected object: line ends, or the sides and corners of its box.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    Start,
    End,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

/// Measures text: (text, font index, size, with background) to its size, background spacing included.
pub type TextMeasure<'a> = &'a dyn Fn(&str, usize, f32, bool) -> Vec2;

/// Snapshots kept for undo.
const HISTORY: usize = 200;
/// Smallest box side a resize can reach, so a drag past the opposite side never mirrors the shape.
const MIN_SIDE: f32 = 1.0;
/// Smallest text size a resize can reach.
const MIN_TEXT: f32 = 4.0;

/// Bounds in picture pixels, including half the stroke and the text background.
pub fn ann_bounds(a: &Ann, m: TextMeasure) -> Rect {
    let half = a.width * 0.5;
    match &a.shape {
        Shape::Pen(p) => points_rect(p).expand(half),
        Shape::Marker(p) => points_rect(p).expand(half * MARKER_WIDTH_FACTOR),
        Shape::Line(p0, p1) => norm_rect(*p0, *p1).expand(half),
        Shape::Arrow(p0, p1) => {
            let (_, head) = arrow_geom(*p0, *p1, a.width);
            let mut r = norm_rect(*p0, *p1);
            for p in head {
                r.extend_with(p);
            }
            r.expand(half)
        }
        Shape::Rect(p0, p1) | Shape::Ellipse(p0, p1) => norm_rect(*p0, *p1).expand(half),
        Shape::Blur(p0, p1, _) => norm_rect(*p0, *p1),
        Shape::Text {
            pos,
            text,
            font,
            size,
            boxed,
        } => {
            let outer = m(text, *font, *size, *boxed);
            // The background reaches out from `pos` by its spacing on every side.
            let pad = if *boxed {
                (outer - m(text, *font, *size, false)) * 0.5
            } else {
                Vec2::ZERO
            };
            Rect::from_min_size(*pos - pad, outer)
        }
        Shape::Counter { center, .. } => {
            let r = counter_radius(a.width);
            Rect::from_center_size(*center, vec2(r * 2.0, r * 2.0))
        }
    }
}

fn points_rect(p: &[Pos2]) -> Rect {
    let mut r = Rect::NOTHING;
    for q in p {
        r.extend_with(*q);
    }
    r
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 <= f32::EPSILON {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn dist_to_polyline(p: Pos2, pts: &[Pos2]) -> f32 {
    match pts {
        [] => f32::INFINITY,
        [only] => p.distance(*only),
        _ => pts
            .windows(2)
            .map(|w| dist_to_segment(p, w[0], w[1]))
            .fold(f32::INFINITY, f32::min),
    }
}

fn dist_to_rect_border(p: Pos2, r: Rect) -> f32 {
    if r.contains(p) {
        (p.x - r.min.x)
            .min(r.max.x - p.x)
            .min(p.y - r.min.y)
            .min(r.max.y - p.y)
    } else {
        r.distance_to_pos(p)
    }
}

/// Distance along the ray from the centre: exact on the axes, close enough elsewhere for picking.
fn dist_to_ellipse_border(p: Pos2, r: Rect) -> f32 {
    let c = r.center();
    let (rx, ry) = (r.width() * 0.5, r.height() * 0.5);
    if rx <= f32::EPSILON || ry <= f32::EPSILON {
        return dist_to_segment(p, r.min, r.max);
    }
    let d = p - c;
    let k = ((d.x / rx).powi(2) + (d.y / ry).powi(2)).sqrt();
    if k <= f32::EPSILON {
        return rx.min(ry);
    }
    (1.0 - 1.0 / k).abs() * d.length()
}

fn inside_ellipse(p: Pos2, r: Rect) -> bool {
    let c = r.center();
    let (rx, ry) = (r.width() * 0.5, r.height() * 0.5);
    rx > 0.0 && ry > 0.0 && ((p.x - c.x) / rx).powi(2) + ((p.y - c.y) / ry).powi(2) <= 1.0
}

fn point_in_triangle(p: Pos2, t: [Pos2; 3]) -> bool {
    // A zero-length arrow collapses its head to a point, which would contain every point.
    if (t[1] - t[0]).x * (t[2] - t[0]).y - (t[1] - t[0]).y * (t[2] - t[0]).x == 0.0 {
        return false;
    }
    let side = |a: Pos2, b: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let (d0, d1, d2) = (side(t[0], t[1]), side(t[1], t[2]), side(t[2], t[0]));
    let neg = d0 < 0.0 || d1 < 0.0 || d2 < 0.0;
    let pos = d0 > 0.0 || d1 > 0.0 || d2 > 0.0;
    !(neg && pos)
}

fn hits(a: &Ann, p: Pos2, tol: f32, m: TextMeasure) -> bool {
    let reach = tol + a.width * 0.5;
    match &a.shape {
        Shape::Pen(pts) => dist_to_polyline(p, pts) <= reach,
        Shape::Marker(pts) => dist_to_polyline(p, pts) <= tol + a.width * MARKER_WIDTH_FACTOR * 0.5,
        Shape::Line(p0, p1) => dist_to_segment(p, *p0, *p1) <= reach,
        Shape::Arrow(p0, p1) => {
            let (_, head) = arrow_geom(*p0, *p1, a.width);
            dist_to_segment(p, *p0, *p1) <= reach || point_in_triangle(p, head)
        }
        Shape::Rect(p0, p1) => {
            let r = norm_rect(*p0, *p1);
            (a.filled && r.contains(p)) || dist_to_rect_border(p, r) <= reach
        }
        Shape::Ellipse(p0, p1) => {
            let r = norm_rect(*p0, *p1);
            (a.filled && inside_ellipse(p, r)) || dist_to_ellipse_border(p, r) <= reach
        }
        Shape::Blur(..) | Shape::Text { .. } | Shape::Counter { .. } => {
            ann_bounds(a, m).expand(tol).contains(p)
        }
    }
}

fn corners(r: Rect) -> Vec<(Handle, Pos2)> {
    vec![
        (Handle::NW, r.left_top()),
        (Handle::NE, r.right_top()),
        (Handle::SE, r.right_bottom()),
        (Handle::SW, r.left_bottom()),
    ]
}

fn sides_and_corners(r: Rect) -> Vec<(Handle, Pos2)> {
    vec![
        (Handle::NW, r.left_top()),
        (Handle::N, r.center_top()),
        (Handle::NE, r.right_top()),
        (Handle::E, r.right_center()),
        (Handle::SE, r.right_bottom()),
        (Handle::S, r.center_bottom()),
        (Handle::SW, r.left_bottom()),
        (Handle::W, r.left_center()),
    ]
}

/// The corner that stays put while corner `h` is dragged, and which way `h` points from it.
fn anchor(r: Rect, h: Handle) -> Option<(Pos2, Vec2)> {
    match h {
        Handle::SE => Some((r.left_top(), vec2(1.0, 1.0))),
        Handle::NW => Some((r.right_bottom(), vec2(-1.0, -1.0))),
        Handle::NE => Some((r.left_bottom(), vec2(1.0, -1.0))),
        Handle::SW => Some((r.right_top(), vec2(-1.0, 1.0))),
        _ => None,
    }
}

/// Moves the sides that `h` controls to `to`, never past the opposite side.
fn resize_box(r: Rect, h: Handle, to: Pos2, keep_ratio: bool) -> Rect {
    let (n, s, w, e) = match h {
        Handle::N => (true, false, false, false),
        Handle::S => (false, true, false, false),
        Handle::W => (false, false, true, false),
        Handle::E => (false, false, false, true),
        Handle::NW => (true, false, true, false),
        Handle::NE => (true, false, false, true),
        Handle::SW => (false, true, true, false),
        Handle::SE => (false, true, false, true),
        Handle::Start | Handle::End => return r,
    };
    let (mut min, mut max) = (r.min, r.max);
    if n {
        min.y = to.y.min(max.y - MIN_SIDE);
    }
    if s {
        max.y = to.y.max(min.y + MIN_SIDE);
    }
    if w {
        min.x = to.x.min(max.x - MIN_SIDE);
    }
    if e {
        max.x = to.x.max(min.x + MIN_SIDE);
    }
    let out = Rect::from_min_max(min, max);
    match anchor(r, h) {
        Some((fixed, dir)) if keep_ratio && r.width() > 0.0 && r.height() > 0.0 => {
            let k = (out.width() / r.width()).max(out.height() / r.height());
            let far = fixed + vec2(dir.x * r.width() * k, dir.y * r.height() * k);
            Rect::from_two_pos(fixed, far)
        }
        _ => out,
    }
}

/// Fixed corner and per-axis scale that take corner `h` of `r` to `to`, never flipping.
fn corner_scale(r: Rect, h: Handle, to: Pos2, keep_ratio: bool) -> Option<(Pos2, Vec2)> {
    let (fixed, dir) = anchor(r, h)?;
    let axis = |span: f32, reach: f32| {
        if span <= f32::EPSILON {
            1.0
        } else {
            (reach / span).max(0.01)
        }
    };
    let mut k = vec2(
        axis(r.width(), (to.x - fixed.x) * dir.x),
        axis(r.height(), (to.y - fixed.y) * dir.y),
    );
    if keep_ratio {
        let u = k.x.max(k.y);
        k = vec2(u, u);
    }
    Some((fixed, k))
}

#[derive(Default)]
pub struct Doc {
    objs: Vec<Obj>,
    next_id: ObjId,
    selected: Vec<ObjId>,
    undo: Vec<Vec<Obj>>,
    redo: Vec<Vec<Obj>>,
    revision: u64,
    /// While a gesture is held (a slider dragged), whether it already saved its undo step.
    gesture: Option<bool>,
}

/// What a restyle changes on the selected objects; `None` leaves that part as it is. The colour
/// and the opacity are apart, so a multi-selection keeps each object's other half. The text
/// options reach only texts and the blur mode only blurs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Restyle {
    pub rgb: Option<[u8; 3]>,
    pub alpha: Option<u8>,
    pub width: Option<f32>,
    pub filled: Option<bool>,
    pub font: Option<usize>,
    pub font_size: Option<f32>,
    pub text_box: Option<bool>,
    pub blur_mode: Option<BlurMode>,
}

impl Restyle {
    fn differs(&self, a: &Ann) -> bool {
        let shape = match &a.shape {
            Shape::Text {
                font, size, boxed, ..
            } => {
                self.font.is_some_and(|f| f != *font)
                    || self.font_size.is_some_and(|s| s != *size)
                    || self.text_box.is_some_and(|b| b != *boxed)
            }
            Shape::Blur(_, _, m) => self.blur_mode.is_some_and(|b| b != *m),
            _ => false,
        };
        self.rgb.is_some_and(|c| c[..] != a.color[..3])
            || self.alpha.is_some_and(|al| al != a.color[3])
            || self.width.is_some_and(|w| w != a.width)
            || self.filled.is_some_and(|f| f != a.filled)
            || shape
    }

    fn apply(&self, a: &mut Ann) {
        if let Some([r, g, b]) = self.rgb {
            a.color = [r, g, b, a.color[3]];
        }
        if let Some(al) = self.alpha {
            a.color[3] = al;
        }
        if let Some(w) = self.width {
            a.width = w;
        }
        if let Some(f) = self.filled {
            a.filled = f;
        }
        match &mut a.shape {
            Shape::Text {
                font, size, boxed, ..
            } => {
                if let Some(f) = self.font {
                    *font = f;
                }
                if let Some(s) = self.font_size {
                    *size = s;
                }
                if let Some(b) = self.text_box {
                    *boxed = b;
                }
            }
            Shape::Blur(_, _, m) => {
                if let Some(b) = self.blur_mode {
                    *m = b;
                }
            }
            _ => {}
        }
    }
}

impl Doc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_anns(anns: Vec<Ann>) -> Self {
        let mut d = Self::new();
        for ann in anns {
            let id = d.fresh_id();
            d.objs.push(Obj {
                id,
                ann,
                hidden: false,
            });
        }
        d
    }

    /// Objects from back to front.
    pub fn objs(&self) -> &[Obj] {
        &self.objs
    }

    /// What an object is, as the tool that draws it, and the text it shows (a text's content,
    /// a step's number), for the object list.
    pub fn kind(&self, id: ObjId) -> Option<(Tool, Option<String>)> {
        let o = self.get(id)?;
        Some(match &o.ann.shape {
            Shape::Pen(_) => (Tool::Pen, None),
            Shape::Marker(_) => (Tool::Marker, None),
            Shape::Line(..) => (Tool::Line, None),
            Shape::Arrow(..) => (Tool::Arrow, None),
            Shape::Rect(..) => (Tool::Rect, None),
            Shape::Ellipse(..) => (Tool::Ellipse, None),
            Shape::Blur(..) => (Tool::Blur, None),
            Shape::Text { text, .. } => (Tool::Text, Some(text.clone())),
            Shape::Counter { n, .. } => (Tool::Counter, Some(n.to_string())),
        })
    }

    /// Where an object stands in the list, front-most first, and how many there are ("2 of 3").
    pub fn position_of(&self, id: ObjId) -> Option<(usize, usize)> {
        let i = self.objs.iter().position(|o| o.id == id)?;
        Some((self.objs.len() - i, self.objs.len()))
    }

    /// Annotations to render: visible objects, minus `except` (the ones painted live during a drag).
    pub fn visible_anns(&self, except: &[ObjId]) -> Vec<Ann> {
        self.objs
            .iter()
            .filter(|o| !o.hidden && !except.contains(&o.id))
            .map(|o| o.ann.clone())
            .collect()
    }

    /// Adds a finished annotation and selects only it; a click without a drag is dropped.
    pub fn add(&mut self, ann: Ann) -> Option<ObjId> {
        if ann.is_degenerate() {
            return None;
        }
        self.checkpoint();
        let id = self.fresh_id();
        self.objs.push(Obj {
            id,
            ann,
            hidden: false,
        });
        self.selected = vec![id];
        self.changed();
        Some(id)
    }

    pub fn selected(&self) -> &[ObjId] {
        &self.selected
    }

    /// `None` clears; `additive` (Shift+click) adds `id` to the selection instead of replacing it.
    /// Hidden or missing objects are never selected.
    pub fn select(&mut self, id: Option<ObjId>, additive: bool) {
        match (id, additive) {
            (None, false) => self.selected.clear(),
            (None, true) => {}
            (Some(id), _) if !self.get(id).is_some_and(|o| !o.hidden) => {}
            (Some(id), false) => self.selected = vec![id],
            (Some(id), true) => {
                if !self.selected.contains(&id) {
                    self.selected.push(id);
                }
            }
        }
    }

    /// The front-most visible object under `p`.
    pub fn hit(&self, p: Pos2, tol: f32, m: TextMeasure) -> Option<ObjId> {
        self.objs
            .iter()
            .rev()
            .find(|o| !o.hidden && hits(&o.ann, p, tol, m))
            .map(|o| o.id)
    }

    pub fn handles(&self, id: ObjId, m: TextMeasure) -> Vec<(Handle, Pos2)> {
        let Some(o) = self.get(id) else {
            return Vec::new();
        };
        match &o.ann.shape {
            Shape::Line(a, b) | Shape::Arrow(a, b) => {
                vec![(Handle::Start, *a), (Handle::End, *b)]
            }
            Shape::Rect(a, b) | Shape::Ellipse(a, b) | Shape::Blur(a, b, _) => {
                sides_and_corners(norm_rect(*a, *b))
            }
            Shape::Pen(p) | Shape::Marker(p) => corners(points_rect(p)),
            Shape::Text { .. } | Shape::Counter { .. } => corners(ann_bounds(&o.ann, m)),
        }
    }

    /// The handle of a selected, visible object within `tol` of `p`, front-most first.
    pub fn handle_at(&self, p: Pos2, tol: f32, m: TextMeasure) -> Option<(ObjId, Handle)> {
        self.objs
            .iter()
            .rev()
            .filter(|o| !o.hidden && self.selected.contains(&o.id))
            .find_map(|o| {
                self.handles(o.id, m)
                    .into_iter()
                    .find(|(_, at)| at.distance(p) <= tol)
                    .map(|(h, _)| (o.id, h))
            })
    }

    /// Saves the state before a drag, so the whole drag undoes in one step.
    pub fn begin_change(&mut self) {
        self.checkpoint();
    }

    /// Moves the selection; call `begin_change` once before a drag.
    pub fn translate_selected(&mut self, d: Vec2) {
        if self.selected.is_empty() || d == Vec2::ZERO {
            return;
        }
        let sel = &self.selected;
        for o in self.objs.iter_mut().filter(|o| sel.contains(&o.id)) {
            o.ann.translate(d);
        }
        self.changed();
    }

    /// Drags handle `h` of `id` to `to`; call `begin_change` once before a drag.
    pub fn resize(&mut self, id: ObjId, h: Handle, to: Pos2, keep_ratio: bool, m: TextMeasure) {
        let Some(i) = self.objs.iter().position(|o| o.id == id) else {
            return;
        };
        let bounds = ann_bounds(&self.objs[i].ann, m);
        let a = &mut self.objs[i].ann;
        match &mut a.shape {
            Shape::Line(p0, p1) | Shape::Arrow(p0, p1) => {
                let (moving, fixed) = match h {
                    Handle::Start => (p0, *p1),
                    Handle::End => (p1, *p0),
                    _ => return,
                };
                *moving = if keep_ratio {
                    constrain(Tool::Line, fixed, to, true, false).1
                } else {
                    to
                };
            }
            Shape::Rect(p0, p1) | Shape::Ellipse(p0, p1) | Shape::Blur(p0, p1, _) => {
                let r = resize_box(norm_rect(*p0, *p1), h, to, keep_ratio);
                *p0 = r.min;
                *p1 = r.max;
            }
            Shape::Pen(pts) | Shape::Marker(pts) => {
                let Some((fixed, k)) = corner_scale(points_rect(pts), h, to, keep_ratio) else {
                    return;
                };
                for p in pts.iter_mut() {
                    *p = fixed + vec2((p.x - fixed.x) * k.x, (p.y - fixed.y) * k.y);
                }
            }
            Shape::Text { size, .. } => {
                let Some((fixed, dir)) = anchor(bounds, h) else {
                    return;
                };
                let height = ((to.y - fixed.y) * dir.y).max(MIN_SIDE);
                *size = (*size * height / bounds.height().max(MIN_SIDE)).max(MIN_TEXT);
                // Keep the opposite corner where it was: the size change moves it with the background spacing.
                let moved = anchor(ann_bounds(a, m), h).map_or(fixed, |(p, _)| p);
                a.translate(fixed - moved);
            }
            Shape::Counter { center, .. } => {
                let Some((fixed, dir)) = anchor(bounds, h) else {
                    return;
                };
                let side = ((to.x - fixed.x) * dir.x).max((to.y - fixed.y) * dir.y);
                a.width = ((side * 0.5 - 11.0) / 2.2).max(1.0);
                let r = counter_radius(a.width);
                *center = fixed + vec2(dir.x * r, dir.y * r);
            }
        }
        self.changed();
    }

    /// Starts a gesture (a slider or a colour picker held): its restyles undo in one step, saved at
    /// its first change. Beginning one already open changes nothing (the picker begins it on every
    /// frame it changes).
    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(false);
        }
    }

    pub fn end_gesture(&mut self) {
        self.gesture = None;
    }

    pub fn restyle_selected(&mut self, r: &Restyle) {
        if !self
            .objs
            .iter()
            .any(|o| self.selected.contains(&o.id) && r.differs(&o.ann))
        {
            return;
        }
        match self.gesture {
            Some(true) => {}
            Some(false) => {
                self.checkpoint();
                self.gesture = Some(true);
            }
            None => self.checkpoint(),
        }
        let sel = &self.selected;
        for o in self.objs.iter_mut().filter(|o| sel.contains(&o.id)) {
            r.apply(&mut o.ann);
        }
        self.changed();
    }

    /// Copies the selection on top, offset by `offset`, and selects the copies.
    /// A copied numbered step takes the next free number, so numbers never repeat.
    pub fn duplicate_selected(&mut self, offset: Vec2) -> Vec<ObjId> {
        if self.selected.is_empty() {
            return Vec::new();
        }
        self.checkpoint();
        let copies: Vec<Obj> = self
            .objs
            .iter()
            .filter(|o| self.selected.contains(&o.id))
            .cloned()
            .collect();
        let mut ids = Vec::with_capacity(copies.len());
        for mut o in copies {
            o.id = self.fresh_id();
            o.ann.translate(offset);
            if let Shape::Counter { n, .. } = &mut o.ann.shape {
                *n = self.next_counter();
            }
            ids.push(o.id);
            self.objs.push(o);
        }
        self.selected = ids.clone();
        self.changed();
        ids
    }

    pub fn delete_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.checkpoint();
        let sel = std::mem::take(&mut self.selected);
        self.objs.retain(|o| !sel.contains(&o.id));
        self.changed();
    }

    pub fn bring_to_front(&mut self) {
        self.reorder(false);
    }

    pub fn send_to_back(&mut self) {
        self.reorder(true);
    }

    pub fn set_hidden(&mut self, id: ObjId, hidden: bool) {
        let Some(i) = self.objs.iter().position(|o| o.id == id) else {
            return;
        };
        if self.objs[i].hidden == hidden {
            return;
        }
        self.checkpoint();
        self.objs[i].hidden = hidden;
        if hidden {
            self.selected.retain(|s| *s != id);
        }
        self.changed();
    }

    /// Removes `id` so its text can be typed again; adding it back is a new step.
    pub fn take_for_edit(&mut self, id: ObjId) -> Option<Ann> {
        let i = self.objs.iter().position(|o| o.id == id)?;
        self.checkpoint();
        let o = self.objs.remove(i);
        self.selected.retain(|s| *s != id);
        self.changed();
        Some(o.ann)
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        let now = std::mem::replace(&mut self.objs, prev);
        self.redo.push(now);
        self.restored();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        let now = std::mem::replace(&mut self.objs, next);
        self.undo.push(now);
        self.restored();
        true
    }

    /// The same document in coordinates shifted by `d`, history included, so undo keeps working
    /// after the capture hands its objects to the editor (which crops the picture).
    pub fn translated(mut self, d: Vec2) -> Self {
        let shift = |objs: &mut Vec<Obj>| objs.iter_mut().for_each(|o| o.ann.translate(d));
        shift(&mut self.objs);
        self.undo.iter_mut().for_each(shift);
        self.redo.iter_mut().for_each(shift);
        self.changed();
        self
    }

    /// Rises with every change to the objects, undo and redo included; selecting does not count.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The highest step number plus one, so deleting a step never reuses its number.
    pub fn next_counter(&self) -> u32 {
        self.objs
            .iter()
            .filter_map(|o| match o.ann.shape {
                Shape::Counter { n, .. } => Some(n),
                _ => None,
            })
            .max()
            .map_or(1, |n| n + 1)
    }

    fn get(&self, id: ObjId) -> Option<&Obj> {
        self.objs.iter().find(|o| o.id == id)
    }

    fn fresh_id(&mut self) -> ObjId {
        self.next_id += 1;
        self.next_id
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.objs.clone());
        if self.undo.len() > HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn changed(&mut self) {
        self.revision += 1;
    }

    fn restored(&mut self) {
        let objs = &self.objs;
        self.selected
            .retain(|s| objs.iter().any(|o| o.id == *s && !o.hidden));
        self.changed();
    }

    /// Moves the selection to the back (`to_back`) or the front, keeping each group's order.
    fn reorder(&mut self, to_back: bool) {
        let (sel, rest): (Vec<Obj>, Vec<Obj>) = self
            .objs
            .iter()
            .cloned()
            .partition(|o| self.selected.contains(&o.id));
        let order: Vec<Obj> = if to_back {
            sel.into_iter().chain(rest).collect()
        } else {
            rest.into_iter().chain(sel).collect()
        };
        if order
            .iter()
            .map(|o| o.id)
            .eq(self.objs.iter().map(|o| o.id))
        {
            return;
        }
        self.checkpoint();
        self.objs = order;
        self.changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::{Ann, BlurMode, Shape};
    use egui::{Pos2, Rect, Vec2, pos2, vec2};

    fn m(t: &str, _f: usize, size: f32, boxed: bool) -> Vec2 {
        let w = t.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f32 * size * 0.5;
        let h = t.lines().count() as f32 * size * 1.2;
        let pad = if boxed { 8.0 } else { 0.0 };
        vec2(w + pad, h + pad)
    }

    fn ann(shape: Shape) -> Ann {
        Ann {
            shape,
            color: [255, 0, 0, 255],
            width: 4.0,
            filled: false,
        }
    }

    fn rect(a: (f32, f32), b: (f32, f32)) -> Ann {
        ann(Shape::Rect(pos2(a.0, a.1), pos2(b.0, b.1)))
    }

    fn counter(n: u32) -> Ann {
        ann(Shape::Counter {
            center: pos2(50.0 * n as f32, 50.0),
            n,
        })
    }

    #[test]
    fn kinds_name_the_tool_and_the_text() {
        let mut d = Doc::new();
        let arrow = d
            .add(ann(Shape::Arrow(pos2(0.0, 0.0), pos2(40.0, 40.0))))
            .unwrap();
        let text = d
            .add(ann(Shape::Text {
                pos: pos2(10.0, 10.0),
                text: "Pico em outubro".into(),
                font: 0,
                size: 20.0,
                boxed: false,
            }))
            .unwrap();
        let step = d.add(counter(1)).unwrap();
        assert_eq!(d.kind(arrow), Some((Tool::Arrow, None)));
        assert_eq!(
            d.kind(text),
            Some((Tool::Text, Some("Pico em outubro".into())))
        );
        assert_eq!(d.kind(step), Some((Tool::Counter, Some("1".into()))));
        assert_eq!(d.kind(999), None);
    }

    #[test]
    fn position_counts_from_the_top() {
        // "2 de 3": the list shows the front-most object first.
        let mut d = Doc::new();
        let back = d.add(rect((0.0, 0.0), (10.0, 10.0))).unwrap();
        let middle = d.add(rect((20.0, 0.0), (30.0, 10.0))).unwrap();
        let front = d.add(rect((40.0, 0.0), (50.0, 10.0))).unwrap();
        assert_eq!(d.position_of(front), Some((1, 3)));
        assert_eq!(d.position_of(middle), Some((2, 3)));
        assert_eq!(d.position_of(back), Some((3, 3)));
        assert_eq!(d.position_of(999), None);
    }

    fn bounds_of(d: &Doc, id: ObjId) -> Rect {
        let o = d.objs().iter().find(|o| o.id == id).unwrap();
        ann_bounds(&o.ann, &m)
    }

    /// Ids, order and colour of every object: what undo must restore.
    fn state(d: &Doc) -> Vec<(ObjId, [u8; 4], bool)> {
        d.objs()
            .iter()
            .map(|o| (o.id, o.ann.color, o.hidden))
            .collect()
    }

    fn near(a: Pos2, b: Pos2) -> bool {
        a.distance(b) < 0.01
    }

    #[test]
    fn add_selects_new_and_rejects_degenerate() {
        let mut d = Doc::new();
        let id = d.add(rect((10.0, 10.0), (60.0, 40.0))).unwrap();
        assert_eq!(d.selected(), &[id]);
        let line = ann(Shape::Line(pos2(5.0, 5.0), pos2(6.0, 5.0)));
        assert_eq!(d.add(line), None);
        assert_eq!(d.objs().len(), 1);
        assert_eq!(d.selected(), &[id]);
    }

    #[test]
    fn hit_prefers_topmost_and_respects_tolerance() {
        let mut d = Doc::new();
        let _a = d.add(rect((0.0, 0.0), (100.0, 100.0))).unwrap();
        let b = d.add(rect((0.0, 0.0), (50.0, 100.0))).unwrap();
        assert_eq!(d.hit(pos2(0.0, 50.0), 2.0, &m), Some(b));
        // 10 px inside the left edge: farther than tolerance 4 + half width 2.
        assert_eq!(d.hit(pos2(10.0, 50.0), 4.0, &m), None);
    }

    #[test]
    fn filled_rect_and_blur_hit_inside_outline_rect_only_on_border() {
        let mut d = Doc::new();
        let outline = d.add(rect((0.0, 0.0), (100.0, 100.0))).unwrap();
        assert_eq!(d.hit(pos2(50.0, 50.0), 2.0, &m), None);
        assert_eq!(d.hit(pos2(100.0, 50.0), 2.0, &m), Some(outline));

        let mut d = Doc::new();
        let mut filled = rect((0.0, 0.0), (100.0, 100.0));
        filled.filled = true;
        let f = d.add(filled).unwrap();
        assert_eq!(d.hit(pos2(50.0, 50.0), 2.0, &m), Some(f));

        let mut d = Doc::new();
        let blur = d
            .add(ann(Shape::Blur(
                pos2(0.0, 0.0),
                pos2(100.0, 100.0),
                BlurMode::Blur,
            )))
            .unwrap();
        assert_eq!(d.hit(pos2(50.0, 50.0), 2.0, &m), Some(blur));
        assert_eq!(d.hit(pos2(150.0, 50.0), 2.0, &m), None);
    }

    #[test]
    fn arrow_hit_near_shaft() {
        let mut d = Doc::new();
        let id = d
            .add(ann(Shape::Arrow(pos2(0.0, 0.0), pos2(100.0, 0.0))))
            .unwrap();
        assert_eq!(d.hit(pos2(50.0, 3.0), 2.0, &m), Some(id));
        assert_eq!(d.hit(pos2(50.0, 12.0), 2.0, &m), None);
    }

    #[test]
    fn ellipse_hit_on_border_only_unless_filled() {
        let mut d = Doc::new();
        let e = ann(Shape::Ellipse(pos2(0.0, 0.0), pos2(200.0, 100.0)));
        let id = d.add(e.clone()).unwrap();
        assert_eq!(d.hit(pos2(200.0, 50.0), 2.0, &m), Some(id));
        assert_eq!(d.hit(pos2(100.0, 1.0), 2.0, &m), Some(id));
        assert_eq!(d.hit(pos2(100.0, 50.0), 2.0, &m), None);
        assert_eq!(d.hit(pos2(5.0, 5.0), 2.0, &m), None);

        let mut d = Doc::new();
        let mut filled = e;
        filled.filled = true;
        let id = d.add(filled).unwrap();
        assert_eq!(d.hit(pos2(100.0, 50.0), 2.0, &m), Some(id));
    }

    #[test]
    fn one_drag_is_one_undo_step() {
        let mut d = Doc::new();
        let id = d.add(rect((10.0, 10.0), (60.0, 40.0))).unwrap();
        let before = bounds_of(&d, id);
        d.begin_change();
        for _ in 0..30 {
            d.translate_selected(vec2(10.0, 0.0));
        }
        assert!(near(bounds_of(&d, id).min, before.min + vec2(300.0, 0.0)));
        assert!(d.undo());
        assert!(near(bounds_of(&d, id).min, before.min));
        assert!(d.redo());
        assert!(near(bounds_of(&d, id).min, before.min + vec2(300.0, 0.0)));
    }

    #[test]
    fn resize_rect_corner_and_line_endpoint() {
        let mut d = Doc::new();
        let id = d.add(rect((10.0, 10.0), (60.0, 40.0))).unwrap();
        d.resize(id, Handle::SE, pos2(110.0, 90.0), false, &m);
        let b = bounds_of(&d, id);
        assert!(near(b.min, pos2(8.0, 8.0)), "{b:?}");
        assert!(near(b.max, pos2(112.0, 92.0)), "{b:?}");
        // Dragging past the opposite corner stops at the minimum size instead of mirroring.
        d.resize(id, Handle::SE, pos2(0.0, 0.0), false, &m);
        let b = bounds_of(&d, id);
        assert!(near(b.min, pos2(8.0, 8.0)), "{b:?}");
        assert!(b.width() > 4.0 && b.height() > 4.0, "{b:?}");

        let mut d = Doc::new();
        let id = d
            .add(ann(Shape::Line(pos2(0.0, 0.0), pos2(100.0, 0.0))))
            .unwrap();
        d.resize(id, Handle::End, pos2(200.0, 0.0), false, &m);
        match &d.objs()[0].ann.shape {
            Shape::Line(a, b) => {
                assert!(near(*a, pos2(0.0, 0.0)));
                assert!(near(*b, pos2(200.0, 0.0)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn resize_scales_pen_text_and_counter() {
        let mut d = Doc::new();
        let pen = ann(Shape::Pen(vec![
            pos2(0.0, 0.0),
            pos2(10.0, 10.0),
            pos2(20.0, 0.0),
        ]));
        let id = d.add(pen).unwrap();
        d.resize(id, Handle::SE, pos2(40.0, 20.0), false, &m);
        match &d.objs()[0].ann.shape {
            Shape::Pen(p) => {
                assert!(near(p[0], pos2(0.0, 0.0)));
                assert!(near(p[1], pos2(20.0, 20.0)));
                assert!(near(p[2], pos2(40.0, 0.0)));
            }
            other => panic!("{other:?}"),
        }

        // Text "ab" size 20 at (10,10) measures 20×24: dragging SE to a 48 px height doubles the size.
        let text = |pos| {
            ann(Shape::Text {
                pos,
                text: "ab".into(),
                font: 0,
                size: 20.0,
                boxed: false,
            })
        };
        let mut d = Doc::new();
        let id = d.add(text(pos2(10.0, 10.0))).unwrap();
        d.resize(id, Handle::SE, pos2(50.0, 58.0), false, &m);
        match &d.objs()[0].ann.shape {
            Shape::Text { pos, size, .. } => {
                assert!((size - 40.0).abs() < 0.01, "{size}");
                assert!(near(*pos, pos2(10.0, 10.0)), "{pos:?}");
            }
            other => panic!("{other:?}"),
        }
        // Dragging NW keeps the SE corner (30,34) fixed.
        let mut d = Doc::new();
        let id = d.add(text(pos2(10.0, 10.0))).unwrap();
        d.resize(id, Handle::NW, pos2(-10.0, -14.0), false, &m);
        let b = bounds_of(&d, id);
        assert!(near(b.max, pos2(30.0, 34.0)), "{b:?}");
        assert!(near(b.min, pos2(-10.0, -14.0)), "{b:?}");

        // Counter width 4 has radius 19.8; a 44 px box gives radius 22, so width 5, with NW fixed.
        let mut d = Doc::new();
        let id = d.add(counter(1)).unwrap();
        let nw = bounds_of(&d, id).min;
        d.resize(id, Handle::SE, nw + vec2(44.0, 44.0), false, &m);
        let o = &d.objs()[0].ann;
        assert!((o.width - 5.0).abs() < 0.01, "{}", o.width);
        assert!(near(bounds_of(&d, id).min, nw));
        d.resize(id, Handle::SE, nw + vec2(2.0, 2.0), false, &m);
        assert!((d.objs()[0].ann.width - 1.0).abs() < 0.01);
    }

    #[test]
    fn text_bounds_use_measure_with_box_padding() {
        let t = ann(Shape::Text {
            pos: pos2(100.0, 100.0),
            text: "ab\ncde".into(),
            font: 0,
            size: 20.0,
            boxed: true,
        });
        let b = ann_bounds(&t, &m);
        // The box is the measured size (30×48) plus the background spacing, 4 px on every side.
        assert!(near(b.min, pos2(96.0, 96.0)), "{b:?}");
        assert!((b.width() - 38.0).abs() < 0.01, "{b:?}");
        assert!((b.height() - 56.0).abs() < 0.01, "{b:?}");
    }

    #[test]
    fn hidden_objects_are_not_hit_nor_rendered() {
        let mut d = Doc::new();
        let mut filled = rect((0.0, 0.0), (100.0, 100.0));
        filled.filled = true;
        let id = d.add(filled).unwrap();
        d.set_hidden(id, true);
        assert_eq!(d.hit(pos2(50.0, 50.0), 2.0, &m), None);
        assert!(d.visible_anns(&[]).is_empty());
        assert!(d.selected().is_empty());
        d.set_hidden(id, false);
        assert_eq!(d.visible_anns(&[]).len(), 1);
        assert!(d.visible_anns(&[id]).is_empty());
    }

    #[test]
    fn a_held_gesture_undoes_in_one_step() {
        let mut d = Doc::from_anns(vec![rect((0.0, 0.0), (50.0, 50.0))]);
        let a = d.objs()[0].id;
        d.select(Some(a), false);
        let alpha = |d: &mut Doc, v: u8| {
            d.restyle_selected(&Restyle {
                alpha: Some(v),
                ..Default::default()
            })
        };
        d.begin_gesture();
        for v in [230, 200, 191] {
            alpha(&mut d, v);
        }
        d.end_gesture();
        assert_eq!(d.objs()[0].ann.color[3], 191);
        assert!(d.undo());
        assert_eq!(d.objs()[0].ann.color[3], 255);
        assert!(!d.undo(), "one step for the whole gesture");
        d.begin_gesture();
        d.end_gesture();
        assert!(!d.undo(), "a gesture that changed nothing leaves no step");
        alpha(&mut d, 100);
        alpha(&mut d, 50);
        assert!(d.undo());
        assert_eq!(
            d.objs()[0].ann.color[3],
            100,
            "outside a gesture every change is a step"
        );
    }

    #[test]
    fn a_gesture_begun_again_is_still_one_step() {
        // The colour picker begins its gesture on every frame it changes.
        let mut d = Doc::from_anns(vec![rect((0.0, 0.0), (50.0, 50.0))]);
        let a = d.objs()[0].id;
        d.select(Some(a), false);
        for v in [10, 20, 30] {
            d.begin_gesture();
            d.restyle_selected(&Restyle {
                rgb: Some([v, 0, 0]),
                ..Default::default()
            });
        }
        d.end_gesture();
        assert!(d.undo());
        assert_eq!(d.objs()[0].ann.color, [255, 0, 0, 255]);
        assert!(!d.undo());
    }

    #[test]
    fn restyle_reaches_the_text_and_blur_options() {
        let text = ann(Shape::Text {
            pos: pos2(0.0, 0.0),
            text: "hi".into(),
            font: 0,
            size: 20.0,
            boxed: false,
        });
        let blur = ann(Shape::Blur(
            pos2(100.0, 0.0),
            pos2(150.0, 50.0),
            BlurMode::Blur,
        ));
        let mut d = Doc::from_anns(vec![text, blur]);
        let (t, b) = (d.objs()[0].id, d.objs()[1].id);
        d.select(Some(t), false);
        d.select(Some(b), true);
        d.restyle_selected(&Restyle {
            font: Some(2),
            font_size: Some(32.0),
            text_box: Some(true),
            blur_mode: Some(BlurMode::Pixelate),
            ..Default::default()
        });
        match &d.objs()[0].ann.shape {
            Shape::Text {
                font, size, boxed, ..
            } => assert_eq!((*font, *size, *boxed), (2, 32.0, true)),
            s => panic!("{s:?}"),
        }
        assert!(matches!(
            d.objs()[1].ann.shape,
            Shape::Blur(_, _, BlurMode::Pixelate)
        ));
        assert!(d.undo());
        assert!(
            matches!(d.objs()[1].ann.shape, Shape::Blur(_, _, BlurMode::Blur)),
            "one step for the whole restyle"
        );
    }

    #[test]
    fn restyle_duplicate_delete_order_are_undoable() {
        let mut d = Doc::new();
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        let _b = d.add(rect((100.0, 0.0), (150.0, 50.0))).unwrap();
        d.select(Some(a), false);

        type Op = (&'static str, fn(&mut Doc));
        let ops: Vec<Op> = vec![
            ("restyle", |d| {
                d.restyle_selected(&Restyle {
                    rgb: Some([0, 0, 255]),
                    width: Some(8.0),
                    filled: Some(true),
                    ..Default::default()
                })
            }),
            ("duplicate", |d| {
                d.duplicate_selected(vec2(12.0, 12.0));
            }),
            ("delete", |d| d.delete_selected()),
            ("front", |d| d.bring_to_front()),
            ("back", |d| d.send_to_back()),
            ("hide", |d| {
                let id = d.objs()[0].id;
                d.set_hidden(id, true);
            }),
        ];
        for (name, op) in ops {
            d.select(Some(a), false);
            let before = state(&d);
            if name == "back" {
                d.bring_to_front();
                d.select(Some(a), false);
            }
            let mid = state(&d);
            op(&mut d);
            assert_ne!(state(&d), mid, "{name} changed nothing");
            assert!(d.undo(), "{name}");
            assert_eq!(state(&d), mid, "{name}");
            if name == "back" {
                assert!(d.undo());
                assert_eq!(state(&d), before);
            }
        }

        // Duplicate offsets the copy by 12 px and selects it.
        d.select(Some(a), false);
        let copies = d.duplicate_selected(vec2(12.0, 12.0));
        assert_eq!(d.selected(), copies.as_slice());
        let copy = bounds_of(&d, copies[0]);
        assert!(near(copy.min, bounds_of(&d, a).min + vec2(12.0, 12.0)));
    }

    #[test]
    fn counters_never_repeat_after_delete() {
        let mut d = Doc::new();
        let one = d.add(counter(1)).unwrap();
        assert_eq!(d.next_counter(), 2);
        let two = d.add(counter(2)).unwrap();
        let _three = d.add(counter(3)).unwrap();
        d.select(Some(two), false);
        d.delete_selected();
        assert_eq!(d.next_counter(), 4);
        // A duplicated step takes the next free number.
        d.select(Some(one), false);
        let copy = d.duplicate_selected(vec2(12.0, 12.0))[0];
        let n = match d.objs().iter().find(|o| o.id == copy).unwrap().ann.shape {
            Shape::Counter { n, .. } => n,
            _ => unreachable!(),
        };
        assert_eq!(n, 4);
        assert_eq!(Doc::new().next_counter(), 1);
    }

    #[test]
    fn revision_changes_on_every_mutation() {
        let mut d = Doc::new();
        let mut rev = d.revision();
        let mut bumped = |d: &Doc, what: &str| {
            assert!(d.revision() > rev, "{what} did not change the revision");
            rev = d.revision();
        };
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        bumped(&d, "add");
        let t = d
            .add(ann(Shape::Text {
                pos: pos2(200.0, 200.0),
                text: "hi".into(),
                font: 0,
                size: 20.0,
                boxed: false,
            }))
            .unwrap();
        bumped(&d, "add text");
        d.select(Some(a), false);
        d.begin_change();
        d.translate_selected(vec2(5.0, 0.0));
        bumped(&d, "translate");
        d.resize(a, Handle::SE, pos2(90.0, 90.0), false, &m);
        bumped(&d, "resize");
        d.restyle_selected(&Restyle {
            rgb: Some([0, 0, 255]),
            ..Default::default()
        });
        bumped(&d, "restyle");
        d.duplicate_selected(vec2(12.0, 12.0));
        bumped(&d, "duplicate");
        // The copy is already in front: send it back first so both moves change the order.
        d.send_to_back();
        bumped(&d, "back");
        d.bring_to_front();
        bumped(&d, "front");
        d.delete_selected();
        bumped(&d, "delete");
        d.set_hidden(a, true);
        bumped(&d, "hide");
        assert!(d.take_for_edit(t).is_some());
        bumped(&d, "take_for_edit");
        assert!(d.undo());
        bumped(&d, "undo");
        assert!(d.redo());
        bumped(&d, "redo");
        // Selecting is not an edit: it must not mark the picture as changed.
        let before = d.revision();
        d.select(Some(a), false);
        d.select(None, false);
        assert_eq!(d.revision(), before);
    }

    #[test]
    fn undo_selection_drops_ids_that_no_longer_exist() {
        let mut d = Doc::new();
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        assert_eq!(d.selected(), &[a]);
        assert!(d.undo());
        assert!(d.objs().is_empty());
        assert!(d.selected().is_empty());
        assert!(!d.undo());
        // A new change after undo drops the redo branch.
        d.add(rect((0.0, 0.0), (50.0, 50.0)));
        assert!(!d.redo());
    }

    #[test]
    fn from_anns_starts_clean() {
        let d = Doc::from_anns(vec![rect((0.0, 0.0), (50.0, 50.0)), counter(1)]);
        assert_eq!(d.objs().len(), 2);
        assert_ne!(d.objs()[0].id, d.objs()[1].id);
        assert!(d.selected().is_empty());
        assert_eq!(d.revision(), 0);
        assert_eq!(d.next_counter(), 2);
        let mut d = d;
        assert!(!d.undo());
        // Ids keep counting after the loaded objects.
        let id = d.add(rect((100.0, 0.0), (150.0, 50.0))).unwrap();
        assert!(d.objs()[..2].iter().all(|o| o.id != id));
    }

    #[test]
    fn history_keeps_200_steps() {
        let mut d = Doc::new();
        for i in 0..250 {
            let x = i as f32 * 10.0;
            d.add(rect((x, 0.0), (x + 5.0, 5.0)));
        }
        let mut steps = 0;
        while d.undo() {
            steps += 1;
        }
        assert_eq!(steps, 200);
        assert_eq!(d.objs().len(), 50);
    }

    /// Measure with the render's background spacing, which grows with the size.
    fn mp(t: &str, f: usize, size: f32, boxed: bool) -> Vec2 {
        let pad = if boxed {
            vec2(size * 0.4, size * 0.22) * 2.0
        } else {
            Vec2::ZERO
        };
        m(t, f, size, false) + pad
    }

    #[test]
    fn shift_click_adds_and_never_removes() {
        let mut d = Doc::new();
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        let b = d.add(rect((100.0, 0.0), (150.0, 50.0))).unwrap();
        d.select(Some(a), false);
        d.select(Some(b), true);
        assert_eq!(d.selected(), &[a, b]);
        d.select(Some(b), true);
        assert_eq!(d.selected(), &[a, b]);
        d.select(None, true);
        assert_eq!(d.selected(), &[a, b]);
        d.select(None, false);
        assert!(d.selected().is_empty());
    }

    #[test]
    fn zero_length_arrow_is_hit_only_where_it_is() {
        let mut d = Doc::new();
        let id = d
            .add(ann(Shape::Arrow(pos2(0.0, 0.0), pos2(100.0, 0.0))))
            .unwrap();
        d.resize(id, Handle::End, pos2(0.0, 0.0), false, &m);
        assert_eq!(d.hit(pos2(300.0, 300.0), 2.0, &m), None);
        assert_eq!(d.hit(pos2(1.0, 0.0), 2.0, &m), Some(id));
    }

    #[test]
    fn boxed_text_resize_keeps_the_opposite_corner() {
        let text = || {
            ann(Shape::Text {
                pos: pos2(100.0, 100.0),
                text: "ab".into(),
                font: 0,
                size: 20.0,
                boxed: true,
            })
        };
        for (h, keep) in [(Handle::SE, true), (Handle::NW, false)] {
            let mut d = Doc::new();
            let id = d.add(text()).unwrap();
            let before = ann_bounds(&d.objs()[0].ann, &mp);
            let fixed = if keep { before.min } else { before.max };
            let far = if keep { before.max } else { before.min };
            d.resize(id, h, far + (far - fixed), false, &mp);
            let after = ann_bounds(&d.objs()[0].ann, &mp);
            let now = if keep { after.min } else { after.max };
            assert!(near(now, fixed), "{h:?}: {fixed:?} moved to {now:?}");
            assert!(after.height() > before.height() * 1.5, "{h:?}: {after:?}");
        }
    }

    #[test]
    fn hidden_objects_never_stay_selected() {
        let mut d = Doc::new();
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        d.set_hidden(a, true);
        d.select(Some(a), false);
        assert!(d.selected().is_empty());
        assert!(d.undo());
        d.select(Some(a), false);
        assert!(d.redo());
        assert!(d.selected().is_empty(), "redo hid a selected object");
        assert_eq!(d.handle_at(pos2(50.0, 50.0), 4.0, &m), None);
    }

    #[test]
    fn text_and_step_stop_at_their_minimum_past_the_fixed_corner() {
        let mut d = Doc::new();
        let id = d
            .add(ann(Shape::Text {
                pos: pos2(100.0, 100.0),
                text: "ab".into(),
                font: 0,
                size: 20.0,
                boxed: false,
            }))
            .unwrap();
        d.resize(id, Handle::SE, pos2(0.0, 0.0), false, &m);
        match d.objs()[0].ann.shape {
            Shape::Text { size, .. } => assert!((size - MIN_TEXT).abs() < 0.01, "{size}"),
            _ => unreachable!(),
        }
        let mut d = Doc::new();
        let id = d.add(counter(1)).unwrap();
        let nw = bounds_of(&d, id).min;
        d.resize(id, Handle::SE, nw - vec2(100.0, 100.0), false, &m);
        assert!((d.objs()[0].ann.width - 1.0).abs() < 0.01);
    }

    #[test]
    fn edits_that_change_nothing_leave_the_picture_unchanged() {
        let mut d = Doc::new();
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        let b = d.add(rect((100.0, 0.0), (150.0, 50.0))).unwrap();
        let rev = d.revision();
        let check = |d: &Doc, what: &str| assert_eq!(d.revision(), rev, "{what}");
        d.select(None, false);
        d.translate_selected(vec2(5.0, 0.0));
        check(&d, "translate with nothing selected");
        d.select(Some(a), false);
        d.restyle_selected(&Restyle {
            rgb: Some([255, 0, 0]),
            alpha: Some(255),
            width: Some(4.0),
            filled: Some(false),
            ..Default::default()
        });
        check(&d, "restyle to the same style");
        d.send_to_back();
        check(&d, "send the back object to the back");
        d.select(Some(b), false);
        d.bring_to_front();
        check(&d, "bring the front object to the front");
        // Only the two adds are in the history.
        assert!(d.undo() && d.undo() && !d.undo());
    }

    #[test]
    fn reorder_keeps_the_order_inside_a_multi_selection() {
        let mut d = Doc::new();
        let a = d.add(rect((0.0, 0.0), (50.0, 50.0))).unwrap();
        let b = d.add(rect((100.0, 0.0), (150.0, 50.0))).unwrap();
        let c = d.add(rect((200.0, 0.0), (250.0, 50.0))).unwrap();
        let order = |d: &Doc| d.objs().iter().map(|o| o.id).collect::<Vec<_>>();
        d.select(Some(a), false);
        d.select(Some(b), true);
        d.bring_to_front();
        assert_eq!(order(&d), vec![c, a, b]);
        d.send_to_back();
        assert_eq!(order(&d), vec![a, b, c]);
    }

    #[test]
    fn translated_moves_objects_and_history_together() {
        let mut d = Doc::new();
        let a = d.add(rect((100.0, 100.0), (150.0, 150.0))).unwrap();
        d.begin_change();
        d.translate_selected(vec2(20.0, 0.0));
        let hidden = d.add(rect((300.0, 300.0), (350.0, 350.0))).unwrap();
        d.set_hidden(hidden, true);
        let rev = d.revision();
        let mut d = d.translated(vec2(-100.0, -100.0));
        assert!(near(bounds_of(&d, a).min, pos2(18.0, -2.0)));
        assert!(d.objs().iter().any(|o| o.id == hidden && o.hidden));
        assert!(d.revision() > rev);
        // Undo still lands where the object was, in the new coordinates.
        assert!(d.undo() && d.undo() && d.undo());
        assert!(near(bounds_of(&d, a).min, pos2(-2.0, -2.0)));
        assert!(d.redo());
        assert!(near(bounds_of(&d, a).min, pos2(18.0, -2.0)));
    }

    #[test]
    fn translated_also_moves_the_redo_branch() {
        let mut d = Doc::new();
        let a = d.add(rect((100.0, 100.0), (150.0, 150.0))).unwrap();
        d.begin_change();
        d.translate_selected(vec2(20.0, 0.0));
        assert!(d.undo());
        let mut d = d.translated(vec2(-100.0, -100.0));
        assert!(d.redo());
        assert!(near(bounds_of(&d, a).min, pos2(18.0, -2.0)));
    }

    #[test]
    fn handles_per_shape_and_only_on_selection() {
        let mut d = Doc::new();
        let line = d
            .add(ann(Shape::Line(pos2(0.0, 0.0), pos2(100.0, 0.0))))
            .unwrap();
        let kinds: Vec<Handle> = d.handles(line, &m).iter().map(|h| h.0).collect();
        assert_eq!(kinds, vec![Handle::Start, Handle::End]);
        assert_eq!(
            d.handle_at(pos2(99.0, 1.0), 4.0, &m),
            Some((line, Handle::End))
        );

        let r = d.add(rect((200.0, 0.0), (300.0, 100.0))).unwrap();
        assert_eq!(d.handles(r, &m).len(), 8);
        // The line is no longer selected, so its handles do not answer.
        assert_eq!(d.handle_at(pos2(99.0, 1.0), 4.0, &m), None);
        assert_eq!(
            d.handle_at(pos2(300.0, 50.0), 4.0, &m),
            Some((r, Handle::E))
        );

        let c = d.add(counter(1)).unwrap();
        assert_eq!(d.handles(c, &m).len(), 4);
        let pen = d
            .add(ann(Shape::Pen(vec![pos2(0.0, 300.0), pos2(40.0, 340.0)])))
            .unwrap();
        assert_eq!(d.handles(pen, &m).len(), 4);
    }
}
