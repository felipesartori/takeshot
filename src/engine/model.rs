//! Annotation data shared by the capture overlay and the editor.

use egui::{Pos2, Vec2, vec2};
use serde::{Deserialize, Serialize};

/// Default pen colour: opaque red.
pub const DEFAULT_COLOR: [u8; 4] = [255, 0, 0, 255];
/// Default pen width in pixels.
pub const DEFAULT_WIDTH: f32 = 3.0;
/// Arrow head size in pen widths: length 5, width 3.
pub const ARROW_CAP_LENGTH: f32 = 5.0;
pub const ARROW_CAP_WIDTH: f32 = 3.0;
/// Minimum head length so thin arrows keep a visible head.
pub const ARROW_MIN_HEAD: f32 = 14.0;
/// Marker opacity (about half).
pub const MARKER_ALPHA: u8 = 125;
/// How much thicker the marker is than the pen.
pub const MARKER_WIDTH_FACTOR: f32 = 5.0;
/// The screen outside the selection is dimmed to 50 %.
pub const DIM_ALPHA: f32 = 0.5;
/// Size label: dark translucent fill with light grey text.
pub const SIZE_LABEL_FILL: [u8; 4] = [0, 0, 0, 200];
pub const SIZE_LABEL_TEXT: [u8; 3] = [0xde, 0xde, 0xde];

pub const WIDTHS: [f32; 5] = [2.0, 3.0, 5.0, 8.0, 12.0];

pub const PALETTE: [[u8; 4]; 10] = [
    [255, 0, 0, 255],
    [255, 128, 0, 255],
    [255, 214, 10, 255],
    [52, 199, 89, 255],
    [0, 179, 255, 255],
    [45, 91, 255, 255],
    [175, 82, 222, 255],
    [255, 55, 155, 255],
    [255, 255, 255, 255],
    [17, 17, 17, 255],
];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tool {
    Select,
    Pen,
    Line,
    Arrow,
    Rect,
    Ellipse,
    Marker,
    Text,
    Blur,
    Counter,
}

impl Tool {
    pub const DRAWING: [Tool; 9] = [
        Tool::Pen,
        Tool::Line,
        Tool::Arrow,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Marker,
        Tool::Text,
        Tool::Blur,
        Tool::Counter,
    ];

    pub fn key(self) -> egui::Key {
        use egui::Key;
        match self {
            Tool::Select => Key::V,
            Tool::Pen => Key::P,
            Tool::Line => Key::L,
            Tool::Arrow => Key::A,
            Tool::Rect => Key::R,
            Tool::Ellipse => Key::O,
            Tool::Marker => Key::M,
            Tool::Text => Key::T,
            Tool::Blur => Key::B,
            Tool::Counter => Key::N,
        }
    }

    pub fn key_name(self) -> &'static str {
        match self {
            Tool::Select => "V",
            Tool::Pen => "P",
            Tool::Line => "L",
            Tool::Arrow => "A",
            Tool::Rect => "R",
            Tool::Ellipse => "O",
            Tool::Marker => "M",
            Tool::Text => "T",
            Tool::Blur => "B",
            Tool::Counter => "N",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BlurMode {
    Blur,
    Pixelate,
}

#[derive(Clone, Debug)]
pub enum Shape {
    Pen(Vec<Pos2>),
    Marker(Vec<Pos2>),
    Line(Pos2, Pos2),
    Arrow(Pos2, Pos2),
    Rect(Pos2, Pos2),
    Ellipse(Pos2, Pos2),
    Blur(Pos2, Pos2, BlurMode),
    Text {
        pos: Pos2,
        text: String,
        font: usize,
        size: f32,
        boxed: bool,
    },
    Counter {
        center: Pos2,
        n: u32,
    },
}

impl Shape {
    /// The tool that draws this shape, so the options bar can show its options.
    pub fn tool(&self) -> Tool {
        match self {
            Shape::Pen(_) => Tool::Pen,
            Shape::Marker(_) => Tool::Marker,
            Shape::Line(..) => Tool::Line,
            Shape::Arrow(..) => Tool::Arrow,
            Shape::Rect(..) => Tool::Rect,
            Shape::Ellipse(..) => Tool::Ellipse,
            Shape::Blur(..) => Tool::Blur,
            Shape::Text { .. } => Tool::Text,
            Shape::Counter { .. } => Tool::Counter,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ann {
    pub shape: Shape,
    pub color: [u8; 4],
    pub width: f32,
    pub filled: bool,
}

impl Ann {
    pub fn translate(&mut self, d: Vec2) {
        let mv = |p: &mut Pos2| *p += d;
        match &mut self.shape {
            Shape::Pen(pts) | Shape::Marker(pts) => pts.iter_mut().for_each(mv),
            Shape::Line(a, b)
            | Shape::Arrow(a, b)
            | Shape::Rect(a, b)
            | Shape::Ellipse(a, b)
            | Shape::Blur(a, b, _) => {
                *a += d;
                *b += d;
            }
            Shape::Text { pos, .. } => *pos += d,
            Shape::Counter { center, .. } => *center += d,
        }
    }

    /// True when the shape is too small to keep (a click without a drag).
    pub fn is_degenerate(&self) -> bool {
        match &self.shape {
            Shape::Pen(p) | Shape::Marker(p) => p.len() < 2,
            Shape::Line(a, b) | Shape::Arrow(a, b) => a.distance(*b) < 3.0,
            Shape::Rect(a, b) | Shape::Ellipse(a, b) | Shape::Blur(a, b, _) => {
                (a.x - b.x).abs() < 3.0 || (a.y - b.y).abs() < 3.0
            }
            Shape::Text { text, .. } => text.trim().is_empty(),
            Shape::Counter { .. } => false,
        }
    }
}

/// Holding Shift keeps lines at 45° steps and boxes/circles uniform; Alt draws from the centre.
pub fn constrain(tool: Tool, a: Pos2, b: Pos2, shift: bool, alt: bool) -> (Pos2, Pos2) {
    match tool {
        Tool::Line | Tool::Arrow | Tool::Pen | Tool::Marker => {
            if !shift {
                return (a, b);
            }
            let d = b - a;
            let len = d.length();
            let step = std::f32::consts::FRAC_PI_4;
            let ang = (d.y.atan2(d.x) / step).round() * step;
            (a, a + vec2(ang.cos(), ang.sin()) * len)
        }
        Tool::Rect | Tool::Ellipse | Tool::Blur | Tool::Select => {
            let mut d = b - a;
            if shift {
                let side = d.x.abs().max(d.y.abs());
                d = vec2(side * sign(d.x), side * sign(d.y));
            }
            if alt { (a - d, a + d) } else { (a, a + d) }
        }
        _ => (a, b),
    }
}

fn sign(v: f32) -> f32 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

pub fn norm_rect(a: Pos2, b: Pos2) -> egui::Rect {
    egui::Rect::from_two_pos(a, b)
}

/// Arrow geometry shared by the live preview and the final render: (shaft end, [tip, left, right]).
pub fn arrow_geom(a: Pos2, b: Pos2, width: f32) -> (Pos2, [Pos2; 3]) {
    let d = b - a;
    let len = d.length().max(0.001);
    let dir = d / len;
    let normal = vec2(-dir.y, dir.x);
    let head_len = (ARROW_CAP_LENGTH * width)
        .max(ARROW_MIN_HEAD)
        .min(len * 0.9);
    let half_w = head_len * (ARROW_CAP_WIDTH / ARROW_CAP_LENGTH) * 0.5 * 1.6;
    let base = b - dir * head_len;
    let shaft_end = b - dir * (head_len * 0.6);
    (
        shaft_end,
        [b, base + normal * half_w, base - normal * half_w],
    )
}

pub fn counter_radius(width: f32) -> f32 {
    11.0 + width * 2.2
}

pub fn luminance(c: [u8; 4]) -> f32 {
    (0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32) / 255.0
}

pub fn c32(c: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}
