//! CPU renderer: bakes annotations into the screenshot and builds the styled final picture.
//! The same code makes the on-screen preview (at a smaller scale) and the saved file (at 1:1),
//! so what you see is what you get.

use crate::engine::model::*;
use crate::engine::presets;
use crate::system::fonts::{self, FontLib};
use crate::system::settings::{Background, Style, Watermark, WmPos};
use ab_glyph::{Font, FontArc, ScaleFont, point};
use image::RgbaImage;
use tiny_skia as sk;

// ---------------------------------------------------------------- conversions

pub fn to_pixmap(img: &RgbaImage) -> sk::Pixmap {
    let mut pm = sk::Pixmap::new(img.width().max(1), img.height().max(1)).expect("pixmap");
    for (d, s) in pm.pixels_mut().iter_mut().zip(img.pixels()) {
        *d = sk::ColorU8::from_rgba(s[0], s[1], s[2], s[3]).premultiply();
    }
    pm
}

pub fn from_pixmap(pm: &sk::Pixmap) -> RgbaImage {
    let mut out = RgbaImage::new(pm.width(), pm.height());
    for (d, s) in out.pixels_mut().zip(pm.pixels()) {
        let c = s.demultiply();
        *d = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

fn paint_rgba(c: [u8; 4]) -> sk::Paint<'static> {
    let mut p = sk::Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    p
}

fn shader_paint(shader: sk::Shader<'_>) -> sk::Paint<'_> {
    sk::Paint {
        shader,
        anti_alias: true,
        ..Default::default()
    }
}

fn pt(p: egui::Pos2) -> (f32, f32) {
    (p.x, p.y)
}

fn polyline(points: &[egui::Pos2]) -> Option<sk::Path> {
    let mut pb = sk::PathBuilder::new();
    let (x, y) = pt(*points.first()?);
    pb.move_to(x, y);
    if points.len() == 1 {
        pb.line_to(x + 0.01, y);
    }
    for p in &points[1..] {
        pb.line_to(p.x, p.y);
    }
    pb.finish()
}

/// Rounded rectangle with a radius per corner (top-left, top-right, bottom-right, bottom-left).
pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> Option<sk::Path> {
    const K: f32 = 0.552_284_8;
    let lim = (w.min(h) / 2.0).max(0.0);
    let [tl, tr, br, bl] = r.map(|v| v.clamp(0.0, lim));
    let mut pb = sk::PathBuilder::new();
    pb.move_to(x + tl, y);
    pb.line_to(x + w - tr, y);
    pb.cubic_to(
        x + w - tr + tr * K,
        y,
        x + w,
        y + tr - tr * K,
        x + w,
        y + tr,
    );
    pb.line_to(x + w, y + h - br);
    pb.cubic_to(
        x + w,
        y + h - br + br * K,
        x + w - br + br * K,
        y + h,
        x + w - br,
        y + h,
    );
    pb.line_to(x + bl, y + h);
    pb.cubic_to(
        x + bl - bl * K,
        y + h,
        x,
        y + h - bl + bl * K,
        x,
        y + h - bl,
    );
    pb.line_to(x, y + tl);
    pb.cubic_to(x, y + tl - tl * K, x + tl - tl * K, y, x + tl, y);
    pb.close();
    pb.finish()
}

// ---------------------------------------------------------------- pixel effects

/// Three box-blur passes ≈ gaussian, on premultiplied RGBA bytes of a sub-rectangle.
fn blur_region(
    data: &mut [u8],
    stride_w: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    radius: usize,
) {
    if w < 2 || h < 2 || radius == 0 {
        return;
    }
    let mut buf: Vec<[u32; 4]> = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let i = ((y0 + y) * stride_w + x0 + x) * 4;
            buf.push([
                data[i] as u32,
                data[i + 1] as u32,
                data[i + 2] as u32,
                data[i + 3] as u32,
            ]);
        }
    }
    let mut tmp = buf.clone();
    for _ in 0..3 {
        box_pass(&buf, &mut tmp, w, h, radius, true);
        box_pass(&tmp, &mut buf, w, h, radius, false);
    }
    for y in 0..h {
        for x in 0..w {
            let i = ((y0 + y) * stride_w + x0 + x) * 4;
            let p = buf[y * w + x];
            data[i] = p[0] as u8;
            data[i + 1] = p[1] as u8;
            data[i + 2] = p[2] as u8;
            data[i + 3] = p[3] as u8;
        }
    }
}

fn box_pass(
    src: &[[u32; 4]],
    dst: &mut [[u32; 4]],
    w: usize,
    h: usize,
    r: usize,
    horizontal: bool,
) {
    let (len, lines) = if horizontal { (w, h) } else { (h, w) };
    let idx = |line: usize, k: usize| {
        if horizontal {
            line * w + k
        } else {
            k * w + line
        }
    };
    let div = (2 * r + 1) as u32;
    for line in 0..lines {
        let mut acc = [0u32; 4];
        for k in 0..=(2 * r) {
            let s = src[idx(line, k.saturating_sub(r).min(len - 1))];
            for c in 0..4 {
                acc[c] += s[c];
            }
        }
        for k in 0..len {
            let o = &mut dst[idx(line, k)];
            for c in 0..4 {
                o[c] = acc[c] / div;
            }
            let out_i = src[idx(line, k.saturating_sub(r))];
            let in_i = src[idx(line, (k + r + 1).min(len - 1))];
            for c in 0..4 {
                acc[c] = acc[c] + in_i[c] - out_i[c];
            }
        }
    }
}

fn pixelate_region(
    data: &mut [u8],
    stride_w: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    block: usize,
) {
    let block = block.max(2);
    let mut by = 0;
    while by < h {
        let bh = block.min(h - by);
        let mut bx = 0;
        while bx < w {
            let bw = block.min(w - bx);
            let mut acc = [0u32; 4];
            for y in 0..bh {
                for x in 0..bw {
                    let i = ((y0 + by + y) * stride_w + x0 + bx + x) * 4;
                    for c in 0..4 {
                        acc[c] += data[i + c] as u32;
                    }
                }
            }
            let n = (bw * bh) as u32;
            for y in 0..bh {
                for x in 0..bw {
                    let i = ((y0 + by + y) * stride_w + x0 + bx + x) * 4;
                    for c in 0..4 {
                        data[i + c] = (acc[c] / n) as u8;
                    }
                }
            }
            bx += block;
        }
        by += block;
    }
}

fn clamp_region(
    pm: &sk::Pixmap,
    a: egui::Pos2,
    b: egui::Pos2,
) -> Option<(usize, usize, usize, usize)> {
    let r = norm_rect(a, b);
    let x0 = r.min.x.floor().max(0.0) as usize;
    let y0 = r.min.y.floor().max(0.0) as usize;
    let x1 = (r.max.x.ceil() as i64).clamp(0, pm.width() as i64) as usize;
    let y1 = (r.max.y.ceil() as i64).clamp(0, pm.height() as i64) as usize;
    (x1 > x0 + 1 && y1 > y0 + 1).then_some((x0, y0, x1 - x0, y1 - y0))
}

// ---------------------------------------------------------------- text

/// Draw text with its top-left corner at (x, y). `opacity` multiplies the colour alpha.
/// `at` is the top-left corner of the text.
pub fn draw_text(
    pm: &mut sk::Pixmap,
    font: &FontArc,
    size: f32,
    at: (f32, f32),
    color: [u8; 4],
    text: &str,
    opacity: f32,
) {
    let (x, y) = at;
    let sf = font.as_scaled(fonts::px_scale(font, size));
    let line_h = sf.height() + sf.line_gap();
    let (pw, ph) = (pm.width() as i32, pm.height() as i32);
    let alpha = color[3] as f32 / 255.0 * opacity.clamp(0.0, 1.0);
    let pixels = pm.pixels_mut();
    for (li, line) in text.split('\n').enumerate() {
        let baseline = y + sf.ascent() + li as f32 * line_h;
        let mut cx = x;
        let mut prev = None;
        for ch in line.chars() {
            let id = sf.glyph_id(ch);
            if let Some(p) = prev {
                cx += sf.kern(p, id);
            }
            let g = id.with_scale_and_position(sf.scale(), point(cx, baseline));
            cx += sf.h_advance(id);
            prev = Some(id);
            let Some(og) = font.outline_glyph(g) else {
                continue;
            };
            let bb = og.px_bounds();
            og.draw(|gx, gy, cov| {
                let px = bb.min.x as i32 + gx as i32;
                let py = bb.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || px >= pw || py >= ph {
                    return;
                }
                let a = (cov * alpha).clamp(0.0, 1.0);
                if a <= 0.0 {
                    return;
                }
                let dst = &mut pixels[(py * pw + px) as usize];
                let inv = 1.0 - a;
                let r = color[0] as f32 * a + dst.red() as f32 * inv;
                let gg = color[1] as f32 * a + dst.green() as f32 * inv;
                let b = color[2] as f32 * a + dst.blue() as f32 * inv;
                let al = 255.0 * a + dst.alpha() as f32 * inv;
                if let Some(c) = sk::PremultipliedColorU8::from_rgba(
                    r.min(al) as u8,
                    gg.min(al) as u8,
                    b.min(al) as u8,
                    al as u8,
                ) {
                    *dst = c;
                }
            });
        }
    }
}

pub fn text_box_padding(size: f32) -> egui::Vec2 {
    egui::vec2(size * 0.4, size * 0.22)
}

// ---------------------------------------------------------------- annotations

/// The outline of `r` with corners of `radius` (clamped to half the shorter side), as cubic arcs.
fn rounded_rect_path(r: egui::Rect, radius: f32) -> Option<sk::Path> {
    let rad = radius.clamp(0.0, r.width().min(r.height()) / 2.0);
    if rad <= 0.0 {
        let rect = sk::Rect::from_ltrb(r.min.x, r.min.y, r.max.x, r.max.y)?;
        return Some(sk::PathBuilder::from_rect(rect));
    }
    // A quarter circle as one cubic: control points 0.5523 of the radius along the tangents.
    let c = rad * 0.552_284_8;
    let (l, t, rt, b) = (r.min.x, r.min.y, r.max.x, r.max.y);
    let mut pb = sk::PathBuilder::new();
    pb.move_to(l + rad, t);
    pb.line_to(rt - rad, t);
    pb.cubic_to(rt - rad + c, t, rt, t + rad - c, rt, t + rad);
    pb.line_to(rt, b - rad);
    pb.cubic_to(rt, b - rad + c, rt - rad + c, b, rt - rad, b);
    pb.line_to(l + rad, b);
    pb.cubic_to(l + rad - c, b, l, b - rad + c, l, b - rad);
    pb.line_to(l, t + rad);
    pb.cubic_to(l, t + rad - c, l + rad - c, t, l + rad, t);
    pb.close();
    pb.finish()
}

pub fn render_annotations(base: &RgbaImage, anns: &[Ann], fonts: &FontLib) -> RgbaImage {
    if anns.is_empty() {
        return base.clone();
    }
    let mut pm = to_pixmap(base);
    for a in anns {
        draw_ann(&mut pm, a, fonts);
    }
    from_pixmap(&pm)
}

fn stroke(width: f32, cap: sk::LineCap, join: sk::LineJoin) -> sk::Stroke {
    sk::Stroke {
        width,
        line_cap: cap,
        line_join: join,
        ..Default::default()
    }
}

pub fn draw_ann(pm: &mut sk::Pixmap, a: &Ann, fonts: &FontLib) {
    let id = sk::Transform::identity();
    let round = stroke(a.width, sk::LineCap::Round, sk::LineJoin::Round);
    let paint = paint_rgba(a.color);
    match &a.shape {
        Shape::Pen(points) => {
            if let Some(p) = polyline(points) {
                pm.stroke_path(&p, &paint, &round, id, None);
            }
        }
        Shape::Marker(points) => {
            let mut c = a.color;
            c[3] = MARKER_ALPHA;
            let mut paint = paint_rgba(c);
            paint.blend_mode = sk::BlendMode::Multiply;
            if crate::engine::model::luminance(a.color) < 0.35 {
                paint.blend_mode = sk::BlendMode::SourceOver;
            }
            let s = stroke(
                a.width * MARKER_WIDTH_FACTOR,
                sk::LineCap::Butt,
                sk::LineJoin::Round,
            );
            if let Some(p) = polyline(points) {
                pm.stroke_path(&p, &paint, &s, id, None);
            }
        }
        Shape::Line(p0, p1) => {
            if let Some(p) = polyline(&[*p0, *p1]) {
                pm.stroke_path(&p, &paint, &round, id, None);
            }
        }
        Shape::Arrow(p0, p1) => {
            let (shaft_end, head) = arrow_geom(*p0, *p1, a.width);
            if let Some(p) = polyline(&[*p0, shaft_end]) {
                pm.stroke_path(&p, &paint, &round, id, None);
            }
            let mut pb = sk::PathBuilder::new();
            pb.move_to(head[0].x, head[0].y);
            pb.line_to(head[1].x, head[1].y);
            pb.line_to(head[2].x, head[2].y);
            pb.close();
            if let Some(p) = pb.finish() {
                pm.fill_path(&p, &paint, sk::FillRule::Winding, id, None);
                pm.stroke_path(
                    &p,
                    &paint,
                    &stroke(a.width.min(3.0), sk::LineCap::Round, sk::LineJoin::Round),
                    id,
                    None,
                );
            }
        }
        Shape::Rect(p0, p1, radius) => {
            let r = norm_rect(*p0, *p1);
            if let Some(path) = rounded_rect_path(r, *radius) {
                if a.filled {
                    pm.fill_path(&path, &paint, sk::FillRule::Winding, id, None);
                } else {
                    pm.stroke_path(
                        &path,
                        &paint,
                        &stroke(a.width, sk::LineCap::Square, sk::LineJoin::Miter),
                        id,
                        None,
                    );
                }
            }
        }
        Shape::Ellipse(p0, p1) => {
            let r = norm_rect(*p0, *p1);
            if let Some(rect) = sk::Rect::from_ltrb(r.min.x, r.min.y, r.max.x, r.max.y)
                && let Some(path) = sk::PathBuilder::from_oval(rect)
            {
                if a.filled {
                    pm.fill_path(&path, &paint, sk::FillRule::Winding, id, None);
                } else {
                    pm.stroke_path(&path, &paint, &round, id, None);
                }
            }
        }
        Shape::Blur(p0, p1, mode) => {
            if let Some((x, y, w, h)) = clamp_region(pm, *p0, *p1) {
                let stride = pm.width() as usize;
                let data = pm.data_mut();
                match mode {
                    BlurMode::Blur => {
                        blur_region(data, stride, x, y, w, h, (4.0 + a.width * 2.0) as usize)
                    }
                    BlurMode::Pixelate => {
                        pixelate_region(data, stride, x, y, w, h, (6.0 + a.width * 2.0) as usize)
                    }
                }
            }
        }
        Shape::Text {
            pos,
            text,
            font,
            size,
            boxed,
        } => {
            let f = &fonts.get(*font).font;
            let color = if *boxed {
                let (w, h) = fonts::measure(f, *size, text);
                let pad = text_box_padding(*size);
                if let Some(path) = rounded_rect(
                    pos.x - pad.x,
                    pos.y - pad.y,
                    w + pad.x * 2.0,
                    h + pad.y * 2.0,
                    [size * 0.25; 4],
                ) {
                    pm.fill_path(&path, &paint, sk::FillRule::Winding, id, None);
                }
                if luminance(a.color) > 0.6 {
                    [17, 17, 17, 255]
                } else {
                    [255, 255, 255, 255]
                }
            } else {
                a.color
            };
            draw_text(pm, f, *size, (pos.x, pos.y), color, text, 1.0);
        }
        Shape::Counter { center, n } => {
            let r = counter_radius(a.width);
            if let Some(path) = sk::PathBuilder::from_circle(center.x, center.y, r) {
                pm.fill_path(&path, &paint, sk::FillRule::Winding, id, None);
                let ring = paint_rgba([255, 255, 255, 230]);
                pm.stroke_path(
                    &path,
                    &ring,
                    &stroke((r * 0.12).max(1.5), sk::LineCap::Round, sk::LineJoin::Round),
                    id,
                    None,
                );
            }
            let f = &fonts.get(fonts.bold).font;
            let label = n.to_string();
            let size = fonts::em_for_height(f, r * 1.25);
            let (w, h) = fonts::measure(f, size, &label);
            let tc = if luminance(a.color) > 0.7 {
                [17, 17, 17, 255]
            } else {
                [255, 255, 255, 255]
            };
            draw_text(
                pm,
                f,
                size,
                (center.x - w / 2.0, center.y - h / 2.0),
                tc,
                &label,
                1.0,
            );
        }
    }
}

// ---------------------------------------------------------------- composition

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub w: f32,
    pub h: f32,
    /// The screenshot inside the canvas.
    pub img: [f32; 4],
    /// The card (screenshot plus optional window bar).
    pub card: [f32; 4],
    pub bar_h: f32,
    pub band: Option<[f32; 4]>,
}

pub fn layout(iw: f32, ih: f32, style: &Style, wm: &Watermark) -> Layout {
    let m = iw.max(ih);
    let pad = style.padding.max(0.0) / 100.0 * m;
    let bar_h = if style.window_bar {
        (m * 0.03).max(22.0)
    } else {
        0.0
    };
    let (cw, ch) = (iw, ih + bar_h);
    let mut w = cw + pad * 2.0;
    let mut h = ch + pad * 2.0;
    if let Some(r) = style.aspect.ratio() {
        if w / h < r {
            w = h * r;
        } else {
            h = w / r;
        }
    }
    let band_h = if wm.enabled && wm.position.is_band() {
        (w * 0.055).max(30.0)
    } else {
        0.0
    };
    let top = if wm.enabled && wm.position == WmPos::BandTop {
        band_h
    } else {
        0.0
    };
    let cx = ((w - cw) / 2.0).round();
    let cy = ((h - ch) / 2.0 + top).round();
    let band = (band_h > 0.0).then(|| {
        if wm.position == WmPos::BandTop {
            [0.0, 0.0, w, band_h]
        } else {
            [0.0, h, w, band_h]
        }
    });
    Layout {
        w: w.round(),
        h: (h + band_h).round(),
        img: [cx, cy + bar_h, iw, ih],
        card: [cx, cy, cw, ch],
        bar_h,
        band,
    }
}

fn gradient_shader(
    stops: &[(f32, [u8; 3])],
    angle_deg: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Option<sk::Shader<'static>> {
    let a = angle_deg.to_radians();
    // CSS-like angle: 0° points up, 90° points right.
    let dir = (a.sin(), -a.cos());
    let half = (w * dir.0.abs() + h * dir.1.abs()) / 2.0;
    let c = (x + w / 2.0, y + h / 2.0);
    let start = sk::Point::from_xy(c.0 - dir.0 * half, c.1 - dir.1 * half);
    let end = sk::Point::from_xy(c.0 + dir.0 * half, c.1 + dir.1 * half);
    let st = stops
        .iter()
        .map(|(p, c)| sk::GradientStop::new(*p, sk::Color::from_rgba8(c[0], c[1], c[2], 255)))
        .collect();
    sk::LinearGradient::new(
        start,
        end,
        st,
        sk::SpreadMode::Pad,
        sk::Transform::identity(),
    )
}

pub type Stops = Vec<(f32, [u8; 3])>;

pub fn background_stops(style: &Style) -> Option<(Stops, f32)> {
    match style.background {
        Background::Preset(i) => {
            let p = &presets::PRESETS[i.min(presets::PRESETS.len() - 1)];
            Some((p.stops.to_vec(), p.angle))
        }
        Background::Custom => Some((
            vec![
                (
                    0.0,
                    [style.custom_a[0], style.custom_a[1], style.custom_a[2]],
                ),
                (
                    1.0,
                    [style.custom_b[0], style.custom_b[1], style.custom_b[2]],
                ),
            ],
            style.custom_angle,
        )),
        _ => None,
    }
}

/// Build the final picture. `src` is the annotated screenshot already scaled by `unit`
/// (1.0 for the saved file, smaller for previews); absolute sizes such as the radius scale with it.
pub fn compose(
    src: &RgbaImage,
    style: &Style,
    wm: &Watermark,
    logo: Option<&RgbaImage>,
    fonts: &FontLib,
    unit: f32,
) -> (RgbaImage, Layout) {
    let (iw, ih) = (src.width() as f32, src.height() as f32);
    let unit_style = Style {
        radius: style.radius * unit,
        ..style.clone()
    };
    let mut lay = layout(iw / unit, ih / unit, style, wm);
    // Scale the 1:1 layout down to this unit so previews match the export exactly.
    let s = unit;
    lay.w = (lay.w * s).round().max(1.0);
    lay.h = (lay.h * s).round().max(1.0);
    for v in [&mut lay.img, &mut lay.card] {
        v[0] = (v[0] * s).round();
        v[1] = (v[1] * s).round();
    }
    lay.img[2] = iw;
    lay.img[3] = ih;
    lay.bar_h *= s;
    lay.card[1] = lay.img[1] - lay.bar_h;
    lay.card[2] = iw;
    lay.card[3] = ih + lay.bar_h;
    if let Some(b) = &mut lay.band {
        for v in b.iter_mut() {
            *v = (*v * s).round();
        }
        b[2] = lay.w;
        if wm.position == WmPos::BandBottom {
            b[3] = lay.h - b[1];
        }
    }
    let style = &unit_style;

    let mut pm = sk::Pixmap::new(lay.w as u32, lay.h as u32).expect("canvas");
    let id = sk::Transform::identity();
    let full = sk::Rect::from_xywh(0.0, 0.0, lay.w, lay.h).unwrap();

    // Background
    match style.background {
        Background::None => {}
        Background::Solid => {
            pm.fill_rect(full, &paint_rgba(style.solid), id, None);
        }
        Background::Blurred => {
            let small_w = 48u32;
            let small_h = ((ih / iw) * small_w as f32).max(8.0) as u32;
            let small = image::imageops::resize(
                src,
                small_w,
                small_h,
                image::imageops::FilterType::Triangle,
            );
            let mut spm = to_pixmap(&small);
            let (w, h) = (spm.width() as usize, spm.height() as usize);
            blur_region(spm.data_mut(), w, 0, 0, w, h, 3);
            // cover the canvas
            let k = (lay.w / w as f32).max(lay.h / h as f32);
            let ox = (lay.w - w as f32 * k) / 2.0;
            let oy = (lay.h - h as f32 * k) / 2.0;
            let p = shader_paint(sk::Pattern::new(
                spm.as_ref(),
                sk::SpreadMode::Pad,
                sk::FilterQuality::Bicubic,
                1.0,
                sk::Transform::from_row(k, 0.0, 0.0, k, ox, oy),
            ));
            pm.fill_rect(full, &p, id, None);
            pm.fill_rect(full, &paint_rgba([0, 0, 0, 40]), id, None);
        }
        _ => {
            if let Some((stops, angle)) = background_stops(style)
                && let Some(sh) = gradient_shader(&stops, angle, 0.0, 0.0, lay.w, lay.h)
            {
                pm.fill_rect(full, &shader_paint(sh), id, None);
            }
        }
    }

    let [cx, cy, cw, ch] = lay.card;
    let r = style.radius.max(0.0);
    let has_room = lay.w > cw + 1.0 || lay.h > ch + 1.0;

    // Shadow: a blurred, downscaled silhouette of the card.
    if style.shadow > 0.01 && has_room {
        let f = 4.0;
        let sw = (lay.w / f).ceil() as u32 + 1;
        let sh = (lay.h / f).ceil() as u32 + 1;
        if let Some(mut mask) = sk::Pixmap::new(sw, sh) {
            let m = cw.max(ch);
            let offset = m * 0.012;
            if let Some(path) = rounded_rect(cx / f, (cy + offset) / f, cw / f, ch / f, [r / f; 4])
            {
                mask.fill_path(
                    &path,
                    &paint_rgba([0, 0, 0, 255]),
                    sk::FillRule::Winding,
                    id,
                    None,
                );
            }
            let rad = ((m * 0.022 / f) * (0.6 + style.shadow)).max(2.0) as usize;
            blur_region(
                mask.data_mut(),
                sw as usize,
                0,
                0,
                sw as usize,
                sh as usize,
                rad,
            );
            let pp = sk::PixmapPaint {
                opacity: (style.shadow * 0.9).min(1.0),
                quality: sk::FilterQuality::Bilinear,
                ..Default::default()
            };
            pm.draw_pixmap(
                0,
                0,
                mask.as_ref(),
                &pp,
                sk::Transform::from_scale(f, f),
                None,
            );
        }
    }

    // Window bar
    if lay.bar_h > 0.0 {
        let dark = background_is_light(style);
        let bar_col = if dark {
            [30, 30, 34, 255]
        } else {
            [246, 246, 248, 255]
        };
        if let Some(path) = rounded_rect(cx, cy, cw, ch, [r, r, r, r]) {
            pm.fill_path(&path, &paint_rgba(bar_col), sk::FillRule::Winding, id, None);
        }
        let dot_r = lay.bar_h * 0.15;
        let mut x = cx + lay.bar_h * 0.55;
        for c in [[255, 95, 87, 255], [254, 188, 46, 255], [40, 200, 64, 255]] {
            if let Some(p) = sk::PathBuilder::from_circle(x, cy + lay.bar_h / 2.0, dot_r) {
                pm.fill_path(&p, &paint_rgba(c), sk::FillRule::Winding, id, None);
            }
            x += dot_r * 3.2;
        }
    }

    // Screenshot with rounded corners
    let src_pm = to_pixmap(src);
    let [ix, iy, _, _] = lay.img;
    let top_r = if lay.bar_h > 0.0 { 0.0 } else { r };
    if let Some(path) = rounded_rect(ix, iy, iw, ih, [top_r, top_r, r, r]) {
        let p = shader_paint(sk::Pattern::new(
            src_pm.as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            1.0,
            sk::Transform::from_translate(ix, iy),
        ));
        pm.fill_path(&path, &p, sk::FillRule::Winding, id, None);
    }
    // Hairline so light screenshots don't melt into light backgrounds.
    if has_room
        && !matches!(style.background, Background::None)
        && let Some(path) = rounded_rect(cx + 0.5, cy + 0.5, cw - 1.0, ch - 1.0, [r; 4])
    {
        pm.stroke_path(
            &path,
            &paint_rgba([0, 0, 0, 28]),
            &stroke(1.0, sk::LineCap::Round, sk::LineJoin::Round),
            id,
            None,
        );
    }

    // Watermark
    if wm.enabled {
        draw_watermark(&mut pm, &lay, wm, logo, fonts);
    }

    (from_pixmap(&pm), lay)
}

fn background_is_light(style: &Style) -> bool {
    match style.background {
        Background::Solid => luminance(style.solid) > 0.6,
        _ => background_stops(style)
            .map(|(s, _)| {
                let c = presets::sample(&s, 0.5);
                luminance([c[0], c[1], c[2], 255]) > 0.6
            })
            .unwrap_or(false),
    }
}

fn draw_watermark(
    pm: &mut sk::Pixmap,
    lay: &Layout,
    wm: &Watermark,
    logo: Option<&RgbaImage>,
    fonts: &FontLib,
) {
    let id = sk::Transform::identity();
    let font = &fonts.get(fonts.bold).font;
    let text = wm.text.trim();
    let opacity = wm.opacity.clamp(0.0, 1.0);

    if let Some([bx, by, bw, bh]) = lay.band {
        let band = sk::Rect::from_xywh(bx, by, bw, bh).unwrap();
        pm.fill_rect(band, &paint_rgba(wm.band_color), id, None);
        let pad = bh * 0.3;
        let mut x = bx + pad;
        if let Some(l) = logo {
            let lh = bh * 0.56;
            let k = lh / l.height() as f32;
            let lpm = to_pixmap(l);
            let pp = sk::PixmapPaint {
                opacity,
                quality: sk::FilterQuality::Bicubic,
                ..Default::default()
            };
            pm.draw_pixmap(
                0,
                0,
                lpm.as_ref(),
                &pp,
                sk::Transform::from_row(k, 0.0, 0.0, k, x, by + (bh - lh) / 2.0),
                None,
            );
            x += l.width() as f32 * k + pad * 0.7;
        }
        if !text.is_empty() {
            let size = fonts::em_for_height(font, bh * 0.36);
            let (_, th) = fonts::measure(font, size, text);
            let tc = if luminance(wm.band_color) > 0.6 {
                [20, 20, 20, 255]
            } else {
                [245, 245, 245, 255]
            };
            draw_text(pm, font, size, (x, by + (bh - th) / 2.0), tc, text, opacity);
        }
        return;
    }

    // Area the mark is placed in: the whole canvas, or only the screenshot.
    let area = if wm.inside_image {
        lay.img
    } else {
        [0.0, 0.0, lay.w, lay.h]
    };
    let margin = wm.margin / 100.0 * area[2].min(area[3]);
    let mut lw = 0.0;
    let mut lh = 0.0;
    let mut k = 1.0;
    if let Some(l) = logo {
        lw = wm.size / 100.0 * area[2];
        k = lw / l.width() as f32;
        lh = l.height() as f32 * k;
        if lh > area[3] * 0.4 {
            k *= area[3] * 0.4 / lh;
            lw = l.width() as f32 * k;
            lh = l.height() as f32 * k;
        }
    }
    let height = if logo.is_some() {
        (lh * 0.42).max(10.0)
    } else {
        (wm.size / 100.0 * area[2] * 0.24).max(10.0)
    };
    let size = fonts::em_for_height(font, height);
    let (tw, th) = if text.is_empty() {
        (0.0, 0.0)
    } else {
        fonts::measure(font, size, text)
    };
    let gap = if lw > 0.0 && tw > 0.0 {
        height * 0.5
    } else {
        0.0
    };
    let gw = lw + gap + tw;
    let gh = lh.max(th);
    let (ax, ay) = wm.position.anchor();
    let x = area[0] + margin + (area[2] - margin * 2.0 - gw) * ax;
    let y = area[1] + margin + (area[3] - margin * 2.0 - gh) * ay;
    if let Some(l) = logo {
        let lpm = to_pixmap(l);
        let pp = sk::PixmapPaint {
            opacity,
            quality: sk::FilterQuality::Bicubic,
            ..Default::default()
        };
        pm.draw_pixmap(
            0,
            0,
            lpm.as_ref(),
            &pp,
            sk::Transform::from_row(k, 0.0, 0.0, k, x, y + (gh - lh) / 2.0),
            None,
        );
    }
    if tw > 0.0 {
        let tx = x + lw + gap;
        let ty = y + (gh - th) / 2.0;
        // soft shadow keeps the text readable on any background
        draw_text(
            pm,
            font,
            size,
            (tx + height * 0.05, ty + height * 0.07),
            [0, 0, 0, 90],
            text,
            opacity,
        );
        draw_text(pm, font, size, (tx, ty), wm.text_color, text, opacity);
    }
}

/// Only the rounded corners, no background.
pub fn corners_only(src: &RgbaImage, radius: f32) -> RgbaImage {
    let (w, h) = (src.width() as f32, src.height() as f32);
    let mut pm = sk::Pixmap::new(src.width(), src.height()).expect("pixmap");
    let s = to_pixmap(src);
    if let Some(path) = rounded_rect(0.0, 0.0, w, h, [radius; 4]) {
        let p = shader_paint(sk::Pattern::new(
            s.as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            1.0,
            sk::Transform::identity(),
        ));
        pm.fill_path(
            &path,
            &p,
            sk::FillRule::Winding,
            sk::Transform::identity(),
            None,
        );
    }
    from_pixmap(&pm)
}

/// Final picture for a crop, following the quick-action setting.
pub fn finish(
    src: &RgbaImage,
    settings: &crate::system::settings::Settings,
    logo: Option<&RgbaImage>,
    fonts: &FontLib,
    quick: crate::system::settings::QuickStyle,
) -> RgbaImage {
    use crate::system::settings::QuickStyle;
    match quick {
        QuickStyle::Plain => src.clone(),
        QuickStyle::CornersOnly => corners_only(src, settings.style.radius),
        QuickStyle::Full => compose(src, &settings.style, &settings.watermark, logo, fonts, 1.0).0,
    }
}

pub fn load_logo(path: &str) -> Option<RgbaImage> {
    image::open(path).ok().map(|i| i.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::settings::{Aspect, Background, Style, Watermark, WmPos};
    use egui::pos2;

    fn logo() -> RgbaImage {
        let mut pm = sk::Pixmap::new(200, 80).unwrap();
        let mut p = sk::Paint::default();
        p.set_color_rgba8(255, 255, 255, 255);
        p.anti_alias = true;
        if let Some(path) = rounded_rect(0.0, 0.0, 80.0, 80.0, [20.0; 4]) {
            pm.fill_path(
                &path,
                &p,
                sk::FillRule::Winding,
                sk::Transform::identity(),
                None,
            );
        }
        p.set_color_rgba8(255, 255, 255, 200);
        if let Some(path) = rounded_rect(95.0, 25.0, 105.0, 30.0, [15.0; 4]) {
            pm.fill_path(
                &path,
                &p,
                sk::FillRule::Winding,
                sk::Transform::identity(),
                None,
            );
        }
        from_pixmap(&pm)
    }

    #[test]
    fn a_rounded_rect_renders_with_its_radius() {
        let fonts = FontLib::load();
        let base = RgbaImage::from_pixel(200, 200, image::Rgba([255, 255, 255, 255]));
        let rect = |radius: f32| Ann {
            shape: Shape::Rect(pos2(40.0, 40.0), pos2(160.0, 160.0), radius),
            color: [255, 0, 0, 255],
            width: 6.0,
            filled: true,
        };
        let square = render_annotations(&base, &[rect(0.0)], &fonts);
        let round = render_annotations(&base, &[rect(30.0)], &fonts);
        // The box's corner is inside the square and outside the rounded one.
        assert_eq!(square.get_pixel(42, 42).0, [255, 0, 0, 255]);
        assert_eq!(round.get_pixel(42, 42).0, [255, 255, 255, 255]);
        // The middle of a side is in both.
        assert_eq!(round.get_pixel(100, 42).0, [255, 0, 0, 255]);
    }

    #[test]
    fn renders_styles_and_marks() {
        let fonts = FontLib::load();
        assert!(fonts.list.len() >= 2, "at least the two built-in fonts");
        let base = crate::system::capture::sample_window(1000, 620);
        let mut anns = vec![
            Ann {
                shape: Shape::Ellipse(pos2(400.0, 100.0), pos2(700.0, 260.0)),
                color: [255, 0, 0, 255],
                width: 4.0,
                filled: false,
            },
            Ann {
                shape: Shape::Rect(pos2(250.0, 280.0), pos2(500.0, 400.0), 0.0),
                color: [0, 179, 255, 255],
                width: 3.0,
                filled: false,
            },
            Ann {
                shape: Shape::Blur(pos2(700.0, 300.0), pos2(960.0, 420.0), BlurMode::Pixelate),
                color: [0, 0, 0, 255],
                width: 3.0,
                filled: false,
            },
            Ann {
                shape: Shape::Marker(vec![pos2(260.0, 160.0), pos2(380.0, 160.0)]),
                color: [255, 214, 10, 255],
                width: 3.0,
                filled: false,
            },
        ];
        for (i, f) in fonts.list.iter().enumerate().take(6) {
            anns.push(Ann {
                shape: Shape::Text {
                    pos: pos2(
                        260.0 + (i % 3) as f32 * 230.0,
                        450.0 + (i / 3) as f32 * 60.0,
                    ),
                    text: f.name.clone(),
                    font: i,
                    size: 30.0,
                    boxed: i == 0,
                },
                color: [17, 17, 17, 255],
                width: 3.0,
                filled: false,
            });
        }
        anns.push(Ann {
            shape: Shape::Counter {
                center: pos2(720.0, 90.0),
                n: 1,
            },
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: true,
        });
        anns.push(Ann {
            shape: Shape::Arrow(pos2(900.0, 560.0), pos2(720.0, 300.0)),
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: false,
        });
        let annotated = render_annotations(&base, &anns, &fonts);
        let lg = logo();
        let cases = [
            (
                "corner",
                Style::default(),
                Watermark {
                    enabled: true,
                    text: "@takeshot".into(),
                    position: WmPos::BottomRight,
                    ..Default::default()
                },
            ),
            (
                "band",
                Style {
                    background: Background::Preset(1),
                    window_bar: true,
                    aspect: Aspect::Wide,
                    ..Default::default()
                },
                Watermark {
                    enabled: true,
                    text: "takeshot.app".into(),
                    position: WmPos::BandBottom,
                    ..Default::default()
                },
            ),
            (
                "blurred",
                Style {
                    background: Background::Blurred,
                    radius: 28.0,
                    ..Default::default()
                },
                Watermark {
                    enabled: true,
                    position: WmPos::TopLeft,
                    inside_image: true,
                    ..Default::default()
                },
            ),
        ];
        std::fs::create_dir_all("shots").unwrap();
        for (name, style, wm) in cases {
            let (img, lay) = compose(&annotated, &style, &wm, Some(&lg), &fonts, 1.0);
            assert_eq!(img.width() as f32, lay.w);
            // corners of the screenshot are rounded: the canvas pixel just inside the corner shows background
            crate::system::actions::save_png(
                &img,
                std::path::Path::new(&format!("shots/test-{name}.png")),
            )
            .unwrap();
            // preview at half scale has the same proportions
            let half = image::imageops::resize(
                &annotated,
                500,
                310,
                image::imageops::FilterType::Triangle,
            );
            let (small, _) = compose(&half, &style, &wm, Some(&lg), &fonts, 0.5);
            assert!(
                (small.width() as f32 - img.width() as f32 * 0.5).abs() <= 2.0,
                "{name}: preview width"
            );
        }
        // no background: transparent rounded corners
        let st = Style {
            background: Background::None,
            padding: 0.0,
            ..Default::default()
        };
        let (img, _) = compose(&base, &st, &Watermark::default(), None, &fonts, 1.0);
        assert_eq!(img.get_pixel(0, 0).0[3], 0, "corner is transparent");
        assert_eq!(img.get_pixel(500, 300).0[3], 255, "middle is opaque");
    }

    #[test]
    fn shift_makes_uniform_shapes() {
        use crate::engine::model::{Tool, constrain};
        let (a, b) = constrain(
            Tool::Ellipse,
            pos2(0.0, 0.0),
            pos2(100.0, 40.0),
            true,
            false,
        );
        assert_eq!((b - a).x.abs(), (b - a).y.abs());
        let (_, b) = constrain(Tool::Line, pos2(0.0, 0.0), pos2(100.0, 8.0), true, false);
        assert!(b.y.abs() < 0.01, "snaps to horizontal");
        let (a, b) = constrain(Tool::Rect, pos2(50.0, 50.0), pos2(70.0, 60.0), false, true);
        assert_eq!(a, pos2(30.0, 40.0));
        assert_eq!(b, pos2(70.0, 60.0));
    }
}
