//! TakeUI's Início: the capture buttons, the gallery filters, search and folder, and the grid of
//! captures from the save folder, as in "Início · escuro" and "Início · claro" of the design.

use crate::Shared;
use crate::screens::home::{Home, MainAction};
use crate::system::hotkeys::{Shot, label};
use crate::system::library::Period;
use crate::ui::i18n::{Msg, t as tr, tf};
use crate::ui::kit::{self, BtnKind, CARD_TEXT_H, CardAction, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::Tokens;
use egui::{Align, Layout, RichText, Ui, vec2};

/// The narrowest card and the gap between cards.
const CARD_MIN: f32 = 168.0;
const GAP: f32 = 12.0;

/// Columns and card size for a grid `width` wide: pictures in 16:10 over the two text lines.
pub fn grid(width: f32) -> (usize, egui::Vec2) {
    let cols = (((width + GAP) / (CARD_MIN + GAP)).floor() as usize).max(1);
    let w = (width - GAP * (cols - 1) as f32) / cols as f32;
    (cols, vec2(w, (w - 12.0) * 10.0 / 16.0 + 6.0 + CARD_TEXT_H))
}

pub fn start_page(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    let area = label(&shared.settings.hotkey_area);
    let full = label(&shared.settings.hotkey_full);
    header(ui, t, &area, &full, acts);
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        let [all, today, week] = home.counts();
        for (p, msg, n) in [
            (Period::All, Msg::FilterAll, all),
            (Period::Today, Msg::Today, today),
            (Period::Week, Msg::ThisWeek, week),
        ] {
            if kit::filter_pill(ui, t, tr(msg), n, home.filter() == p).clicked() {
                acts.push(MainAction::Filter(p));
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if kit::button(
                ui,
                t,
                BtnKind::Secondary,
                Some(Ico::FolderOpen),
                tr(Msg::OpenFolder),
                None,
            )
            .clicked()
            {
                acts.push(MainAction::OpenFolder);
            }
            let mut q = home.query().to_string();
            if kit::text_field(
                ui,
                t,
                &mut q,
                tr(Msg::SearchByName),
                Some(Ico::Search),
                220.0,
            )
            .changed()
            {
                acts.push(MainAction::Search(q));
            }
        });
    });
    ui.add_space(14.0);
    if home.folder_missing() {
        empty(
            ui,
            t,
            Ico::FolderSearch,
            tr(Msg::FolderMissing),
            None,
            Some(acts),
        );
    } else if !home.has_captures() {
        let hint = tr(Msg::HomeSubtitle).replace("PrtSc", &area);
        empty(
            ui,
            t,
            Ico::Images,
            tr(Msg::NoCapturesYet),
            Some(&hint),
            None,
        );
    } else {
        gallery(ui, t, home, acts);
    }
}

/// Title and subtitle with the capture buttons on their right, or under them when the window is
/// too narrow for one row. The buttons' width is remembered from the frame before.
fn header(ui: &mut Ui, t: &Tokens, area: &str, full: &str, acts: &mut Vec<MainAction>) {
    let (title, sub) = (tr(Msg::Home), tr(Msg::HomeSubtitle));
    let title_w = {
        let p = ui.painter();
        let w = |s: &str, f| p.layout_no_wrap(s.to_string(), f, t.text).size().x;
        w(title, font(22.0, Weight::Bold)).max(w(sub, font(13.0, Weight::Regular)))
    };
    let id = egui::Id::unique("takeui-home-buttons");
    let buttons_w = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(720.0);
    let titles = |ui: &mut Ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(font(22.0, Weight::Bold))
                    .color(t.text),
            );
            ui.label(
                RichText::new(sub)
                    .font(font(13.0, Weight::Regular))
                    .color(t.muted),
            );
        });
    };
    let w = if ui.available_width() >= title_w + buttons_w + 24.0 {
        ui.horizontal(|ui| {
            titles(ui);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.scope(|ui| capture_buttons(ui, t, area, full, acts, true))
                    .response
                    .rect
                    .width()
            })
            .inner
        })
        .inner
    } else {
        titles(ui);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.scope(|ui| capture_buttons(ui, t, area, full, acts, false))
                .response
                .rect
                .width()
        })
        .inner
    };
    ui.ctx().data_mut(|d| d.insert_temp(id, w));
}

/// Full screen, in 3 seconds, open image and capture area (primary), left to right on screen;
/// `rtl` when the layout places them from the right.
fn capture_buttons(
    ui: &mut Ui,
    t: &Tokens,
    area: &str,
    full: &str,
    acts: &mut Vec<MainAction>,
    rtl: bool,
) {
    let mut items = [
        (
            BtnKind::Secondary,
            Ico::Monitor,
            Msg::FullScreen,
            Some(full),
            MainAction::Capture(Shot::Full),
        ),
        (
            BtnKind::Secondary,
            Ico::Timer,
            Msg::In3Seconds,
            None,
            MainAction::CaptureIn3s,
        ),
        (
            BtnKind::Secondary,
            Ico::ImagePlus,
            Msg::OpenImage,
            Some("Ctrl+O"),
            MainAction::OpenImage,
        ),
        (
            BtnKind::Primary,
            Ico::Scan,
            Msg::CaptureArea,
            Some(area),
            MainAction::Capture(Shot::Area),
        ),
    ];
    if rtl {
        items.reverse();
    }
    for (kind, icon, msg, keys, act) in items {
        if kit::button(ui, t, kind, Some(icon), tr(msg), keys).clicked() {
            acts.push(act);
        }
    }
}

/// The grid; only the rows on screen are laid out and ask for thumbnails.
fn gallery(ui: &mut Ui, t: &Tokens, home: &mut Home, acts: &mut Vec<MainAction>) {
    let (cols, card) = grid(ui.available_width() - 8.0);
    let shown = home.shown().to_vec();
    let rows = shown.len().div_ceil(cols);
    ui.spacing_mut().item_spacing.y = GAP;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, card.y, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for &i in shown.iter().skip(row * cols).take(cols) {
                        let e = home.entry(i).clone();
                        let (thumb, dims) = home.card_thumb(&e.path);
                        let size = human_size(e.bytes);
                        let meta = match dims {
                            Some([w, h]) => {
                                tf(Msg::CaptureMeta, &[&w.to_string(), &h.to_string(), &size])
                            }
                            None => size,
                        };
                        let selected = home.selected() == Some(e.path.as_path());
                        let r = kit::capture_card(ui, t, card, thumb, &e.name, &meta, selected);
                        if let Some(a) = r.action {
                            acts.push(match a {
                                CardAction::Copy => MainAction::Copy(e.path.clone()),
                                CardAction::Edit => MainAction::Edit(e.path.clone()),
                                CardAction::Folder => MainAction::ShowInFolder(e.path.clone()),
                                CardAction::Trash => MainAction::Trash(e.path.clone()),
                            });
                        } else if r.resp.double_clicked() {
                            acts.push(MainAction::Edit(e.path.clone()));
                        } else if r.resp.clicked() {
                            acts.push(MainAction::Select(e.path.clone()));
                        }
                        let resp = r.resp.on_hover_text(details(&e, &meta));
                        resp.context_menu(|ui| {
                            for (icon, msg, act) in [
                                (Ico::Copy, Msg::Copy, MainAction::Copy(e.path.clone())),
                                (
                                    Ico::Edit,
                                    Msg::EditInEditor,
                                    MainAction::Edit(e.path.clone()),
                                ),
                                (
                                    Ico::FolderOpen,
                                    Msg::ShowInFolder,
                                    MainAction::ShowInFolder(e.path.clone()),
                                ),
                                (Ico::Pin, Msg::PinOnScreen, MainAction::Pin(e.path.clone())),
                                (Ico::Trash, Msg::Delete, MainAction::Trash(e.path.clone())),
                            ] {
                                if kit::menu_item(ui, t, Some(icon), tr(msg), None, false).clicked()
                                {
                                    acts.push(act);
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            }
        });
}

/// The card's tooltip: size, file size, folder and date.
fn details(e: &crate::system::library::Entry, meta: &str) -> String {
    let when = chrono::DateTime::<chrono::Local>::from(e.modified).format("%d/%m/%Y %H:%M");
    let folder = e
        .path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    format!("{}\n{meta}\n{folder}\n{when}", e.name)
}

/// 412 KB, 1.2 MB: decimal units, as file managers show them.
fn human_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{} KB", bytes.div_ceil(1000))
    }
}

/// An empty gallery: an icon, the reason, a hint, and "Choose folder" when the folder is missing.
fn empty(
    ui: &mut Ui,
    t: &Tokens,
    icon: Ico,
    title: &str,
    hint: Option<&str>,
    choose: Option<&mut Vec<MainAction>>,
) {
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(crate::ui::icon::glyph(icon).to_string())
                .font(crate::ui::icon::font(28.0))
                .color(t.faint),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(title)
                .font(font(15.0, Weight::Medium))
                .color(t.text),
        );
        if let Some(h) = hint {
            ui.label(
                RichText::new(h)
                    .font(font(13.0, Weight::Regular))
                    .color(t.muted),
            );
        }
        if let Some(acts) = choose {
            ui.add_space(10.0);
            if kit::button(
                ui,
                t,
                BtnKind::Secondary,
                Some(Ico::FolderOpen),
                tr(Msg::ChooseFolder),
                None,
            )
            .clicked()
            {
                acts.push(MainAction::ChooseFolder);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::screens::home::{Home, MainAction};
    use crate::skins::takeui::testing::{click, double_click, painted_at, run};
    use crate::system::library::{Entry, Period};
    use crate::ui::i18n::{Msg, every};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn entry(name: &str, modified: SystemTime) -> Entry {
        Entry {
            path: PathBuf::from(name),
            name: name.to_string(),
            modified,
            bytes: 412_000,
        }
    }

    #[test]
    fn a_double_click_opens_the_card_in_the_editor() {
        let now = SystemTime::now();
        let (mut home, mut shared) = Home::for_tests(vec![
            entry("a.png", now),
            entry("b.png", now - Duration::from_secs(60)),
        ]);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let b = painted_at(&painted, &["b.png"], |_| true).expect("b.png painted");
        let (acts, _) = run(
            &mut home,
            &mut shared,
            (1160.0, 740.0),
            true,
            double_click(b),
        );
        assert!(
            acts.contains(&MainAction::Edit(PathBuf::from("b.png"))),
            "{acts:?}"
        );
    }

    #[test]
    fn filters_narrow_the_grid_and_show_counts() {
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_577_836_800); // 2020-01-01
        let (mut home, mut shared) = Home::for_tests(vec![
            entry("today.png", SystemTime::now()),
            entry("old.png", old),
        ]);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        assert!(painted.iter().any(|(s, _)| s == "2"), "the All count");
        let today = painted_at(&painted, &every(Msg::Today), |r| r.top() > 40.0).unwrap();
        let (acts, _) = run(&mut home, &mut shared, (1160.0, 740.0), true, click(today));
        assert!(
            acts.contains(&MainAction::Filter(Period::Today)),
            "{acts:?}"
        );
        for a in acts {
            home.apply(a, &egui::Context::default(), &mut shared);
        }
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let names: Vec<&str> = painted.iter().map(|(s, _)| s.as_str()).collect();
        assert!(names.contains(&"today.png"));
        assert!(!names.contains(&"old.png"), "filtered out");
    }

    #[test]
    fn the_header_fits_at_the_minimum_size() {
        // The capture buttons move under the title when the window is too narrow for one row.
        let now = SystemTime::now();
        let (mut home, mut shared) = Home::for_tests(vec![entry("a.png", now)]);
        for size in [(900.0, 600.0), (1160.0, 740.0)] {
            let (_, painted) = run(&mut home, &mut shared, size, true, vec![]);
            let head: Vec<_> = painted
                .iter()
                .filter(|(_, r)| r.center().y > 40.0 && r.center().y < 190.0)
                .collect();
            for (i, (a, ra)) in head.iter().enumerate() {
                assert!(ra.right() <= size.0, "{a:?} {ra:?} outside at {size:?}");
                for (b, rb) in &head[i + 1..] {
                    assert!(
                        !ra.shrink(0.5).intersects(rb.shrink(0.5)),
                        "{a:?} {ra:?} overlaps {b:?} {rb:?} at {size:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_visible_rows_ask_for_thumbnails() {
        let now = SystemTime::now();
        let entries = (0..2000)
            .map(|i| entry(&format!("n{i}.png"), now - Duration::from_secs(i)))
            .collect();
        let (mut home, mut shared) = Home::for_tests(entries);
        run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let asked = home.thumb_requests();
        assert!(asked > 0, "the visible cards ask");
        assert!(
            asked <= 40,
            "{asked} thumbnails asked for a 1160 × 740 window"
        );
    }
}
