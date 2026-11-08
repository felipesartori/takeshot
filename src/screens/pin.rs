//! A capture pinned on top of other windows, in the theme: drag the middle to move, an edge or a
//! corner to resize (the picture keeps its proportions, the opposite side stays), wheel to zoom,
//! double click or Esc to close, and a menu (copy, save as, open in the editor, original size,
//! close).

use crate::screens::overlay::color_image;
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, Ico, MenuRow};
use crate::ui::tokens::Tokens;
use crate::{Shared, Switch};
use egui::{Rect, ResizeDirection, Sense, Stroke, Ui, Vec2, pos2, vec2};
use image::RgbaImage;

/// What the user asked for on a pinned picture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PinAction {
    Copy,
    Save,
    Edit,
    OriginalSize,
    Close,
}

pub struct Pin {
    img: RgbaImage,
    tex: Option<egui::TextureHandle>,
    zoom: f32,
    base_size: egui::Vec2,
    /// The window title went out in the user's language.
    titled: bool,
    /// An edge or corner being dragged: its direction, the window and the pointer (on the screen)
    /// when the drag began.
    resizing: Option<(ResizeDirection, Rect, egui::Pos2)>,
}

/// The pinned window resized from `start` by dragging `dir`'s edge or corner `delta` on the
/// screen: the picture keeps its proportions, the opposite side or corner stays (an edge keeps the
/// other axis centred), between 15 % and 400 % of `base`, the picture's size at 100 %.
pub fn resize_window(start: Rect, dir: ResizeDirection, delta: Vec2, base: Vec2) -> Rect {
    use ResizeDirection as D;
    let (sx, sy): (f32, f32) = match dir {
        D::East => (1.0, 0.0),
        D::West => (-1.0, 0.0),
        D::South => (0.0, 1.0),
        D::North => (0.0, -1.0),
        D::SouthEast => (1.0, 1.0),
        D::SouthWest => (-1.0, 1.0),
        D::NorthEast => (1.0, -1.0),
        D::NorthWest => (-1.0, -1.0),
    };
    let (w, h) = (start.width().max(1.0), start.height().max(1.0));
    // Along the drag's axis; a corner projects the drag on the diagonal.
    let k = if sx != 0.0 && sy != 0.0 {
        1.0 + (sx * delta.x * w + sy * delta.y * h) / (w * w + h * h)
    } else if sx != 0.0 {
        1.0 + sx * delta.x / w
    } else {
        1.0 + sy * delta.y / h
    };
    let width = (w * k).clamp(base.x * 0.15, base.x * 4.0);
    let size = vec2(width, width * h / w);
    let x = match sx {
        s if s > 0.0 => start.left(),
        s if s < 0.0 => start.right() - size.x,
        _ => start.center().x - size.x / 2.0,
    };
    let y = match sy {
        s if s > 0.0 => start.top(),
        s if s < 0.0 => start.bottom() - size.y,
        _ => start.center().y - size.y / 2.0,
    };
    Rect::from_min_size(pos2(x, y), size)
}

impl Pin {
    pub fn new(img: RgbaImage, base_size: egui::Vec2) -> Self {
        Self {
            img,
            tex: None,
            zoom: 1.0,
            base_size,
            titled: false,
            resizing: None,
        }
    }

    /// One frame: draws the picture and carries out what the user asked for.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        frame: &eframe::Frame,
        t: &Tokens,
        shared: &mut Shared,
    ) -> Switch {
        let mut switch = Switch::None;
        for a in self.draw(ui, t) {
            match a {
                PinAction::Copy => {
                    let _ = crate::system::actions::copy_image(&self.img);
                }
                PinAction::Save => {
                    if let Some(p) = crate::system::actions::ask_save_path(
                        &shared.settings.save_dir,
                        Some(frame),
                    ) {
                        let _ = crate::system::actions::save_png(&self.img, &p);
                    }
                }
                PinAction::Edit => {
                    if let Some(p) = crate::system::actions::temp_png(&self.img) {
                        let _ =
                            crate::system::actions::spawn_self(&["--edit", &p.to_string_lossy()]);
                    }
                }
                PinAction::OriginalSize => {
                    self.zoom = 1.0;
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::InnerSize(self.base_size));
                }
                PinAction::Close => switch = Switch::Close,
            }
        }
        switch
    }

    /// Draws the picture with its frame (the theme's primary colour under the pointer) and its
    /// menu; moves, zooms and returns what the user asked for.
    pub fn draw(&mut self, ui: &mut Ui, t: &Tokens) -> Vec<PinAction> {
        let ctx = ui.ctx().clone();
        if !self.titled {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(tr(Msg::PinnedTitle).into()));
            self.titled = true;
        }
        let tex = self
            .tex
            .get_or_insert_with(|| {
                ctx.load_texture("pin", color_image(&self.img), egui::TextureOptions::LINEAR)
            })
            .id();
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.scope_id().with("pin"), Sense::click_and_drag());
        ui.painter().image(
            tex,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        let border = if resp.hovered() || self.resizing.is_some() {
            t.primary
        } else {
            t.border
        };
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let mut acts = Vec::new();
        // A menu open at the start of the frame takes Esc (it closes while this frame draws).
        let menu_open = egui::Popup::is_any_open(&ctx);
        // Edges and corners resize, the middle moves.
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let zone = pointer.and_then(|p| crate::ui::chrome::edge_at(rect, p, false));
        let outer = ctx
            .input(|i| i.viewport().outer_rect)
            .unwrap_or(Rect::from_min_size(pos2(0.0, 0.0), rect.size()));
        if resp.drag_started() {
            // Where the button went down decides: the pointer has moved on by the time a drag starts.
            let origin = ui.input(|i| i.pointer.press_origin()).or(pointer);
            match origin.and_then(|o| crate::ui::chrome::edge_at(rect, o, false).map(|d| (d, o))) {
                Some((dir, o)) => self.resizing = Some((dir, outer, outer.min + o.to_vec2())),
                None => ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag),
            }
        }
        if !ui.input(|i| i.pointer.primary_down()) {
            self.resizing = None;
        }
        if let (Some((dir, start, from)), Some(p)) = (self.resizing, pointer) {
            let to = outer.min + p.to_vec2();
            let r = resize_window(start, dir, to - from, self.base_size);
            self.zoom = r.width() / self.base_size.x.max(1.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(r.min));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(r.size()));
        }
        if let Some(dir) = self.resizing.map(|(d, ..)| d).or(zone) {
            ctx.set_cursor_icon(crate::ui::chrome::cursor_for(dir));
        }
        // The corners' handles show where to grab, and the size while it changes.
        if resp.hovered() || self.resizing.is_some() {
            let p = ui.painter();
            for c in [
                rect.left_top(),
                rect.right_top(),
                rect.left_bottom(),
                rect.right_bottom(),
            ] {
                let at = pos2(
                    c.x.clamp(rect.left() + 4.0, rect.right() - 4.0),
                    c.y.clamp(rect.top() + 4.0, rect.bottom() - 4.0),
                );
                p.rect_filled(Rect::from_center_size(at, vec2(8.0, 8.0)), 2.0, t.primary);
            }
        }
        if self.resizing.is_some() {
            let label = format!("{} %", (self.zoom * 100.0).round());
            let p = ui.painter();
            let g = p.layout_no_wrap(
                label,
                crate::ui::style::font(12.0, crate::ui::style::Weight::Medium),
                t.tip_text,
            );
            let pill = Rect::from_center_size(rect.center(), g.size() + vec2(16.0, 8.0));
            p.rect_filled(pill, 6.0, t.tip_bg);
            p.galley(pill.center() - g.size() / 2.0, g, t.tip_text);
        }
        if resp.double_clicked() || (!menu_open && ui.input(|i| i.key_pressed(egui::Key::Escape))) {
            acts.push(PinAction::Close);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll.abs() > 0.5 {
            self.zoom = (self.zoom * if scroll > 0.0 { 1.06 } else { 1.0 / 1.06 }).clamp(0.15, 4.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.base_size * self.zoom));
        }
        let rows = [
            MenuRow::Item(Ico::Copy, tr(Msg::Copy), PinAction::Copy),
            MenuRow::Item(Ico::Download, tr(Msg::SaveAs), PinAction::Save),
            MenuRow::Item(Ico::Edit, tr(Msg::EditInEditor), PinAction::Edit),
            MenuRow::Item(
                Ico::Maximize,
                tr(Msg::PinOriginalSize),
                PinAction::OriginalSize,
            ),
            MenuRow::Line,
            MenuRow::Item(Ico::Close, tr(Msg::CloseWord), PinAction::Close),
        ];
        acts.extend(kit::context_menu(&resp, t, &rows));
        acts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Texts, rectangle fills, rectangle strokes and circle fills one frame painted.
    #[derive(Default)]
    struct Shapes {
        texts: Vec<(String, egui::Rect)>,
        fills: Vec<egui::Color32>,
        strokes: Vec<egui::Color32>,
        dots: Vec<egui::Color32>,
        rects: Vec<egui::Rect>,
    }

    fn shapes(clipped: &[egui::epaint::ClippedShape]) -> Shapes {
        fn walk(s: &egui::Shape, out: &mut Shapes) {
            match s {
                egui::Shape::Text(t) => out.texts.push((
                    t.galley.text().to_owned(),
                    t.galley.rect.translate(t.pos.to_vec2()),
                )),
                egui::Shape::Rect(r) => {
                    out.fills.push(r.fill);
                    out.strokes.push(r.stroke.color);
                    out.rects.push(r.rect);
                }
                egui::Shape::Circle(c) => out.dots.push(c.fill),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = Shapes::default();
        clipped.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    /// Frames of `draw` at 800 × 600 in `t`'s look, one per event list after a first empty one;
    /// returns what the last frame painted.
    fn frames(
        t: &crate::ui::tokens::Tokens,
        events: Vec<Vec<egui::Event>>,
        mut draw: impl FnMut(&mut egui::Ui),
    ) -> Shapes {
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, t);
        let mut last = Shapes::default();
        for (i, ev) in std::iter::once(Vec::new()).chain(events).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                time: Some(i as f64 * 0.05),
                events: ev,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| draw(ui));
            full.textures_delta.clear();
            last = shapes(&full.shapes);
        }
        last
    }

    fn pin() -> Pin {
        Pin::new(
            RgbaImage::from_pixel(400, 300, image::Rgba([200, 200, 200, 255])),
            egui::vec2(400.0, 300.0),
        )
    }

    fn mouse(at: egui::Pos2, button: egui::PointerButton, down: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: at,
            button,
            pressed: down,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn a_corner_drag_keeps_the_aspect_and_the_opposite_corner() {
        use egui::ResizeDirection as D;
        let base = egui::vec2(400.0, 300.0);
        let start = Rect::from_min_size(pos2(100.0, 100.0), base);
        // The bottom right corner out by 100 x 100: the top left stays, the ratio stays 4:3.
        let r = resize_window(start, D::SouthEast, egui::vec2(100.0, 100.0), base);
        assert_eq!(r.min, start.min);
        assert!((r.width() / r.height() - 4.0 / 3.0).abs() < 1e-3, "{r:?}");
        assert!(r.width() > start.width());
        // The top left corner in: the bottom right stays.
        let r = resize_window(start, D::NorthWest, egui::vec2(80.0, 60.0), base);
        assert_eq!(r.max, start.max);
        assert!((r.width() / r.height() - 4.0 / 3.0).abs() < 1e-3);
        assert!((r.width() - 320.0).abs() < 0.5, "{r:?}");
        // The right edge: the left edge stays, the height follows, centred.
        let r = resize_window(start, D::East, egui::vec2(200.0, 0.0), base);
        assert_eq!(r.left(), start.left());
        assert_eq!(r.width(), 600.0);
        assert_eq!(r.height(), 450.0);
        assert_eq!(r.center().y, start.center().y);
        // Never below 15 % nor above 400 % of the picture.
        let r = resize_window(start, D::SouthEast, egui::vec2(-1000.0, -1000.0), base);
        assert!((r.width() - 60.0).abs() < 0.5, "{r:?}");
        let r = resize_window(start, D::SouthEast, egui::vec2(9000.0, 9000.0), base);
        assert!((r.width() - 1600.0).abs() < 0.5, "{r:?}");
    }

    #[test]
    fn the_border_resizes_and_the_middle_moves() {
        use crate::ui::tokens::{ThemeId, tokens};
        let t = tokens(ThemeId::TakeUi, true);
        let left = egui::PointerButton::Primary;
        let run = |at: egui::Pos2| {
            let ctx = egui::Context::default();
            crate::system::fonts::FontLib::load().install(&ctx);
            let mut p = pin();
            let mut out = Vec::new();
            for (i, ev) in [
                vec![egui::Event::PointerMoved(at)],
                vec![mouse(at, left, true)],
                vec![egui::Event::PointerMoved(at + egui::vec2(30.0, 30.0))],
                vec![egui::Event::PointerMoved(at + egui::vec2(60.0, 60.0))],
            ]
            .into_iter()
            .enumerate()
            {
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        egui::vec2(400.0, 300.0),
                    )),
                    time: Some(i as f64 * 0.05),
                    events: ev,
                    ..Default::default()
                };
                let mut full = ctx.run_ui(input, |ui| {
                    p.draw(ui, &t);
                });
                full.textures_delta.clear();
                out.push((
                    full.platform_output.cursor_icon,
                    full.viewport_output
                        .values()
                        .flat_map(|v| v.commands.clone())
                        .collect::<Vec<_>>(),
                ));
            }
            out
        };
        // The bottom right corner: the resize pointer, then the window grows, never moved by drag.
        let corner = run(pos2(398.0, 298.0));
        assert_eq!(corner[0].0, egui::CursorIcon::ResizeSouthEast);
        let cmds: Vec<_> = corner.iter().flat_map(|(_, c)| c.clone()).collect();
        assert!(
            cmds.iter()
                .any(|c| matches!(c, egui::ViewportCommand::InnerSize(_))),
            "{cmds:?}"
        );
        assert!(
            !cmds
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::StartDrag)),
            "{cmds:?}"
        );
        // The middle: the window moves.
        let middle = run(pos2(200.0, 150.0));
        let cmds: Vec<_> = middle.iter().flat_map(|(_, c)| c.clone()).collect();
        assert!(
            cmds.iter()
                .any(|c| matches!(c, egui::ViewportCommand::StartDrag)),
            "{cmds:?}"
        );
    }

    #[test]
    fn the_pin_menu_is_translated_and_themed() {
        use crate::ui::i18n::{Lang, Msg, set_lang, t as tr};
        use crate::ui::tokens::{ThemeId, tokens};
        let at = egui::pos2(200.0, 150.0);
        let right = egui::PointerButton::Secondary;
        let open = vec![
            vec![egui::Event::PointerMoved(at)],
            vec![mouse(at, right, true)],
            vec![mouse(at, right, false)],
            vec![],
        ];
        for lang in [Lang::Pt, Lang::En] {
            set_lang(lang);
            let t = tokens(ThemeId::Coral, true);
            let mut p = pin();
            let mut acts = Vec::new();
            let painted = frames(&t, open.clone(), |ui| acts.extend(p.draw(ui, &t)));
            for m in [
                Msg::Copy,
                Msg::SaveAs,
                Msg::EditInEditor,
                Msg::PinOriginalSize,
                Msg::CloseWord,
            ] {
                assert!(
                    painted.texts.iter().any(|(s, _)| s == tr(m)),
                    "{lang:?}: {m:?} in {:?}",
                    painted.texts
                );
            }
            let copy = painted
                .texts
                .iter()
                .find(|(s, _)| s == tr(Msg::Copy))
                .map(|(_, r)| r.center())
                .unwrap();
            let left = egui::PointerButton::Primary;
            let mut p = pin();
            let mut acts = Vec::new();
            let mut events = open.clone();
            events.extend([
                vec![egui::Event::PointerMoved(copy)],
                vec![mouse(copy, left, true)],
                vec![mouse(copy, left, false)],
            ]);
            frames(&t, events, |ui| acts.extend(p.draw(ui, &t)));
            assert_eq!(acts, vec![PinAction::Copy], "{lang:?}");
        }
        set_lang(Lang::En);
    }

    #[test]
    fn the_pin_menu_is_as_wide_as_its_labels() {
        use crate::ui::i18n::{Msg, t as tr};
        use crate::ui::tokens::{ThemeId, tokens};
        let at = egui::pos2(200.0, 150.0);
        let right = egui::PointerButton::Secondary;
        let open = vec![
            vec![egui::Event::PointerMoved(at)],
            vec![mouse(at, right, true)],
            vec![mouse(at, right, false)],
            vec![],
            vec![],
        ];
        let t = tokens(ThemeId::TakeUi, true);
        let mut p = pin();
        let painted = frames(&t, open, |ui| {
            p.draw(ui, &t);
        });
        let copy = painted
            .texts
            .iter()
            .find(|(s, _)| s == tr(Msg::Copy))
            .map(|(_, r)| r.center())
            .expect("the menu");
        let w = painted
            .rects
            .iter()
            .filter(|r| (r.height() - 32.0).abs() < 0.5 && r.contains(copy))
            .map(|r| r.width())
            .fold(f32::INFINITY, f32::min);
        assert!(w <= 240.0, "a {w} px menu in an 800 px pin");
    }

    #[test]
    fn the_pin_title_is_in_the_user_s_language() {
        use crate::ui::i18n::{Lang, Msg, set_lang, t as tr};
        use crate::ui::tokens::{ThemeId, tokens};
        set_lang(Lang::Pt);
        let t = tokens(ThemeId::TakeUi, true);
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        let mut p = pin();
        let mut full = ctx.run_ui(egui::RawInput::default(), |ui| {
            p.draw(ui, &t);
        });
        full.textures_delta.clear();
        let titles: Vec<_> = full
            .viewport_output
            .values()
            .flat_map(|v| v.commands.iter())
            .filter_map(|c| match c {
                egui::ViewportCommand::Title(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(titles, vec![tr(Msg::PinnedTitle).to_string()]);
        set_lang(Lang::En);
    }

    #[test]
    fn esc_over_the_menu_closes_only_the_menu() {
        use crate::ui::tokens::{ThemeId, tokens};
        let at = egui::pos2(200.0, 150.0);
        let right = egui::PointerButton::Secondary;
        let esc = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let t = tokens(ThemeId::TakeUi, true);
        let mut p = pin();
        let mut acts = Vec::new();
        frames(
            &t,
            vec![
                vec![egui::Event::PointerMoved(at)],
                vec![mouse(at, right, true)],
                vec![mouse(at, right, false)],
                vec![],
                vec![esc],
                vec![],
            ],
            |ui| acts.extend(p.draw(ui, &t)),
        );
        assert!(!acts.contains(&PinAction::Close), "{acts:?}");
    }

    #[test]
    fn the_pin_frame_takes_the_theme_s_colour_on_hover() {
        use crate::ui::tokens::{ThemeId, tokens};
        let at = egui::pos2(200.0, 150.0);
        for theme in [ThemeId::TakeUi, ThemeId::Coral] {
            let t = tokens(theme, true);
            let mut p = pin();
            let rest = frames(&t, vec![], |ui| {
                p.draw(ui, &t);
            });
            assert!(!rest.strokes.contains(&t.primary), "{theme:?}: at rest");
            let mut p = pin();
            let over = frames(
                &t,
                vec![vec![egui::Event::PointerMoved(at)], vec![]],
                |ui| {
                    p.draw(ui, &t);
                },
            );
            assert!(over.strokes.contains(&t.primary), "{theme:?}: hovered");
        }
    }
}
