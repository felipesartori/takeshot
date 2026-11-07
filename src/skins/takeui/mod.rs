//! The TakeUI skin (option A of the prototypes): the TakeFlow family look.

pub mod home;
pub mod settings;
pub mod shell;
pub mod style;

/// Headless frames over the main window, for the skin's tests.
#[cfg(test)]
pub(crate) mod testing {
    use crate::Shared;
    use crate::screens::home::{Home, MainAction};
    use crate::ui::tokens::Tokens;
    use egui::{Color32, Event, Pos2, Rect};

    /// A painted rectangle and its fill.
    pub type Block = (Rect, Color32);

    /// Runs a warm-up frame and then one frame per event list at `size`; returns every action and
    /// the texts painted in the last frame with their screen rects.
    pub fn run(
        home: &mut Home,
        shared: &mut Shared,
        size: (f32, f32),
        dark: bool,
        frames: Vec<Vec<Event>>,
    ) -> (Vec<MainAction>, Vec<(String, Rect)>) {
        let (acts, painted, _) = run_painted(home, shared, size, dark, frames);
        (acts, painted)
    }

    /// `run`, plus the rectangles painted in the last frame.
    pub fn run_painted(
        home: &mut Home,
        shared: &mut Shared,
        size: (f32, f32),
        dark: bool,
        frames: Vec<Vec<Event>>,
    ) -> (Vec<MainAction>, Vec<(String, Rect)>, Vec<Block>) {
        shared.settings.ui_mode = if dark {
            crate::ui::tokens::UiMode::Dark
        } else {
            crate::ui::tokens::UiMode::Light
        };
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        let mut acts = Vec::new();
        let mut painted = Vec::new();
        let mut blocks = Vec::new();
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
            blocks.clear();
            for c in &full.shapes {
                walk(&c.shape, &mut painted, &mut blocks);
            }
        }
        (acts, painted, blocks)
    }

    fn walk(s: &egui::Shape, texts: &mut Vec<(String, Rect)>, blocks: &mut Vec<Block>) {
        match s {
            egui::Shape::Text(t) => texts.push((
                t.galley.text().to_owned(),
                t.galley.rect.translate(t.pos.to_vec2()),
            )),
            egui::Shape::Rect(r) => blocks.push((r.rect, r.fill)),
            // A line counts as a block of its stroke's colour.
            egui::Shape::LineSegment { points, stroke } => {
                blocks.push((Rect::from_two_pos(points[0], points[1]), stroke.color))
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts, blocks)),
            _ => {}
        }
    }

    /// The rounded box the pages sit in: the largest rectangle in the surface colour.
    pub fn content_box(blocks: &[Block], t: &Tokens) -> Rect {
        blocks
            .iter()
            .filter(|(_, c)| *c == t.surf)
            .map(|(r, _)| *r)
            .max_by(|a, b| a.area().total_cmp(&b.area()))
            .expect("the content box")
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
