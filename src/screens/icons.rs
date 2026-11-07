//! Hand-drawn line icons, painted with egui so they stay crisp at any scale.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
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
    Undo,
    Redo,
    Copy,
    Save,
    Edit,
    Pin,
    Close,
    Full,
    Timer,
    Open,
    Folder,
}

pub fn for_tool(t: crate::engine::model::Tool) -> Icon {
    use crate::engine::model::Tool;
    match t {
        Tool::Select => Icon::Select,
        Tool::Pen => Icon::Pen,
        Tool::Line => Icon::Line,
        Tool::Arrow => Icon::Arrow,
        Tool::Rect => Icon::Rect,
        Tool::Ellipse => Icon::Ellipse,
        Tool::Marker => Icon::Marker,
        Tool::Text => Icon::Text,
        Tool::Blur => Icon::Blur,
        Tool::Counter => Icon::Counter,
    }
}

pub fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 20.0;
    let c = rect.center();
    let at = |x: f32, y: f32| -> Pos2 { pos2(c.x + (x - 10.0) * s, c.y + (y - 10.0) * s) };
    let st = Stroke::new(1.6 * s, color);
    match icon {
        Icon::Select => {
            let pts = vec![
                at(5.5, 3.5),
                at(15.0, 11.5),
                at(10.6, 12.2),
                at(13.2, 17.0),
                at(11.4, 17.8),
                at(8.8, 13.0),
                at(5.5, 15.8),
            ];
            p.add(Shape::closed_line(pts, st));
        }
        Icon::Pen => {
            p.line_segment([at(4.5, 15.5), at(14.5, 5.5)], Stroke::new(2.0 * s, color));
            p.line_segment([at(13.0, 4.0), at(16.0, 7.0)], st);
            p.add(Shape::convex_polygon(
                vec![at(3.5, 16.5), at(4.0, 14.3), at(5.7, 16.0)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Line => {
            p.line_segment([at(4.5, 15.5), at(15.5, 4.5)], Stroke::new(1.9 * s, color));
        }
        Icon::Arrow => {
            p.line_segment([at(4.5, 15.5), at(13.0, 7.0)], Stroke::new(1.9 * s, color));
            p.add(Shape::convex_polygon(
                vec![at(16.0, 4.0), at(15.2, 10.4), at(9.6, 4.8)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Rect => {
            p.rect_stroke(
                Rect::from_min_max(at(4.0, 5.5), at(16.0, 14.5)),
                1.5 * s,
                st,
                egui::StrokeKind::Middle,
            );
        }
        Icon::Ellipse => {
            p.circle_stroke(at(10.0, 10.0), 6.2 * s, st);
        }
        Icon::Marker => {
            let hl = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 90);
            p.line_segment([at(3.5, 14.0), at(16.5, 14.0)], Stroke::new(4.5 * s, hl));
            p.add(Shape::closed_line(
                vec![at(8.0, 11.5), at(12.5, 4.0), at(15.0, 5.5), at(10.5, 13.0)],
                st,
            ));
        }
        Icon::Text => {
            p.line_segment([at(5.0, 5.0), at(15.0, 5.0)], Stroke::new(1.9 * s, color));
            p.line_segment([at(10.0, 5.0), at(10.0, 15.5)], Stroke::new(1.9 * s, color));
            p.line_segment([at(8.0, 15.5), at(12.0, 15.5)], st);
        }
        Icon::Blur => {
            for i in 0..3 {
                for j in 0..3 {
                    let a = [230u8, 120, 60, 150, 220, 110, 70, 130, 210][i * 3 + j];
                    let col = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a);
                    let r = Rect::from_min_size(
                        at(4.0 + j as f32 * 4.2, 4.0 + i as f32 * 4.2),
                        vec2(3.6 * s, 3.6 * s),
                    );
                    p.rect_filled(r, 0.8 * s, col);
                }
            }
        }
        Icon::Counter => {
            p.circle_stroke(at(10.0, 10.0), 6.8 * s, st);
            p.line_segment([at(8.6, 7.6), at(10.6, 6.4)], st);
            p.line_segment([at(10.6, 6.4), at(10.6, 13.8)], st);
        }
        Icon::Undo | Icon::Redo => {
            let flip = if icon == Icon::Redo { -1.0 } else { 1.0 };
            let fx = |x: f32| if flip < 0.0 { 20.0 - x } else { x };
            let mut pts = Vec::new();
            for k in 0..=12 {
                let t = std::f32::consts::PI * (1.0 + k as f32 / 12.0 * 1.0);
                pts.push(at(fx(10.5 + 4.6 * -t.cos()), 11.0 + 4.6 * t.sin()));
            }
            pts.reverse();
            p.add(Shape::line(pts, st));
            p.add(Shape::line(
                vec![at(fx(8.5), 3.5), at(fx(5.9), 6.4), at(fx(8.9), 8.8)],
                st,
            ));
            p.line_segment([at(fx(6.0), 6.4), at(fx(10.5), 6.4)], st);
        }
        Icon::Copy => {
            p.rect_stroke(
                Rect::from_min_max(at(7.0, 7.0), at(16.0, 16.0)),
                2.0 * s,
                st,
                egui::StrokeKind::Middle,
            );
            p.add(Shape::line(
                vec![at(4.0, 12.5), at(4.0, 5.5), at(5.5, 4.0), at(12.5, 4.0)],
                st,
            ));
        }
        Icon::Save => {
            p.line_segment([at(10.0, 3.5), at(10.0, 12.0)], st);
            p.add(Shape::line(
                vec![at(6.5, 8.8), at(10.0, 12.3), at(13.5, 8.8)],
                st,
            ));
            p.add(Shape::line(
                vec![at(4.0, 12.5), at(4.0, 16.0), at(16.0, 16.0), at(16.0, 12.5)],
                st,
            ));
        }
        Icon::Edit => {
            // three sliders: "adjust and style"
            for (i, k) in [6.0f32, 12.5, 9.0].iter().enumerate() {
                let y = 5.5 + i as f32 * 4.5;
                p.line_segment(
                    [at(3.5, y), at(16.5, y)],
                    Stroke::new(1.4 * s, color.gamma_multiply(0.6)),
                );
                p.circle_filled(at(*k, y), 1.9 * s, color);
            }
        }
        Icon::Pin => {
            p.add(Shape::closed_line(
                vec![
                    at(7.0, 3.5),
                    at(13.0, 3.5),
                    at(12.0, 9.0),
                    at(14.5, 11.5),
                    at(5.5, 11.5),
                    at(8.0, 9.0),
                ],
                st,
            ));
            p.line_segment([at(10.0, 11.5), at(10.0, 17.0)], st);
        }
        Icon::Close => {
            p.line_segment([at(5.5, 5.5), at(14.5, 14.5)], st);
            p.line_segment([at(14.5, 5.5), at(5.5, 14.5)], st);
        }
        Icon::Full => {
            for (a, b, d) in [
                ((4.0, 8.0), (4.0, 4.0), (8.0, 4.0)),
                ((12.0, 4.0), (16.0, 4.0), (16.0, 8.0)),
                ((16.0, 12.0), (16.0, 16.0), (12.0, 16.0)),
                ((8.0, 16.0), (4.0, 16.0), (4.0, 12.0)),
            ] {
                p.add(Shape::line(
                    vec![at(a.0, a.1), at(b.0, b.1), at(d.0, d.1)],
                    st,
                ));
            }
        }
        Icon::Timer => {
            p.circle_stroke(at(10.0, 11.0), 6.0 * s, st);
            p.line_segment([at(10.0, 11.0), at(10.0, 7.5)], st);
            p.line_segment([at(8.0, 3.0), at(12.0, 3.0)], st);
        }
        Icon::Open => {
            p.rect_stroke(
                Rect::from_min_max(at(3.5, 4.5), at(16.5, 15.5)),
                2.0 * s,
                st,
                egui::StrokeKind::Middle,
            );
            p.add(Shape::line(
                vec![
                    at(3.5, 13.0),
                    at(8.0, 9.0),
                    at(11.0, 12.0),
                    at(13.0, 10.5),
                    at(16.5, 13.5),
                ],
                st,
            ));
            p.circle_filled(at(12.8, 7.6), 1.3 * s, color);
        }
        Icon::Folder => {
            p.add(Shape::closed_line(
                vec![
                    at(3.5, 5.0),
                    at(8.0, 5.0),
                    at(9.5, 6.8),
                    at(16.5, 6.8),
                    at(16.5, 15.0),
                    at(3.5, 15.0),
                ],
                st,
            ));
        }
    }
}

/// App icon: a rounded tile with viewfinder corners.
pub fn app_icon() -> egui::IconData {
    let n = 64u32;
    let mut px = vec![0u8; (n * n * 4) as usize];
    let acc = [255u8, 90, 54];
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let r = 14.0;
            let dx = (fx - fx.clamp(r, n as f32 - r)).abs();
            let dy = (fy - fy.clamp(r, n as f32 - r)).abs();
            let inside = (dx * dx + dy * dy).sqrt() <= r;
            let i = ((y * n + x) * 4) as usize;
            if inside {
                let t = fy / n as f32;
                px[i] = (acc[0] as f32 * (1.0 - t * 0.15)) as u8;
                px[i + 1] = (acc[1] as f32 + t * 40.0) as u8;
                px[i + 2] = acc[2];
                px[i + 3] = 255;
                let corner = |cx: f32, cy: f32| {
                    let ax = (fx - cx).abs();
                    let ay = (fy - cy).abs();
                    (ax < 12.0 && ay < 3.0) || (ay < 12.0 && ax < 3.0)
                };
                let near = [(18.0, 18.0), (46.0, 18.0), (18.0, 46.0), (46.0, 46.0)]
                    .iter()
                    .any(|(cx, cy)| {
                        let inner = (fx - 32.0).abs() < 11.0 || (fy - 32.0).abs() < 11.0;
                        corner(*cx, *cy) && !inner
                    });
                if near {
                    px[i] = 255;
                    px[i + 1] = 255;
                    px[i + 2] = 255;
                }
            }
        }
    }
    egui::IconData {
        rgba: px,
        width: n,
        height: n,
    }
}
