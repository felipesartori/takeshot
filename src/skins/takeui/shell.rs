//! TakeUI's main window: the title bar (mark, page pills, PrtSc status, light and dark, About and
//! the window buttons) over a rounded content panel, as in "A · Janela principal" of the design.

use crate::Shared;
use crate::screens::home::{Home, MainAction, hotkey_text};
use crate::system::instance::Page;
use crate::ui::chrome::{self, BAR_HEIGHT};
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::{Tokens, UiMode};
use egui::{Align, Frame, Layout, Margin, RichText, Stroke, Ui, vec2};

pub fn main_window(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
) -> Vec<MainAction> {
    let mut acts = Vec::new();
    egui::Panel::top("takeui-title-bar")
        .exact_size(BAR_HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(t.shell))
        .show(ui, |ui| title_bar(ui, t, home, shared, &mut acts));
    egui::CentralPanel::no_frame()
        .frame(Frame::new().fill(t.shell).inner_margin(Margin::same(8)))
        .show(ui, |ui| {
            // Each page lays out its own padding: lists and columns run to the box's edges.
            Frame::new()
                .fill(t.surf)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(12)
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    match home.page() {
                        Page::Start => super::home::start_page(ui, t, home, shared, &mut acts),
                        Page::Style => super::style::style_page(ui, t, home, shared, &mut acts),
                        Page::Settings => {
                            super::settings::settings_page(ui, t, home, shared, &mut acts)
                        }
                        Page::About => {}
                    }
                });
        });
    chrome::resize_edges(ui);
    acts
}

fn title_bar(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &Shared,
    acts: &mut Vec<MainAction>,
) {
    // The empty bar moves the window; it goes first so the bar's buttons sit above it.
    chrome::drag_area(ui, ui.max_rect());
    // Measures from "A · Janela principal": 4 px between items, the mark 16 px in, the pills
    // 28 px after the name, the window buttons flush right behind a short line.
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(16.0 + chrome::leading_inset());
        let mark = home.mark(ui.ctx(), t);
        ui.add(egui::Image::new((mark, vec2(22.0, 22.0))));
        ui.add_space(12.0);
        ui.label(
            RichText::new("Takeshot")
                .font(font(17.0, Weight::Brand))
                .color(t.text),
        );
        ui.add_space(28.0);
        for (page, icon, msg) in [
            (Page::Start, Ico::Home, Msg::Home),
            (Page::Style, Ico::Palette, Msg::Style),
            (Page::Settings, Ico::Settings, Msg::Settings),
        ] {
            if kit::nav_pill(ui, t, icon, tr(msg), home.page() == page).clicked() {
                acts.push(MainAction::Page(page));
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !chrome::draws_window_buttons() {
                ui.add_space(12.0);
            } else {
                if let Some(c) = chrome::window_buttons(ui, t) {
                    acts.push(MainAction::Window(c));
                }
                ui.add_space(10.0);
                let (line, _) = ui.allocate_exact_size(vec2(1.0, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(line, 0.0, t.border);
            }
            if kit::icon_button(ui, t, Ico::Info, false, tr(Msg::About)).clicked() {
                acts.push(MainAction::About);
            }
            let (icon, tip, next) = if t.dark {
                (Ico::Moon, Msg::LightMode, UiMode::Light)
            } else {
                (Ico::Sun, Msg::DarkMode, UiMode::Dark)
            };
            if kit::icon_button(ui, t, icon, false, tr(tip)).clicked() {
                acts.push(MainAction::SetMode(next));
            }
            ui.add_space(12.0);
            let status = hotkey_text(home.hotkey_status.as_ref(), &shared.settings);
            let tip = match &status {
                Ok(s) | Err(s) => s.clone(),
            };
            kit::status_chip(ui, t, status.is_ok()).on_hover_text(tip);
        });
    });
}

#[cfg(test)]
mod tests {
    use crate::screens::home::{Home, MainAction};
    use crate::skins::takeui::testing::{click, content_box, painted_at, run, run_painted};
    use crate::system::instance::Page;
    use crate::ui::chrome::BAR_HEIGHT;
    use crate::ui::i18n::{Msg, every};
    use crate::ui::icon::{Ico, glyph};
    use crate::ui::tokens::UiMode;

    #[test]
    fn the_title_bar_matches_the_design() {
        // "A · Janela principal": a 52 px bar, the window buttons its full height, flush right.
        assert_eq!(BAR_HEIGHT, 52.0);
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        if cfg!(not(target_os = "macos")) {
            let close = blocks
                .iter()
                .map(|(r, _)| *r)
                .find(|r| {
                    r.right() == 1160.0
                        && r.top() == 0.0
                        && r.height() == BAR_HEIGHT
                        && r.width() < 100.0
                })
                .expect("the close button");
            assert_eq!(close.width(), 44.0, "{close:?}");
        }
        let name =
            painted_at(&painted, &["Takeshot"], |r| r.center().y < BAR_HEIGHT).expect("the name");
        assert!((name.y - BAR_HEIGHT / 2.0).abs() <= 1.5, "{name:?}");
    }

    #[test]
    fn no_line_runs_under_the_title_bar() {
        // The bar flows into the window's background, as in the design; only the box has a border.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let (_, _, blocks) = run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let lines: Vec<_> = blocks
            .iter()
            .filter(|(r, _)| {
                r.height() <= 2.0 && r.left() <= 1.0 && r.right() >= 1159.0 && r.top() > 20.0
            })
            .collect();
        assert!(lines.is_empty(), "{lines:?}");
    }

    #[test]
    fn the_content_box_has_the_same_gap_under_the_bar_as_around_it() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let (_, _, blocks) = run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let b = content_box(&blocks, &shared.tokens(None));
        assert_eq!(b.left(), 8.0, "{b:?}");
        assert_eq!(b.top() - BAR_HEIGHT, b.left(), "under the bar: {b:?}");
        assert_eq!(740.0 - b.bottom(), b.left(), "at the bottom: {b:?}");
    }

    #[test]
    fn the_title_bar_fits_at_the_minimum_size() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let (_, painted) = run(&mut home, &mut shared, (900.0, 600.0), true, vec![]);
        let bar: Vec<_> = painted
            .iter()
            .filter(|(_, r)| r.center().y < BAR_HEIGHT)
            .collect();
        assert!(
            bar.len() >= 6,
            "logo, three pills, status and icons: {bar:?}"
        );
        for (i, (a, ra)) in bar.iter().enumerate() {
            assert!(
                ra.left() >= 0.0 && ra.right() <= 900.0,
                "{a:?} {ra:?} outside"
            );
            for (b, rb) in &bar[i + 1..] {
                assert!(
                    !ra.shrink(0.5).intersects(rb.shrink(0.5)),
                    "{a:?} {ra:?} overlaps {b:?} {rb:?}"
                );
            }
        }
    }

    #[test]
    fn every_page_survives_a_tiny_window() {
        // Layout passes can hand a page next to no room (a first pass, a window being restored).
        for page in [Page::Start, Page::Style, Page::Settings] {
            for size in [(30.0, 30.0), (1.0, 1.0)] {
                let (mut home, mut shared) = Home::for_tests(vec![]);
                home.open(page);
                run(&mut home, &mut shared, size, true, vec![]);
            }
        }
    }

    #[test]
    fn the_page_pills_switch_page() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let style = painted_at(&painted, &every(Msg::Style), |r| r.center().y < BAR_HEIGHT)
            .expect("the Style pill");
        let (acts, _) = run(&mut home, &mut shared, (1160.0, 740.0), true, click(style));
        assert!(acts.contains(&MainAction::Page(Page::Style)), "{acts:?}");
    }

    #[test]
    fn the_moon_sets_an_explicit_mode() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let moon = painted_at(&painted, &[glyph(Ico::Moon).to_string().as_str()], |r| {
            r.center().y < BAR_HEIGHT
        })
        .expect("the moon in the dark look");
        let (acts, _) = run(&mut home, &mut shared, (1160.0, 740.0), true, click(moon));
        assert!(
            acts.contains(&MainAction::SetMode(UiMode::Light)),
            "{acts:?}"
        );
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), false, vec![]);
        let sun = painted_at(&painted, &[glyph(Ico::Sun).to_string().as_str()], |r| {
            r.center().y < BAR_HEIGHT
        })
        .expect("the sun in the light look");
        let (acts, _) = run(&mut home, &mut shared, (1160.0, 740.0), false, click(sun));
        assert!(
            acts.contains(&MainAction::SetMode(UiMode::Dark)),
            "{acts:?}"
        );
    }
}
