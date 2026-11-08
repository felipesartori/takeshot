//! A capture pinned on top of other windows, in the theme: drag to move, wheel to zoom, double
//! click or Esc to close, and a menu (copy, save as, open in the editor, original size, close).

use crate::screens::overlay::color_image;
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, Ico};
use crate::ui::tokens::Tokens;
use crate::{Shared, Switch};
use egui::{Rect, Sense, Stroke, Ui, pos2, vec2};
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
}

impl Pin {
    pub fn new(img: RgbaImage, base_size: egui::Vec2) -> Self {
        Self {
            img,
            tex: None,
            zoom: 1.0,
            base_size,
            titled: false,
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
        let border = if resp.hovered() { t.primary } else { t.border };
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let mut acts = Vec::new();
        // A menu open at the start of the frame takes Esc (it closes while this frame draws).
        let menu_open = egui::Popup::is_any_open(&ctx);
        if resp.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if resp.double_clicked() || (!menu_open && ui.input(|i| i.key_pressed(egui::Key::Escape))) {
            acts.push(PinAction::Close);
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll.abs() > 0.5 {
            self.zoom = (self.zoom * if scroll > 0.0 { 1.06 } else { 1.0 / 1.06 }).clamp(0.15, 4.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.base_size * self.zoom));
        }
        resp.context_menu(|ui| {
            ui.set_min_width(200.0);
            for (icon, msg, act) in [
                (Ico::Copy, Msg::Copy, PinAction::Copy),
                (Ico::Download, Msg::SaveAs, PinAction::Save),
                (Ico::Edit, Msg::EditInEditor, PinAction::Edit),
                (Ico::Maximize, Msg::PinOriginalSize, PinAction::OriginalSize),
            ] {
                if kit::menu_item(ui, t, Some(icon), tr(msg), None, false).clicked() {
                    acts.push(act);
                    ui.close();
                }
            }
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
            ui.painter()
                .hline(line.x_range(), line.center().y, Stroke::new(1.0, t.border));
            if kit::menu_item(ui, t, Some(Ico::Close), tr(Msg::CloseWord), None, false).clicked() {
                acts.push(PinAction::Close);
                ui.close();
            }
        });
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
