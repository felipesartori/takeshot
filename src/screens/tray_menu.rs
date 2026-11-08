//! The tray menu Takeshot draws itself on Windows and macOS: a borderless window by the tray icon, in the theme.

use crate::system::tray::{TrayAction, TrayEntry};
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::{Tokens, UiMode};
use egui::{Color32, Pos2, Rect, Sense, Stroke, TextureId, Ui, Vec2, pos2, vec2};
use std::path::{Path, PathBuf};

const MENU_WIDTH: f32 = 280.0;
const SUB_WIDTH: f32 = 270.0;
const PAD: f32 = 6.0;
const ROW: f32 = 32.0;
const HEADER: f32 = 38.0;
const SEPARATOR: f32 = 9.0;
const RECENT_ROW: f32 = 44.0;

/// The submenus: recent captures and the light/dark mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sub {
    Recents,
    Mode,
}

#[derive(Clone, PartialEq, Debug)]
pub enum MenuOutcome {
    Open,
    Closed,
    Chosen(TrayAction),
}

/// Where the menu goes: its right edge on the icon's, above the icon when it fits (taskbar at the
/// bottom), below it otherwise (macOS menu bar), always inside the monitor.
pub fn menu_pos(icon: Rect, menu: Vec2, monitor: Rect) -> Pos2 {
    let x = (icon.max.x - menu.x).clamp(monitor.min.x, (monitor.max.x - menu.x).max(monitor.min.x));
    let above = icon.min.y - menu.y - 4.0;
    let y = if above >= monitor.min.y {
        above
    } else {
        icon.max.y + 4.0
    };
    pos2(
        x,
        y.clamp(monitor.min.y, (monitor.max.y - menu.y).max(monitor.min.y)),
    )
}

/// Watches focus across the menu and its flyout. The menu closes on Esc, or once neither window
/// has had focus for two frames in a row after one of them had it (a click elsewhere; moving from
/// one window to the other can read as a single frame without focus). Before the first focus it
/// stays: the windows were just opened.
#[derive(Clone, Copy, Default, Debug)]
pub struct FocusWatch {
    had: bool,
    unfocused: u8,
}

impl FocusWatch {
    /// Feeds one frame's focus of the menu and of the flyout (`None` when unknown or absent);
    /// true when the menu should close.
    pub fn step(&mut self, main: Option<bool>, sub: Option<bool>, esc: bool) -> bool {
        if esc {
            return true;
        }
        if main == Some(true) || sub == Some(true) {
            self.had = true;
            self.unfocused = 0;
            return false;
        }
        if self.had && (main.is_some() || sub.is_some()) {
            self.unfocused = self.unfocused.saturating_add(1);
        }
        self.unfocused >= 2
    }

    /// The menu or its flyout has had the focus since opening.
    pub fn had_focus(&self) -> bool {
        self.had
    }

    /// One frame without focus went by: the next one decides, so it must come.
    pub fn deciding(&self) -> bool {
        self.unfocused > 0
    }
}

fn menu_size(entries: &[TrayEntry]) -> Vec2 {
    let h: f32 = entries
        .iter()
        .map(|e| match e {
            TrayEntry::Separator => SEPARATOR,
            _ => ROW,
        })
        .sum();
    vec2(MENU_WIDTH, HEADER + h + PAD * 2.0)
}

fn sub_size(sub: Sub, recents: usize) -> Vec2 {
    let h = match sub {
        Sub::Recents => recents as f32 * RECENT_ROW + SEPARATOR + ROW,
        Sub::Mode => 3.0 * ROW,
    };
    vec2(SUB_WIDTH, h + PAD * 2.0)
}

fn icon_for(a: &TrayAction) -> Ico {
    match a {
        TrayAction::Capture(crate::system::hotkeys::Shot::Area) => Ico::Scan,
        TrayAction::Capture(crate::system::hotkeys::Shot::Full) => Ico::Monitor,
        TrayAction::CaptureIn3s => Ico::Timer,
        TrayAction::Show(crate::system::instance::Page::Start) => Ico::Home,
        TrayAction::Show(crate::system::instance::Page::Style) => Ico::Palette,
        TrayAction::Show(crate::system::instance::Page::Settings) => Ico::Settings,
        TrayAction::Show(crate::system::instance::Page::About) => Ico::Info,
        TrayAction::Quit => Ico::Power,
        _ => Ico::Images,
    }
}

/// What the main menu body did this frame: a chosen action, and the submenu row under the pointer.
pub struct MainOutcome {
    pub chosen: Option<TrayAction>,
    pub sub: Option<(Sub, Rect)>,
    /// The pointer is on a plain row, so an open flyout should close.
    pub plain_hovered: bool,
}

/// The main menu: a header with the shortcut status chip, then the entries.
pub fn main_body(
    ui: &mut Ui,
    t: &Tokens,
    entries: &[TrayEntry],
    chip: (kit::ChipState, &str),
) -> MainOutcome {
    let mut out = MainOutcome {
        chosen: None,
        sub: None,
        plain_hovered: false,
    };
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.horizontal(|ui| {
        ui.set_height(HEADER);
        ui.add_space(8.0);
        ui.label(crate::ui::brand::wordmark(t.theme, 13.0).color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            kit::status_chip(ui, t, chip.0, chip.1);
        });
    });
    for e in entries {
        match e {
            TrayEntry::Separator => kit::menu_line(ui, t),
            TrayEntry::Item {
                label,
                keys,
                action,
            } => {
                let r =
                    kit::menu_item(ui, t, Some(icon_for(action)), label, keys.as_deref(), false);
                out.plain_hovered |= r.hovered();
                if r.clicked() {
                    out.chosen = Some(action.clone());
                }
            }
            TrayEntry::Recents(_) => {
                let r = kit::menu_item(ui, t, Some(Ico::Images), tr(Msg::Recent), None, true);
                if r.hovered() || r.clicked() {
                    out.sub = Some((Sub::Recents, r.rect));
                }
            }
            TrayEntry::Mode(_) => {
                let r = kit::menu_item(ui, t, Some(Ico::SunMoon), tr(Msg::Theme), None, true);
                if r.hovered() || r.clicked() {
                    out.sub = Some((Sub::Mode, r.rect));
                }
            }
        }
    }
    out
}

/// The recent captures: click copies, Shift+click opens in the editor; then "Open captures folder".
pub fn recents_body(
    ui: &mut Ui,
    t: &Tokens,
    files: &[PathBuf],
    thumb: &dyn Fn(&Path) -> Option<TextureId>,
) -> Option<TrayAction> {
    let mut chosen = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    for f in files {
        let (rect, resp) =
            ui.allocate_exact_size(vec2(ui.available_width(), RECENT_ROW), Sense::click());
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, t.radius, t.hover);
        }
        let img = Rect::from_min_size(rect.min + vec2(8.0, 7.0), vec2(44.0, 30.0));
        match thumb(f) {
            Some(id) => {
                p.image(
                    id,
                    img,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                p.rect_filled(img, 3.0, t.raised);
            }
        }
        let x = img.max.x + 10.0;
        p.text(
            pos2(x, rect.min.y + 8.0),
            egui::Align2::LEFT_TOP,
            crate::system::recents::recent_label(f),
            font(12.5, Weight::Medium),
            t.text,
        );
        p.text(
            pos2(x, rect.min.y + 25.0),
            egui::Align2::LEFT_TOP,
            tr(Msg::RecentHint),
            font(11.0, Weight::Regular),
            t.faint,
        );
        if resp.clicked() {
            let shift = ui.input(|i| i.modifiers.shift);
            chosen = Some(if shift {
                TrayAction::EditRecent(f.clone())
            } else {
                TrayAction::CopyRecent(f.clone())
            });
        }
    }
    kit::menu_line(ui, t);
    if kit::menu_item(
        ui,
        t,
        Some(Ico::FolderOpen),
        tr(Msg::OpenCapturesFolder),
        None,
        false,
    )
    .clicked()
    {
        chosen = Some(TrayAction::OpenCapturesFolder);
    }
    chosen
}

/// Light, Dark and System, with a check on the current one.
pub fn mode_body(ui: &mut Ui, t: &Tokens, current: UiMode) -> Option<TrayAction> {
    let mut chosen = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    for (m, msg) in [
        (UiMode::Light, Msg::ModeLight),
        (UiMode::Dark, Msg::ModeDark),
        (UiMode::System, Msg::ModeSystem),
    ] {
        let icon = (m == current).then_some(Ico::Check);
        if kit::menu_item(ui, t, icon, tr(msg), None, false).clicked() {
            chosen = Some(TrayAction::SetMode(m));
        }
    }
    chosen
}

/// The title of the menu windows, which only the frame fix below looks for.
const MENU_TITLE: &str = "Takeshot menu";

/// The open menu and its submenu, each a borderless always-on-top window.
pub struct TrayMenu {
    at: Option<Pos2>,
    monitor: Rect,
    focus: FocusWatch,
    sub: Option<(Sub, Pos2)>,
}

impl Default for TrayMenu {
    fn default() -> Self {
        Self {
            at: None,
            monitor: Rect::NOTHING,
            focus: FocusWatch::default(),
            sub: None,
        }
    }
}

impl TrayMenu {
    /// Opens next to the tray icon (`icon` and `monitor` in points).
    pub fn open(&mut self, icon: Rect, monitor: Rect, entries: &[TrayEntry]) {
        self.at = Some(menu_pos(icon, menu_size(entries), monitor));
        self.monitor = monitor;
        self.focus = FocusWatch::default();
        self.sub = None;
    }

    pub fn is_open(&self) -> bool {
        self.at.is_some()
    }

    pub fn close(&mut self) {
        self.at = None;
        self.sub = None;
    }

    /// Draws the menu windows for this frame and reports what happened.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        t: &Tokens,
        entries: &[TrayEntry],
        chip: (kit::ChipState, &str),
        thumb: &dyn Fn(&Path) -> Option<TextureId>,
    ) -> MenuOutcome {
        let Some(at) = self.at else {
            return MenuOutcome::Closed;
        };
        let recents: Vec<PathBuf> = entries
            .iter()
            .find_map(|e| match e {
                TrayEntry::Recents(r) => Some(r.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let mode = entries
            .iter()
            .find_map(|e| match e {
                TrayEntry::Mode(m) => Some(*m),
                _ => None,
            })
            .unwrap_or_default();
        let frame = egui::Frame::new()
            .fill(t.surf)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(10)
            .inner_margin(PAD);
        let builder = |pos: Pos2, size: Vec2| {
            egui::ViewportBuilder::default()
                .with_title(MENU_TITLE)
                .with_decorations(false)
                .with_always_on_top()
                .with_taskbar(false)
                .with_resizable(false)
                .with_transparent(true)
                .with_position(pos)
                .with_inner_size(size)
        };
        let mut chosen = None;
        let mut main_focus = None;
        let mut sub_focus = None;
        let mut esc = false;
        let main_id = egui::ViewportId::from_hash_of("takeshot-tray-menu");
        let (hovered_sub, plain) =
            ctx.show_viewport_immediate(main_id, builder(at, menu_size(entries)), |ui, _| {
                main_focus = ui.input(|i| i.viewport().focused);
                // Only until it first has the focus: afterwards a window that takes it keeps it.
                if !self.focus.had_focus()
                    && main_focus != Some(true)
                    && sub_focus.is_none()
                    && self.sub.is_none()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                esc |= ui.input(|i| i.key_pressed(egui::Key::Escape));
                let mut out = (None, false);
                egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                    let r = main_body(ui, t, entries, chip);
                    chosen = chosen.take().or(r.chosen);
                    out = (r.sub, r.plain_hovered);
                });
                (out.0.map(|(s, row)| (s, at.y + row.min.y - PAD)), out.1)
            });
        if plain {
            self.sub = None;
        }
        if let Some((s, row_y)) = hovered_sub {
            let size = sub_size(s, recents.len());
            // Opens to the left of the menu (the tray sits at the right), or to the right without room.
            let left = at.x - size.x - 4.0;
            let x = if left >= self.monitor.min.x {
                left
            } else {
                at.x + MENU_WIDTH + 4.0
            };
            // min then max, not clamp: a monitor smaller than the flyout must not panic.
            let y = row_y
                .min(self.monitor.max.y - size.y)
                .max(self.monitor.min.y);
            self.sub = Some((s, pos2(x, y)));
        }
        if let Some((s, pos)) = self.sub {
            let sub_id = egui::ViewportId::from_hash_of(("takeshot-tray-sub", s as u8));
            ctx.show_viewport_immediate(
                sub_id,
                builder(pos, sub_size(s, recents.len())),
                |ui, _| {
                    sub_focus = ui.input(|i| i.viewport().focused);
                    esc |= ui.input(|i| i.key_pressed(egui::Key::Escape));
                    egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                        let r = match s {
                            Sub::Recents => recents_body(ui, t, &recents, thumb),
                            Sub::Mode => mode_body(ui, t, mode),
                        };
                        chosen = chosen.take().or(r);
                    });
                },
            );
        }
        // Rounded corners stay clear: no 1 px top line or square shadow from Windows.
        crate::system::borderless::flatten_borderless(MENU_TITLE);
        let close = self.focus.step(main_focus, sub_focus, esc);
        let outcome = match chosen {
            Some(a) => MenuOutcome::Chosen(a),
            None if close => MenuOutcome::Closed,
            None => {
                if self.focus.deciding() {
                    ctx.request_repaint();
                }
                return MenuOutcome::Open;
            }
        };
        self.close();
        // The windows only go away on the next pass, when they are no longer shown: ask for it.
        ctx.request_repaint();
        outcome
    }
}

#[cfg(test)]
mod tests {
    /// The status chip the menu tests show.
    const READY: (kit::ChipState, &str) = (kit::ChipState::Ready, "PrtSc ready");
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

    #[test]
    fn the_tray_menu_header_follows_the_theme() {
        use crate::ui::tokens::{ThemeId, tokens};
        for (theme, word) in [(ThemeId::TakeUi, "Takeshot"), (ThemeId::Coral, "takeshot")] {
            let t = tokens(theme, true);
            let painted = frames(&t, vec![], |ui| {
                main_body(ui, &t, &[], (kit::ChipState::Ready, "PrtSc"));
            });
            assert!(
                painted.texts.iter().any(|(s, _)| s == word),
                "{theme:?}: {:?}",
                painted.texts
            );
        }
    }
    use crate::system::hotkeys::Shot;
    use crate::system::tray::tray_entries;
    use crate::ui::tokens::{ThemeId, UiMode, tokens};
    use egui::{Event, Modifiers, PointerButton};

    #[test]
    fn menu_sits_above_the_icon_and_inside_the_monitor() {
        let monitor = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
        let menu = vec2(300.0, 420.0);
        // Windows taskbar at the bottom, icon near the right edge.
        let icon = Rect::from_min_size(pos2(1800.0, 1050.0), vec2(24.0, 24.0));
        let at = menu_pos(icon, menu, monitor);
        let r = Rect::from_min_size(at, menu);
        assert!(r.max.y <= icon.min.y, "above the icon: {r:?}");
        assert!(monitor.contains_rect(r), "inside the monitor: {r:?}");
        // macOS menu bar at the top: below the icon.
        let icon = Rect::from_min_size(pos2(1500.0, 0.0), vec2(24.0, 24.0));
        let r = Rect::from_min_size(menu_pos(icon, menu, monitor), menu);
        assert!(r.min.y >= icon.max.y, "below the icon: {r:?}");
        assert!(monitor.contains_rect(r), "{r:?}");
    }

    #[test]
    fn focus_keeps_the_menu_while_the_menu_or_its_flyout_has_it() {
        let mut w = FocusWatch::default();
        assert!(!w.step(None, None, false), "unknown focus");
        assert!(
            !w.step(Some(false), None, false),
            "just opened: no focus yet"
        );
        assert!(!w.step(Some(true), None, false));
        assert!(
            !w.step(Some(false), Some(true), false),
            "moved into the flyout"
        );
        assert!(
            !w.step(Some(false), Some(false), false),
            "one frame between windows"
        );
        assert!(
            w.step(Some(false), Some(false), false),
            "focus went elsewhere"
        );
        assert!(FocusWatch::default().step(None, None, true), "Esc");
    }

    #[test]
    fn the_menu_lets_another_window_keep_the_focus_and_closes() {
        // The capture overlay takes the focus: the menu must neither take it back nor wait for
        // some other event to close.
        let t = tokens(ThemeId::TakeUi, true);
        let entries = tray_entries(&[], UiMode::Dark, "PrintScreen", "shift+PrintScreen");
        let icon = Rect::from_min_size(pos2(560.0, 760.0), vec2(24.0, 24.0));
        let monitor = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 800.0));
        let mut menu = TrayMenu::default();
        menu.open(icon, monitor, &entries);
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        let mut n = 0.0;
        let mut frame = |focused: bool, menu: &mut TrayMenu| {
            n += 1.0;
            let mut input = egui::RawInput {
                screen_rect: Some(monitor),
                time: Some(n),
                ..Default::default()
            };
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .focused = Some(focused);
            let mut out = MenuOutcome::Open;
            let mut full = ctx.run_ui(input, |ui| {
                out = menu.show(ui.ctx(), &t, &entries, READY, &|_| None);
            });
            full.textures_delta.clear();
            let vp = &full.viewport_output[&egui::ViewportId::ROOT];
            let asks_focus = vp
                .commands
                .iter()
                .any(|c| matches!(c, egui::ViewportCommand::Focus));
            (out, asks_focus, vp.repaint_delay)
        };
        let (_, asks_focus, _) = frame(false, &mut menu);
        assert!(asks_focus, "just opened: it asks for the focus");
        assert_eq!(frame(true, &mut menu).0, MenuOutcome::Open);
        let (out, asks_focus, wake) = frame(false, &mut menu);
        assert_eq!(
            out,
            MenuOutcome::Open,
            "one frame may be the move into the flyout"
        );
        assert!(!asks_focus, "it leaves the focus where it went");
        assert_eq!(
            wake,
            std::time::Duration::ZERO,
            "it asks for the frame that decides"
        );
        assert_eq!(frame(false, &mut menu).0, MenuOutcome::Closed);
    }

    #[test]
    fn the_real_menu_closes_on_escape_and_after_a_choice() {
        let t = tokens(ThemeId::TakeUi, true);
        let entries = tray_entries(&[], UiMode::Dark, "PrintScreen", "shift+PrintScreen");
        let icon = Rect::from_min_size(pos2(560.0, 760.0), vec2(24.0, 24.0));
        let monitor = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 800.0));
        let mut menu = TrayMenu::default();
        menu.open(icon, monitor, &entries);
        let esc = Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let mut last = None;
        let step = |ui: &mut egui::Ui, menu: &mut TrayMenu, last: &mut Option<MenuOutcome>| {
            let o = menu.show(ui.ctx(), &t, &entries, READY, &|_| None);
            if !matches!(o, MenuOutcome::Open) {
                *last = Some(o);
            }
        };
        drive(
            |ui| {
                step(ui, &mut menu, &mut last);
                None::<()>
            },
            vec![vec![], vec![esc]],
        );
        assert!(matches!(last.take(), Some(MenuOutcome::Closed)));
        assert!(!menu.is_open());

        menu.open(icon, monitor, &entries);
        let label = match &entries[0] {
            TrayEntry::Item { label, .. } => label.clone(),
            _ => unreachable!(),
        };
        let (_, painted) = drive(
            |ui| {
                step(ui, &mut menu, &mut last);
                None::<()>
            },
            vec![vec![]],
        );
        let at = center_of(&painted, &label);
        drive(
            |ui| {
                step(ui, &mut menu, &mut last);
                None::<()>
            },
            click_at(at, false),
        );
        assert!(matches!(
            last,
            Some(MenuOutcome::Chosen(TrayAction::Capture(Shot::Area)))
        ));
        assert!(!menu.is_open());
    }

    /// Runs a body over a few frames with these events, returning the last frame's answer and the
    /// screen rect of every painted text.
    fn drive<T>(
        mut body: impl FnMut(&mut egui::Ui) -> Option<T>,
        frames: Vec<Vec<Event>>,
    ) -> (Option<T>, Vec<(String, Rect)>) {
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        let mut answer = None;
        let mut painted = Vec::new();
        for (i, ev) in std::iter::once(vec![]).chain(frames).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 800.0))),
                time: Some(i as f64 * 0.016),
                events: ev,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                answer = body(ui);
            });
            full.textures_delta.clear();
            painted.clear();
            for c in &full.shapes {
                if let egui::Shape::Text(t) = &c.shape {
                    painted.push((
                        t.galley.text().to_owned(),
                        t.galley.rect.translate(t.pos.to_vec2()),
                    ));
                }
            }
        }
        (answer, painted)
    }

    fn click_at(p: Pos2, shift: bool) -> Vec<Vec<Event>> {
        let m = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        let b = |down| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed: down,
            modifiers: m,
        };
        vec![
            vec![Event::ModifiersChanged(m), Event::PointerMoved(p)],
            vec![Event::ModifiersChanged(m), b(true)],
            vec![Event::ModifiersChanged(m), b(false)],
        ]
    }

    fn center_of(painted: &[(String, Rect)], text: &str) -> Pos2 {
        painted
            .iter()
            .find(|(s, _)| s == text)
            .map(|(_, r)| r.center())
            .unwrap_or_else(|| panic!("{text:?} not painted"))
    }

    #[test]
    fn choosing_a_main_row_returns_its_action() {
        let t = tokens(ThemeId::TakeUi, true);
        let entries = tray_entries(&[], UiMode::Dark, "PrintScreen", "shift+PrintScreen");
        let label = match &entries[0] {
            TrayEntry::Item { label, .. } => label.clone(),
            _ => unreachable!(),
        };
        let body = |ui: &mut egui::Ui| main_body(ui, &t, &entries, READY).chosen;
        let (none, painted) = drive(body, vec![]);
        assert!(none.is_none());
        let at = center_of(&painted, &label);
        let (chosen, _) = drive(body, click_at(at, false));
        assert_eq!(chosen, Some(TrayAction::Capture(Shot::Area)));
    }

    #[test]
    fn shift_click_on_a_recent_opens_it_in_the_editor() {
        let t = tokens(ThemeId::TakeUi, true);
        let files = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let names: Vec<String> = files
            .iter()
            .map(|f| crate::system::recents::recent_label(f))
            .collect();
        let body = |ui: &mut egui::Ui| recents_body(ui, &t, &files, &|_| None);
        let (_, painted) = drive(body, vec![]);
        let at = center_of(&painted, &names[1]);
        let (copy, _) = drive(body, click_at(at, false));
        assert_eq!(copy, Some(TrayAction::CopyRecent(files[1].clone())));
        let (edit, _) = drive(body, click_at(at, true));
        assert_eq!(edit, Some(TrayAction::EditRecent(files[1].clone())));
    }
}
