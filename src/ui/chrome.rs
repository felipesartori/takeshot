//! The title bar every theme draws instead of the system's (decision 13): drag to move, double
//! click to maximize, resize from the edges, the window buttons, and the native fallback for the
//! drivers that break borderless windows (`TAKESHOT_NATIVE_TITLEBAR=1`, `--native-titlebar`).

use crate::ui::kit::{self, WindowControl};
use crate::ui::tokens::Tokens;
use egui::{CursorIcon, Pos2, Rect, ResizeDirection, Response, Sense, Ui, ViewportCommand};
use std::sync::atomic::{AtomicBool, Ordering};

/// The title bar's height, as in the design ("A · Janela principal").
pub const BAR_HEIGHT: f32 = 52.0;
/// How far in from the window border a press resizes.
pub const EDGE: f32 = 6.0;
/// How far along a side from a corner a press resizes diagonally.
pub const CORNER: f32 = 12.0;

static NATIVE: AtomicBool = AtomicBool::new(false);

/// The system's own bar is used only when asked for.
pub fn native_titlebar(flag: bool, env: Option<&str>) -> bool {
    flag || env == Some("1")
}

/// Reads the flag and `TAKESHOT_NATIVE_TITLEBAR` once at start-up.
pub fn init(flag: bool) {
    let env = std::env::var("TAKESHOT_NATIVE_TITLEBAR").ok();
    NATIVE.store(native_titlebar(flag, env.as_deref()), Ordering::Relaxed);
}

/// The windows keep the system's title bar and frame.
pub fn native() -> bool {
    NATIVE.load(Ordering::Relaxed)
}

/// Room the macOS traffic lights take at the start of the bar.
pub fn leading_inset() -> f32 {
    if cfg!(target_os = "macos") { 78.0 } else { 0.0 }
}

/// The resize direction of a press at `p`, or none inside the window or when it is maximized.
pub fn edge_at(window: Rect, p: Pos2, maximized: bool) -> Option<ResizeDirection> {
    if maximized || !window.contains(p) {
        return None;
    }
    let (l, r) = (p.x - window.left(), window.right() - p.x);
    let (t, b) = (p.y - window.top(), window.bottom() - p.y);
    let near = |a: f32, b: f32| (a < EDGE && b < CORNER) || (b < EDGE && a < CORNER);
    use ResizeDirection as D;
    Some(if near(l, t) {
        D::NorthWest
    } else if near(r, t) {
        D::NorthEast
    } else if near(l, b) {
        D::SouthWest
    } else if near(r, b) {
        D::SouthEast
    } else if l < EDGE {
        D::West
    } else if r < EDGE {
        D::East
    } else if t < EDGE {
        D::North
    } else if b < EDGE {
        D::South
    } else {
        return None;
    })
}

fn cursor_for(d: ResizeDirection) -> CursorIcon {
    use ResizeDirection as D;
    match d {
        D::North => CursorIcon::ResizeNorth,
        D::South => CursorIcon::ResizeSouth,
        D::East => CursorIcon::ResizeEast,
        D::West => CursorIcon::ResizeWest,
        D::NorthEast => CursorIcon::ResizeNorthEast,
        D::NorthWest => CursorIcon::ResizeNorthWest,
        D::SouthEast => CursorIcon::ResizeSouthEast,
        D::SouthWest => CursorIcon::ResizeSouthWest,
    }
}

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// The resize bands along the window border, on a layer above the content so a press there resizes
/// instead of reaching a widget. Call once per frame, after the content. Nothing when maximized or
/// with the native bar.
pub fn resize_edges(ui: &Ui) {
    let ctx = ui.ctx();
    if native() || maximized(ctx) {
        return;
    }
    let window = ctx.content_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let Some(dir) = edge_at(window, p, false) else {
        return;
    };
    egui::Area::new(egui::Id::unique("takeshot-resize-edge"))
        .order(egui::Order::Foreground)
        .fixed_pos(window.min)
        .show(ctx, |ui| {
            let band = Rect::from_center_size(p, egui::vec2(EDGE * 2.0, EDGE * 2.0));
            let resp = ui.interact(band, ui.scope_id().with("edge"), Sense::drag());
            ctx.set_cursor_icon(cursor_for(dir));
            // Once, on the press: the system runs the resize from there.
            if resp.hovered() && ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        });
}

/// The bar's empty area: dragging moves the window (the system restores a maximized one), a
/// double click maximizes or restores. Create it before the bar's buttons, so they stay on top.
pub fn drag_area(ui: &mut Ui, rect: Rect) -> Response {
    let resp = ui.interact(
        rect,
        ui.scope_id().with("title-drag"),
        Sense::click_and_drag(),
    );
    if native() {
        return resp;
    }
    let ctx = ui.ctx();
    if resp.double_clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized(ctx)));
    } else if resp.drag_started_by(egui::PointerButton::Primary) {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    resp
}

/// Minimize, maximize or restore, and close, laid out by the caller right to left (Close first);
/// none on macOS (the traffic lights) or with the native bar.
pub fn window_buttons(ui: &mut Ui, t: &Tokens) -> Option<WindowControl> {
    if cfg!(target_os = "macos") || native() {
        return None;
    }
    let m = maximized(ui.ctx());
    let mut chosen = None;
    for c in [
        WindowControl::Close,
        WindowControl::Maximize,
        WindowControl::Minimize,
    ] {
        if kit::window_control(ui, t, c, m).clicked() {
            chosen = Some(c);
        }
    }
    chosen
}

/// Makes this process's window titled `title` take the theme's bar: no system frame on Windows
/// (keeping the shadow), the traffic lights centred on macOS. Cheap when already done, so it can
/// run every frame (a window shown again is a new window).
pub fn adopt_window(title: &str) {
    if native() {
        return;
    }
    #[cfg(windows)]
    crate::system::borderless::frameless_with_shadow(title);
    #[cfg(target_os = "macos")]
    crate::system::mac_titlebar::center_traffic_lights(title);
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = title;
}

/// The window builder bits for the theme's bar: no system decorations, except on macOS, which
/// keeps its frame and traffic lights over the content.
pub fn frameless(vp: egui::ViewportBuilder) -> egui::ViewportBuilder {
    if native() {
        vp.with_decorations(true)
    } else if cfg!(target_os = "macos") {
        vp.with_decorations(true)
            .with_fullsize_content_view(true)
            .with_title_shown(false)
            .with_titlebar_shown(false)
    } else {
        vp.with_decorations(false)
    }
}

/// Carries out a window button; Close goes through the window's close request (the resident's X).
pub fn act(ctx: &egui::Context, c: WindowControl) {
    ctx.send_viewport_cmd(match c {
        WindowControl::Minimize => ViewportCommand::Minimized(true),
        WindowControl::Maximize => ViewportCommand::Maximized(!maximized(ctx)),
        WindowControl::Close => ViewportCommand::Close,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect, ResizeDirection, ViewportCommand, pos2, vec2};

    #[test]
    fn native_titlebar_only_on_request() {
        assert!(!native_titlebar(false, None));
        assert!(native_titlebar(true, None));
        assert!(native_titlebar(false, Some("1")));
        assert!(!native_titlebar(false, Some("0")));
        assert!(!native_titlebar(false, Some("")));
    }

    #[test]
    fn edges_and_corners_resize_unless_maximized() {
        let w = Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 700.0));
        assert_eq!(
            edge_at(w, pos2(2.0, 350.0), false),
            Some(ResizeDirection::West)
        );
        assert_eq!(
            edge_at(w, pos2(998.0, 350.0), false),
            Some(ResizeDirection::East)
        );
        assert_eq!(
            edge_at(w, pos2(500.0, 2.0), false),
            Some(ResizeDirection::North)
        );
        assert_eq!(
            edge_at(w, pos2(500.0, 698.0), false),
            Some(ResizeDirection::South)
        );
        assert_eq!(
            edge_at(w, pos2(2.0, 10.0), false),
            Some(ResizeDirection::NorthWest)
        );
        assert_eq!(
            edge_at(w, pos2(10.0, 2.0), false),
            Some(ResizeDirection::NorthWest)
        );
        assert_eq!(
            edge_at(w, pos2(997.0, 691.0), false),
            Some(ResizeDirection::SouthEast)
        );
        assert_eq!(edge_at(w, pos2(500.0, 350.0), false), None, "inside");
        assert_eq!(edge_at(w, pos2(20.0, 20.0), false), None, "past the band");
        for p in [pos2(2.0, 350.0), pos2(2.0, 2.0), pos2(500.0, 698.0)] {
            assert_eq!(edge_at(w, p, true), None, "maximized: {p:?}");
        }
    }

    /// Frames over a 600 × 300 window whose top 40 px are the drag area.
    fn bar_frames(frames: Vec<Vec<egui::Event>>) -> Vec<ViewportCommand> {
        let ctx = egui::Context::default();
        let mut commands = Vec::new();
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 300.0));
        for (i, events) in std::iter::once(vec![]).chain(frames).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(i as f64 * 0.05),
                events,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                drag_area(ui, Rect::from_min_size(Pos2::ZERO, vec2(600.0, BAR_HEIGHT)));
            });
            full.textures_delta.clear();
            if let Some(vp) = full.viewport_output.get(&egui::ViewportId::ROOT) {
                commands.extend(vp.commands.iter().cloned());
            }
        }
        commands
    }

    fn button(p: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn dragging_the_bar_moves_the_window_and_double_click_maximizes() {
        let p = pos2(300.0, 20.0);
        let drag = bar_frames(vec![
            vec![egui::Event::PointerMoved(p)],
            vec![button(p, true)],
            vec![egui::Event::PointerMoved(p + vec2(20.0, 0.0))],
            vec![egui::Event::PointerMoved(p + vec2(40.0, 0.0))],
        ]);
        assert!(
            drag.iter().any(|c| matches!(c, ViewportCommand::StartDrag)),
            "{drag:?}"
        );
        let double = bar_frames(vec![
            vec![egui::Event::PointerMoved(p)],
            vec![button(p, true)],
            vec![button(p, false)],
            vec![button(p, true)],
            vec![button(p, false)],
        ]);
        assert!(
            double
                .iter()
                .any(|c| matches!(c, ViewportCommand::Maximized(true))),
            "{double:?}"
        );
        assert!(
            !double
                .iter()
                .any(|c| matches!(c, ViewportCommand::StartDrag)),
            "a click is not a drag"
        );
    }
}
