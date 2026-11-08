//! Where floating bars go: the object bar over the selection, kept inside the canvas.

use egui::{Rect, Vec2, pos2};

/// Space between the object bar and the object.
pub const GAP: f32 = 10.0;

/// The object bar's rect, `size` big: centred over `object` and `GAP` above it; below it when the
/// canvas has no room above; at the canvas's top when neither fits. Always inside `canvas`.
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
    Rect::from_min_size(pos2(x, y), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

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

        // An object filling the canvas: still inside it.
        for o in [canvas, canvas.expand(50.0)] {
            assert!(canvas.contains_rect(bar(o)), "{:?}", bar(o));
        }
    }
}
