//! The TakeUI skin (option A of the prototypes): the TakeFlow family look.

pub mod home;
pub mod shell;

/// Headless frames over the main window, for the skin's tests.
#[cfg(test)]
pub(crate) mod testing {
    use crate::Shared;
    use crate::screens::home::{Home, MainAction};
    use egui::{Event, Pos2, Rect};

    /// Runs a warm-up frame and then one frame per event list at `size`; returns every action and
    /// the texts painted in the last frame with their screen rects.
    pub fn run(
        home: &mut Home,
        shared: &mut Shared,
        size: (f32, f32),
        dark: bool,
        frames: Vec<Vec<Event>>,
    ) -> (Vec<MainAction>, Vec<(String, Rect)>) {
        shared.settings.ui_mode = if dark {
            crate::ui::tokens::UiMode::Dark
        } else {
            crate::ui::tokens::UiMode::Light
        };
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        let mut acts = Vec::new();
        let mut painted = Vec::new();
        for (i, events) in std::iter::once(vec![]).chain(frames).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0, size.1))),
                time: Some(i as f64 * 0.05),
                events,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                let t = shared.tokens(None);
                crate::skins::begin_frame(ui.ctx(), &t);
                acts.extend(crate::skins::main_window(ui, &t, home, shared));
            });
            full.textures_delta.clear();
            painted.clear();
            for c in &full.shapes {
                walk(&c.shape, &mut painted);
            }
        }
        (acts, painted)
    }

    fn walk(s: &egui::Shape, out: &mut Vec<(String, Rect)>) {
        match s {
            egui::Shape::Text(t) => out.push((
                t.galley.text().to_owned(),
                t.galley.rect.translate(t.pos.to_vec2()),
            )),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }

    /// The centre of the first painted text equal to one of `texts` whose rect passes `keep`.
    pub fn painted_at(
        painted: &[(String, Rect)],
        texts: &[&str],
        keep: impl Fn(Rect) -> bool,
    ) -> Option<Pos2> {
        painted
            .iter()
            .find(|(s, r)| texts.contains(&s.as_str()) && keep(*r))
            .map(|(_, r)| r.center())
    }

    fn button(p: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    pub fn click(p: Pos2) -> Vec<Vec<Event>> {
        vec![
            vec![Event::PointerMoved(p)],
            vec![button(p, true)],
            vec![button(p, false)],
        ]
    }

    pub fn double_click(p: Pos2) -> Vec<Vec<Event>> {
        vec![
            vec![Event::PointerMoved(p)],
            vec![button(p, true)],
            vec![button(p, false)],
            vec![button(p, true)],
            vec![button(p, false)],
        ]
    }
}
