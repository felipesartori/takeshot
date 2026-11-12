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

/// Where the capture's three parts go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureBars {
    /// The tools, in one column beside the area.
    pub tools: Rect,
    /// The tool's or the selected object's options, beside the tools.
    pub options: Option<Rect>,
    /// Pin, Save, Edit, Copy and close, in one row under (or over) the area.
    pub actions: Rect,
}

/// Places the capture's parts around `area` (the design's capture screens): the tools in a column to the
/// area's right (its left without room), from the area's top and down past its bottom only when
/// the column is taller than the area, in a place that never depends on the options; the options
/// beside the tools, else on the area's other side, from the same top; the
/// actions in a row under the area (over it without room) ending at its right edge, past its left
/// edge only when the row is wider than the area. Nothing leaves `screen` (`MARGIN` inside) or
/// covers another part; the area is covered only when no room is left outside it.
pub fn capture_bars(
    area: Rect,
    tools: Vec2,
    options: Option<Vec2>,
    actions: Vec2,
    screen: Rect,
) -> CaptureBars {
    let inside = screen.shrink(MARGIN);
    let right_room = inside.right() - area.right() - CAPTURE_GAP;
    let left_room = area.left() - CAPTURE_GAP - inside.left();
    #[derive(PartialEq)]
    enum Side {
        Right,
        Left,
    }
    // The tools' side depends on the tools alone, so they stay put while the options come and go:
    // the area's right, else its left, else (no room) over the area at the roomier screen edge.
    let (side, outside) = if right_room >= tools.x {
        (Side::Right, true)
    } else if left_room >= tools.x {
        (Side::Left, true)
    } else if right_room >= left_room {
        (Side::Right, false)
    } else {
        (Side::Left, false)
    };
    let fit_top = |top: f32, h: f32| top.min(inside.bottom() - h).max(inside.top());
    let tools_x = match (&side, outside) {
        (Side::Right, true) => area.right() + CAPTURE_GAP,
        (Side::Left, true) => area.left() - CAPTURE_GAP - tools.x,
        (Side::Right, false) => inside.right() - tools.x,
        (Side::Left, false) => inside.left(),
    };
    let tools_r = Rect::from_min_size(pos2(tools_x, fit_top(area.top(), tools.y)), tools);
    // The options: beside the tools away from the area, else on the area's other side (either
    // may use the screen's margin to keep off the area), else beside the tools over the area.
    let options_r = options.map(|o| {
        let (away, other) = match side {
            Side::Right => (
                screen.right() - tools_r.right() - BAR_SPACING,
                area.left() - CAPTURE_GAP - screen.left(),
            ),
            Side::Left => (
                tools_r.left() - BAR_SPACING - screen.left(),
                screen.right() - area.right() - CAPTURE_GAP,
            ),
        };
        let x = match side {
            Side::Right if outside && away >= o.x => tools_r.right() + BAR_SPACING,
            Side::Left if outside && away >= o.x => tools_r.left() - BAR_SPACING - o.x,
            Side::Right if outside && other >= o.x => area.left() - CAPTURE_GAP - o.x,
            Side::Left if outside && other >= o.x => area.right() + CAPTURE_GAP,
            Side::Right => tools_r.left() - BAR_SPACING - o.x,
            Side::Left => tools_r.right() + BAR_SPACING,
        };
        Rect::from_min_size(pos2(x, fit_top(area.top(), o.y)), o)
    });
    // The actions: under, over, or inside the area's bottom.
    let below = area.bottom() + CAPTURE_GAP;
    let above = area.top() - CAPTURE_GAP - actions.y;
    let top = if below + actions.y <= inside.bottom() {
        below
    } else if above >= inside.top() {
        above
    } else {
        area.bottom() - CAPTURE_GAP - actions.y
    };
    let top = fit_top(top, actions.y);
    let right = area
        .right()
        .min(inside.right())
        .max(inside.left() + actions.x);
    let preferred = Rect::from_min_size(pos2(right - actions.x, top), actions);
    // Clear of the tools and options: else left of them, right of them, over or under them.
    let parts: Vec<Rect> = std::iter::once(tools_r).chain(options_r).collect();
    let column = parts.iter().fold(Rect::NOTHING, |a, r| a.union(*r));
    let free = |r: &Rect| inside.contains_rect(*r) && !parts.iter().any(|p| p.intersects(*r));
    let actions_r = [
        preferred,
        preferred.translate(egui::vec2(
            column.left() - BAR_SPACING - preferred.right(),
            0.0,
        )),
        preferred.translate(egui::vec2(
            column.right() + BAR_SPACING - preferred.left(),
            0.0,
        )),
        Rect::from_min_size(
            pos2(preferred.left(), column.top() - BAR_SPACING - actions.y),
            actions,
        ),
        Rect::from_min_size(
            pos2(preferred.left(), column.bottom() + BAR_SPACING),
            actions,
        ),
    ]
    .into_iter()
    .find(free)
    .unwrap_or(preferred);
    CaptureBars {
        tools: tools_r,
        options: options_r,
        actions: actions_r,
    }
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
    fn capture_bars_stay_on_screen_and_apart() {
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 900.0));
        let inside = screen.shrink(MARGIN);
        let (tools, opts, actions) = (vec2(44.0, 450.0), vec2(220.0, 260.0), vec2(380.0, 42.0));
        let place = |area: Rect, with_opts: bool| {
            let b = capture_bars(area, tools, with_opts.then_some(opts), actions, screen);
            let mut parts = vec![b.tools, b.actions];
            parts.extend(b.options);
            for (i, r) in parts.iter().enumerate() {
                // The options may touch the screen's edge when that keeps them off the area.
                let room = if Some(*r) == b.options {
                    screen
                } else {
                    inside
                };
                assert!(room.contains_rect(*r), "{r:?} off the screen for {area:?}");
                for o in &parts[i + 1..] {
                    assert!(!r.intersects(*o), "{r:?} over {o:?} for {area:?}");
                }
            }
            b
        };

        // Room all around: tools right of the area from its top, options beside them, actions
        // under it ending at its right edge; nothing over the area.
        let area = Rect::from_min_max(pos2(300.0, 100.0), pos2(1000.0, 650.0));
        let b = place(area, true);
        assert_eq!(b.tools.left(), area.right() + CAPTURE_GAP);
        assert_eq!(b.tools.top(), area.top());
        assert!(
            b.tools.bottom() <= area.bottom(),
            "the whole column within the area's height"
        );
        let o = b.options.unwrap();
        assert_eq!(o.left(), b.tools.right() + BAR_SPACING);
        assert_eq!(o.top(), area.top());
        assert_eq!(b.actions.top(), area.bottom() + CAPTURE_GAP);
        assert_eq!(b.actions.right(), area.right());
        for r in [b.tools, o, b.actions] {
            assert!(!r.intersects(area), "{r:?} over the area");
        }

        // At the right edge: tools and options go to the area's left.
        let area = Rect::from_min_max(pos2(1100.0, 100.0), pos2(1590.0, 600.0));
        let b = place(area, true);
        assert_eq!(b.tools.right(), area.left() - CAPTURE_GAP);
        assert_eq!(b.options.unwrap().right(), b.tools.left() - BAR_SPACING);

        // At the bottom: the actions go above the area, still ending at its right edge.
        let area = Rect::from_min_max(pos2(300.0, 400.0), pos2(1000.0, 880.0));
        let b = place(area, true);
        assert_eq!(b.actions.bottom(), area.top() - CAPTURE_GAP);
        assert_eq!(b.actions.right(), area.right());

        // An area lower than the tools: the column keeps one piece and runs past its bottom.
        let area = Rect::from_min_max(pos2(300.0, 100.0), pos2(1000.0, 300.0));
        let b = place(area, false);
        assert_eq!(b.tools.top(), area.top());
        assert_eq!(b.tools.size(), tools, "one column, never cut");
        assert!(b.tools.bottom() > area.bottom());

        // An area narrower than the actions: they run past its left edge, ending at its right.
        let area = Rect::from_min_max(pos2(700.0, 100.0), pos2(860.0, 600.0));
        let b = place(area, true);
        assert_eq!(b.actions.right(), area.right());
        assert!(b.actions.left() < area.left());

        // Room for the tools on each side but not for tools and options together: the tools
        // to the right, the options to the area's left, neither over the area.
        let area = Rect::from_min_max(pos2(240.0, 100.0), pos2(1352.0, 600.0));
        let b = place(area, true);
        assert_eq!(b.tools.left(), area.right() + CAPTURE_GAP);
        let o = b.options.unwrap();
        assert_eq!(o.right(), area.left() - CAPTURE_GAP);
        assert!(!o.intersects(area) && !b.tools.intersects(area));

        // The same with the options a pixel wider than the margin leaves: they touch the
        // screen's edge rather than cover the area.
        let screen_s = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 720.0));
        let area = Rect::from_min_max(pos2(243.2, 122.4), pos2(1036.8, 554.4));
        let b = capture_bars(
            area,
            vec2(46.0, 451.0),
            Some(vec2(224.0, 171.0)),
            vec2(388.0, 42.0),
            screen_s,
        );
        let o = b.options.unwrap();
        assert!(!o.intersects(area) && !b.tools.intersects(area), "{b:?}");
        assert!(screen_s.contains_rect(o), "{o:?}");

        // A tiny area in a corner: everything on screen and apart.
        place(
            Rect::from_min_max(pos2(4.0, 860.0), pos2(40.0, 896.0)),
            true,
        );
        place(
            Rect::from_min_max(pos2(1560.0, 4.0), pos2(1596.0, 40.0)),
            true,
        );
    }

    #[test]
    fn the_tools_stay_put_when_the_options_appear() {
        let (tools, opts, actions) = (vec2(44.0, 450.0), vec2(226.0, 260.0), vec2(380.0, 42.0));
        let check = |area: Rect, screen: Rect| {
            let without = capture_bars(area, tools, None, actions, screen).tools;
            let with = capture_bars(area, tools, Some(opts), actions, screen).tools;
            assert_eq!(with, without, "the tools moved for {area:?} on {screen:?}");
        };
        // The case that showed it: 6 px short of room for both on the right.
        let big = Rect::from_min_size(pos2(0.0, 0.0), vec2(1600.0, 900.0));
        check(
            Rect::from_min_max(pos2(100.0, 200.0), pos2(1310.0, 600.0)),
            big,
        );
        for size in [
            vec2(1280.0, 720.0),
            vec2(1600.0, 900.0),
            vec2(1920.0, 1080.0),
        ] {
            let screen = Rect::from_min_size(pos2(0.0, 0.0), size);
            for left in (0..size.x as i32 - 40).step_by(37) {
                for right in (left + 40..=size.x as i32).step_by(53) {
                    for (top, bottom) in [(100.0, 650.0), (20.0, size.y - 20.0), (300.0, 400.0)] {
                        let area =
                            Rect::from_min_max(pos2(left as f32, top), pos2(right as f32, bottom));
                        check(area, screen);
                    }
                }
            }
        }
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
