//! The TakeUI skin (option A of the prototypes): the TakeFlow family look.

pub mod capture;
pub mod editor;
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

    /// The colour a mesh's block carries.
    pub const MESH: Color32 = Color32::PLACEHOLDER;

    /// What the last frame of a run painted, and the pointer it asked for.
    pub struct Painted {
        pub texts: Vec<(String, Rect)>,
        pub blocks: Vec<Block>,
        pub cursor: egui::CursorIcon,
    }

    /// Runs two warm-up frames (so widths remembered from a frame before settle), then one frame
    /// per event list at `size`, each drawn by `draw` in the theme's look; returns what the last
    /// one painted.
    fn run_frames(
        shared: &mut Shared,
        size: (f32, f32),
        dark: bool,
        frames: Vec<Vec<Event>>,
        mut draw: impl FnMut(&mut egui::Ui, &Tokens, &mut Shared),
    ) -> Painted {
        shared.settings.ui_mode = if dark {
            crate::ui::tokens::UiMode::Dark
        } else {
            crate::ui::tokens::UiMode::Light
        };
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        let mut out = Painted {
            texts: Vec::new(),
            blocks: Vec::new(),
            cursor: egui::CursorIcon::Default,
        };
        for (i, events) in [vec![], vec![]].into_iter().chain(frames).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0, size.1))),
                time: Some(i as f64 * 0.05),
                events,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                let t = shared.tokens(None);
                crate::skins::begin_frame(ui.ctx(), &t);
                draw(ui, &t, shared);
            });
            full.textures_delta.clear();
            out.texts.clear();
            out.blocks.clear();
            for c in &full.shapes {
                walk(&c.shape, &mut out.texts, &mut out.blocks);
            }
            out.cursor = full.platform_output.cursor_icon;
        }
        out
    }

    /// The main window over `run_frames`; returns every action and the texts painted in the last
    /// frame with their screen rects.
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
        let mut acts = Vec::new();
        let p = run_frames(shared, size, dark, frames, |ui, t, shared| {
            acts.extend(crate::skins::main_window(ui, t, home, shared))
        });
        (acts, p.texts, p.blocks)
    }

    /// The editor window over `run_frames`; each frame's actions are carried out right away, as
    /// `Editor::ui` does.
    pub fn drive_editor(
        ed: &mut crate::screens::editor::Editor,
        shared: &mut Shared,
        size: (f32, f32),
        dark: bool,
        frames: Vec<Vec<Event>>,
    ) -> (Vec<crate::screens::editor::EditorAction>, Painted) {
        let mut acts = Vec::new();
        let p = run_frames(shared, size, dark, frames, |ui, t, shared| {
            let frame = crate::skins::editor(ui, t, ed, shared);
            for a in &frame {
                ed.apply(a.clone(), ui.ctx(), None, shared);
            }
            acts.extend(frame);
        });
        (acts, p)
    }

    /// The capture's bars and hint over `run_frames`, the window as large as the sample picture;
    /// returns the actions without carrying them out (they reach the clipboard and the disk).
    pub fn run_capture(
        ov: &mut crate::screens::overlay::Overlay,
        shared: &mut Shared,
        frames: Vec<Vec<Event>>,
    ) -> (Vec<crate::screens::overlay::CaptureAction>, Painted) {
        let mut acts = Vec::new();
        let p = run_frames(shared, (1600.0, 900.0), true, frames, |ui, t, shared| {
            let screen = ui.max_rect();
            acts.extend(crate::skins::capture(ui, t, ov, shared, screen));
        });
        (acts, p)
    }

    /// `drive_editor`, as actions, texts and blocks.
    pub fn run_editor(
        ed: &mut crate::screens::editor::Editor,
        shared: &mut Shared,
        size: (f32, f32),
        dark: bool,
        frames: Vec<Vec<Event>>,
    ) -> (
        Vec<crate::screens::editor::EditorAction>,
        Vec<(String, Rect)>,
        Vec<Block>,
    ) {
        let (acts, p) = drive_editor(ed, shared, size, dark, frames);
        (acts, p.texts, p.blocks)
    }

    fn walk(s: &egui::Shape, texts: &mut Vec<(String, Rect)>, blocks: &mut Vec<Block>) {
        match s {
            egui::Shape::Text(t) => texts.push((
                t.galley.text().to_owned(),
                t.galley.rect.translate(t.pos.to_vec2()),
            )),
            egui::Shape::Rect(r) => blocks.push((r.rect, r.fill)),
            // A line counts as a block of its stroke's colour; a mesh (a gradient, a picture) as
            // a block of `MESH`.
            egui::Shape::LineSegment { points, stroke } => {
                blocks.push((Rect::from_two_pos(points[0], points[1]), stroke.color))
            }
            egui::Shape::Mesh(m) => blocks.push((m.calc_bounds(), MESH)),
            // A circle (a colour or width dot) as the square around it, in its fill.
            egui::Shape::Circle(c) => blocks.push((
                Rect::from_center_size(c.center, egui::Vec2::splat(c.radius * 2.0)),
                c.fill,
            )),
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
