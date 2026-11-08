//! Where floating bars go: the capture's bars beside the area and the editor's object bar over
//! the selection, kept on the screen or the canvas.

use egui::{Rect, Vec2, pos2};

/// Space between the object bar and the object.
pub const GAP: f32 = 10.0;

/// Space between the capture area and its main bar.
pub const CAPTURE_GAP: f32 = 12.0;

/// Space between the capture's two bars.
pub const BAR_SPACING: f32 = 6.0;

/// The capture's bars keep this far from the screen's edges.
pub const MARGIN: f32 = 8.0;

/// Where the capture's bars go, as in "Captura (PrtSc) · escuro": right-aligned with the area,
/// under it with the main bar nearest; above it when there is no room below; at the bottom inside
/// the area when neither fits. The options bar (`opts`, when shown) shares the main bar's right
/// edge. Both stay `MARGIN` inside `screen`.
pub fn capture_bars(
    area: Rect,
    main: Vec2,
    opts: Option<Vec2>,
    screen: Rect,
) -> (Rect, Option<Rect>) {
    let inside = screen.shrink(MARGIN);
    let opts_h = opts.map_or(0.0, |o| BAR_SPACING + o.y);
    let widest = opts.map_or(main.x, |o| o.x.max(main.x));
    let right = area.right().min(inside.right()).max(inside.left() + widest);
    let (main_top, opts_top) = if area.bottom() + CAPTURE_GAP + main.y + opts_h <= inside.bottom() {
        let top = area.bottom() + CAPTURE_GAP;
        (top, top + main.y + BAR_SPACING)
    } else if area.top() - CAPTURE_GAP - main.y - opts_h >= inside.top() {
        let top = area.top() - CAPTURE_GAP - main.y;
        (top, top - opts_h)
    } else {
        let top = (area.bottom() - CAPTURE_GAP - main.y)
            .min(inside.bottom() - main.y)
            .max(inside.top() + opts_h);
        (top, top - opts_h)
    };
    let main_r = Rect::from_min_size(pos2(right - main.x, main_top), main);
    let opts_r = opts.map(|o| Rect::from_min_size(pos2(right - o.x, opts_top), o));
    (main_r, opts_r)
}

/// The object bar's rect, `size` big: centred over `object` and `GAP` above it; below it when the
/// canvas has no room above; at the canvas's top when neither fits. Always inside `canvas`, even
/// for an object scrolled off it.
pub fn object_bar(object: Rect, size: Vec2, canvas: Rect) -> Rect {
    let x = (object.center().x - size.x / 2.0)
        .min(canvas.right() - size.x)
        .max(canvas.left());
    let above = object.top() - GAP - size.y;
    let below = object.bottom() + GAP;
    let y = if above >= canvas.top() {
        above
    } else if below + size.y <= canvas.bottom() {
        below
    } else {
        canvas.top()
    };
    let y = y.min(canvas.bottom() - size.y).max(canvas.top());
    Rect::from_min_size(pos2(x, y), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn bars_stay_on_screen_and_clear_of_the_area() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 900.0));
        let inside = screen.shrink(MARGIN);
        let (main, opts) = (vec2(638.0, 42.0), vec2(552.0, 40.0));
        let bars = |area: Rect, with_opts: bool| {
            let (m, o) = capture_bars(area, main, with_opts.then_some(opts), screen);
            assert_eq!(m.size(), main);
            assert!(inside.contains_rect(m), "{m:?} off the screen for {area:?}");
            if let Some(o) = o {
                assert_eq!(o.size(), opts);
                assert!(inside.contains_rect(o), "{o:?} off the screen for {area:?}");
                assert_eq!(o.right(), m.right(), "the two bars share their right edge");
                assert!(!o.intersects(m));
            }
            (m, o)
        };

        // Room below (the design's frame): under the area, right edges aligned, options under.
        let area = Rect::from_min_max(pos2(260.0, 120.0), pos2(1020.0, 550.0));
        let (m, o) = bars(area, true);
        assert_eq!(m.top(), area.bottom() + CAPTURE_GAP);
        assert_eq!(m.right(), area.right());
        assert_eq!(o.unwrap().top(), m.bottom() + BAR_SPACING);

        // An area at the bottom of the screen: above it, the main bar nearest.
        let area = Rect::from_min_max(pos2(260.0, 500.0), pos2(1020.0, 890.0));
        let (m, o) = bars(area, true);
        assert_eq!(m.bottom(), area.top() - CAPTURE_GAP);
        assert_eq!(o.unwrap().bottom(), m.top() - BAR_SPACING);

        // Room for the main bar under the area but not for both: the pair goes above.
        let area = Rect::from_min_max(pos2(260.0, 300.0), pos2(1020.0, 810.0));
        let (m, _) = bars(area, true);
        assert_eq!(m.bottom(), area.top() - CAPTURE_GAP);

        // Touching the right edge: pulled inside.
        let area = Rect::from_min_max(pos2(1000.0, 100.0), pos2(1600.0, 400.0));
        assert_eq!(bars(area, true).0.right(), screen.right() - MARGIN);

        // A small area near the left edge: the wider bar stays on screen.
        let area = Rect::from_min_max(pos2(20.0, 300.0), pos2(60.0, 330.0));
        let (m, _) = bars(area, true);
        assert_eq!(m.left(), screen.left() + MARGIN);

        // Never over the area while there is room outside it.
        for area in [
            Rect::from_min_max(pos2(260.0, 120.0), pos2(1020.0, 550.0)),
            Rect::from_min_max(pos2(260.0, 500.0), pos2(1020.0, 890.0)),
            Rect::from_min_max(pos2(20.0, 300.0), pos2(60.0, 330.0)),
            Rect::from_min_max(pos2(1000.0, 100.0), pos2(1600.0, 400.0)),
        ] {
            let (m, o) = bars(area, true);
            assert!(
                !m.intersects(area) && !o.unwrap().intersects(area),
                "{area:?}"
            );
            let (m, _) = bars(area, false);
            assert!(!m.intersects(area), "{area:?}");
        }

        // An area as big as the screen: the bars still fit on the screen, over it.
        bars(screen, true);
    }

    #[test]
    fn the_object_bar_sits_above_or_below_inside_the_canvas() {
        let canvas = Rect::from_min_size(pos2(0.0, 100.0), vec2(900.0, 600.0));
        let size = vec2(126.0, 36.0);
        let bar = |o: Rect| object_bar(o, size, canvas);

        // Room above: centred over the object, GAP above it.
        let o = Rect::from_min_max(pos2(300.0, 400.0), pos2(500.0, 500.0));
        let b = bar(o);
        assert_eq!(b.size(), size);
        assert_eq!(b.center().x, 400.0);
        assert_eq!(b.bottom(), o.top() - GAP);

        // An object at the top of the canvas: the bar goes below it.
        let o = Rect::from_min_max(pos2(300.0, 110.0), pos2(500.0, 200.0));
        assert_eq!(bar(o).top(), o.bottom() + GAP);

        // Near the right or left edge: pushed against it.
        let o = Rect::from_min_max(pos2(860.0, 400.0), pos2(895.0, 450.0));
        assert_eq!(bar(o).right(), canvas.right());
        let o = Rect::from_min_max(pos2(2.0, 400.0), pos2(30.0, 450.0));
        assert_eq!(bar(o).left(), canvas.left());

        // An object scrolled past the canvas's bottom or top: the bar stays inside.
        for o in [
            Rect::from_min_max(pos2(300.0, 800.0), pos2(500.0, 850.0)),
            Rect::from_min_max(pos2(300.0, -200.0), pos2(500.0, 20.0)),
        ] {
            assert!(canvas.contains_rect(bar(o)), "{o:?}: {:?}", bar(o));
        }

        // An object filling the canvas: still inside it.
        for o in [canvas, canvas.expand(50.0)] {
            assert!(canvas.contains_rect(bar(o)), "{:?}", bar(o));
        }
    }
}
