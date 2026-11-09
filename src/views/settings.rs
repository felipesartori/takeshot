//! Ajustes: the sections on the left and their grouped rows on the right, as in "Ajustes ·
//! escuro" of the design. Values bind to the settings; dialogs and system changes go through
//! `MainAction`.

use crate::Shared;
use crate::screens::about::AboutInfo;
use crate::screens::home::{Home, MainAction, SettingsSection, chip, hotkey_text};
use crate::system::hotkeys::{Shot, Status, default_keys, label};
use crate::system::settings::{OnClose, TrayClick};
use crate::ui::i18n::{LangPref, Msg, t as tr, tf};
use crate::ui::kit::{self, BtnKind, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::{ThemeId, Tokens, UiMode};
use egui::{
    Align, Align2, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, Ui, UiBuilder, pos2, vec2,
};

const SECTIONS: [(SettingsSection, Ico, Msg); 5] = [
    (SettingsSection::General, Ico::Sliders, Msg::GeneralSection),
    (SettingsSection::Capture, Ico::Scan, Msg::CaptureSection),
    (SettingsSection::Save, Ico::HardDrive, Msg::SaveSection),
    (
        SettingsSection::Shortcuts,
        Ico::Keyboard,
        Msg::ShortcutsSection,
    ),
    (SettingsSection::About, Ico::Info, Msg::AboutSection),
];

/// A drop-down's width in a row, as in the design.
const SELECT_WIDTH: f32 = 220.0;

pub fn settings_page(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    let before = serde_json::to_string(&shared.settings).ok();
    // Measures from "Ajustes · escuro": a 220 px list flush to the box (its 196 px items 12 px
    // in), a line from top to bottom, and the content 40 px in on both sides, 28 from the top.
    let inner = ui.max_rect();
    let b = inner.expand(1.0); // the box, its border included
    let list = Rect::from_min_max(
        pos2(b.left() + 12.0, b.top() + 12.0),
        pos2(b.left() + 208.0, inner.bottom()),
    );
    let line = Rect::from_min_max(
        pos2(b.left() + 220.0, inner.top()),
        pos2(b.left() + 221.0, inner.bottom()),
    );
    let content = Rect::from_min_max(pos2(line.right(), inner.top()), inner.max);
    ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for (s, icon, msg) in SECTIONS {
            if kit::section_item(ui, t, icon, tr(msg), home.section() == s).clicked() {
                acts.push(MainAction::Section(s));
            }
        }
    });
    ui.painter().rect_filled(line, 0.0, t.border);
    ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
        egui::ScrollArea::vertical()
            .id_salt("takeui-settings")
            .auto_shrink([false, false])
            // Clear of the box's 12 px corners.
            .scroll_bar_rect(content.shrink2(vec2(0.0, 12.0)))
            .show(ui, |ui| {
                // The scroll bar sits inside the right padding, so nothing moves when it shows.
                Frame::new()
                    .inner_margin(Margin {
                        left: 40,
                        right: 0,
                        top: 28,
                        bottom: 28,
                    })
                    .show(ui, |ui| {
                        ui.set_width((content.width() - 80.0).max(0.0));
                        // 4 px between a title and its hint; `group` and `heading` count on it.
                        ui.spacing_mut().item_spacing.y = 4.0;
                        match home.section() {
                            SettingsSection::General => general(ui, t, home, shared, acts),
                            SettingsSection::Capture => capture(ui, t, shared),
                            SettingsSection::Save => save(ui, t, shared, acts),
                            SettingsSection::Shortcuts => shortcuts(ui, t, home, shared, acts),
                            SettingsSection::About => about(ui, t, home, acts),
                        }
                    });
            });
    });
    if serde_json::to_string(&shared.settings).ok() != before {
        shared.settings_dirty = true;
    }
}

/// The section's title (22) and its short hint (13), the row spacing apart.
fn heading(ui: &mut Ui, t: &Tokens, title: &str, hint: Option<&str>) {
    ui.label(
        RichText::new(title)
            .font(font(22.0, Weight::Bold))
            .color(t.text),
    );
    if let Some(h) = hint {
        ui.label(
            RichText::new(h)
                .font(font(13.0, Weight::Regular))
                .color(t.muted),
        );
    }
}

/// A titled group of rows in a bordered box, 24 px under what comes before; rows are separated
/// by lines.
fn group(ui: &mut Ui, t: &Tokens, title: &str, rows: impl FnOnce(&mut Ui)) {
    // `add_space` comes on top of the spacing after the previous widget.
    ui.add_space(24.0 - ui.spacing().item_spacing.y);
    ui.label(
        RichText::new(title)
            .font(font(13.0, Weight::Medium))
            .color(t.muted),
    );
    ui.add_space(10.0 - ui.spacing().item_spacing.y);
    Frame::new()
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(10)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_width(ui.available_width());
            rows(ui);
        });
}

fn divider(ui: &mut Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, t.line);
}

/// The two theme cards with the same room above and below, the label centred beside them.
fn theme_row(ui: &mut Ui, t: &Tokens, acts: &mut Vec<MainAction>) {
    const PAD: f32 = 12.0;
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), 112.0 + 2.0 * PAD),
        Sense::hover(),
    );
    ui.painter().text(
        pos2(rect.left() + 20.0, rect.center().y),
        Align2::LEFT_CENTER,
        tr(Msg::Theme),
        font(14.0, Weight::Medium),
        t.text,
    );
    // Centred in the full height, so the scope ends where the row does.
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect.shrink2(vec2(20.0, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            for th in [ThemeId::Coral, ThemeId::TakeUi] {
                // The theme in use (a run override included) is the one marked.
                if kit::theme_card(ui, t, th, t.theme == th).clicked() {
                    acts.push(MainAction::SetTheme(th));
                }
            }
        },
    );
}

fn switch_row(ui: &mut Ui, t: &Tokens, label: &str, hint: Option<&str>, on: &mut bool) {
    kit::setting_row(ui, t, label, hint, |ui| {
        kit::switch(ui, t, on);
    });
}

fn general(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    heading(
        ui,
        t,
        tr(Msg::GeneralSection),
        Some(tr(Msg::SettingsGeneralHint)),
    );
    group(ui, t, tr(Msg::Startup), |ui| {
        let mut on = home.autostart();
        switch_row(
            ui,
            t,
            tr(Msg::StartWithSystem),
            Some(tr(Msg::StartWithSystemHint)),
            &mut on,
        );
        if on != home.autostart() {
            acts.push(MainAction::SetAutostart(on));
        }
    });
    group(ui, t, tr(Msg::Appearance), |ui| {
        theme_row(ui, t, acts);
        divider(ui, t);
        kit::setting_row(ui, t, tr(Msg::Mode), Some(tr(Msg::ModeHint)), |ui| {
            kit::segmented(
                ui,
                t,
                &mut shared.settings.ui_mode,
                &[
                    (UiMode::Light, tr(Msg::ModeLight)),
                    (UiMode::Dark, tr(Msg::ModeDark)),
                    (UiMode::System, tr(Msg::ModeSystem)),
                ],
            );
        });
        divider(ui, t);
        let mut lang = shared.settings.lang;
        kit::setting_row(ui, t, tr(Msg::Language), None, |ui| {
            kit::segmented(
                ui,
                t,
                &mut lang,
                &[
                    (LangPref::Auto, tr(Msg::LangAuto)),
                    (LangPref::En, tr(Msg::LangEn)),
                    (LangPref::Pt, tr(Msg::LangPt)),
                ],
            );
        });
        if lang != shared.settings.lang {
            acts.push(MainAction::SetLanguage(lang));
        }
    });
    group(ui, t, tr(Msg::WindowAndTray), |ui| {
        let s = &mut shared.settings;
        kit::setting_row(
            ui,
            t,
            tr(Msg::OnCloseLabel),
            Some(tr(Msg::OnCloseHint)),
            |ui| {
                kit::select(
                    ui,
                    t,
                    "takeui-on-close",
                    &mut s.on_close,
                    &[
                        (OnClose::Ask, tr(Msg::AskOnClose)),
                        (OnClose::Minimize, tr(Msg::MinimizeToTray)),
                        (OnClose::Quit, tr(Msg::Quit)),
                    ],
                    SELECT_WIDTH,
                );
            },
        );
        divider(ui, t);
        kit::setting_row(ui, t, tr(Msg::TrayClickLabel), None, |ui| {
            kit::select(
                ui,
                t,
                "takeui-tray-click",
                &mut s.tray_click,
                &[
                    (TrayClick::OpenHome, tr(Msg::OpenHome)),
                    (TrayClick::CaptureArea, tr(Msg::CaptureArea)),
                ],
                SELECT_WIDTH,
            );
        });
        divider(ui, t);
        switch_row(
            ui,
            t,
            tr(Msg::ShowNotice),
            Some(tr(Msg::NoticeHint)),
            &mut s.show_bubbles,
        );
        if cfg!(target_os = "linux") {
            divider(ui, t);
            kit::setting_row(ui, t, tr(Msg::QuitTakeshot), None, |ui| {
                if kit::button(
                    ui,
                    t,
                    BtnKind::Secondary,
                    Some(Ico::Power),
                    tr(Msg::Quit),
                    None,
                )
                .clicked()
                {
                    acts.push(MainAction::Quit);
                }
            });
        }
    });
}

fn capture(ui: &mut Ui, t: &Tokens, shared: &mut Shared) {
    heading(ui, t, tr(Msg::CaptureSection), None);
    group(ui, t, tr(Msg::CaptureSection), |ui| {
        let s = &mut shared.settings;
        switch_row(ui, t, tr(Msg::KeepLastArea), None, &mut s.keep_selection);
        divider(ui, t);
        switch_row(
            ui,
            t,
            tr(Msg::FullOpensEditor),
            None,
            &mut s.open_editor_after_full,
        );
        divider(ui, t);
        switch_row(
            ui,
            t,
            tr(Msg::HideOnCapture),
            Some(tr(Msg::HideOnCaptureHint)),
            &mut s.hide_on_capture,
        );
        // Wayland gives a window no position, so the capture stays on one monitor there.
        if !crate::system::resident::on_wayland() {
            divider(ui, t);
            switch_row(
                ui,
                t,
                tr(Msg::AllScreens),
                Some(tr(Msg::AllScreensHint)),
                &mut s.capture_all_screens,
            );
        }
    });
}

fn save(ui: &mut Ui, t: &Tokens, shared: &mut Shared, acts: &mut Vec<MainAction>) {
    heading(ui, t, tr(Msg::SaveSection), None);
    let dir = shared.captures_dir().to_string();
    group(ui, t, tr(Msg::SaveSection), |ui| {
        kit::setting_row(ui, t, tr(Msg::CapturesFolder), Some(&dir), |ui| {
            if kit::button(
                ui,
                t,
                BtnKind::Secondary,
                Some(Ico::FolderOpen),
                tr(Msg::Open),
                None,
            )
            .clicked()
            {
                acts.push(MainAction::OpenFolder);
            }
            if kit::button(ui, t, BtnKind::Secondary, None, tr(Msg::Choose), None).clicked() {
                acts.push(MainAction::ChooseFolder);
            }
        });
        divider(ui, t);
        switch_row(
            ui,
            t,
            tr(Msg::SaveStraight),
            Some(tr(Msg::AlwaysPng)),
            &mut shared.settings.auto_save,
        );
    });
}

fn shortcuts(
    ui: &mut Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
    acts: &mut Vec<MainAction>,
) {
    heading(ui, t, tr(Msg::ShortcutsSection), None);
    let status = hotkey_text(home.hotkey_status.as_ref(), &shared.settings);
    group(ui, t, tr(Msg::GlobalShortcuts), |ui| {
        switch_row(
            ui,
            t,
            tr(Msg::GlobalShortcuts),
            None,
            &mut shared.settings.hotkeys,
        );
        for (shot, msg) in [
            (Shot::Area, Msg::CaptureArea),
            (Shot::Full, Msg::FullScreen),
        ] {
            divider(ui, t);
            let keys = match shot {
                Shot::Area => shared.settings.hotkey_area.clone(),
                Shot::Full => shared.settings.hotkey_full.clone(),
            };
            kit::setting_row(ui, t, tr(msg), None, |ui| {
                if home.capturing() == Some(shot) {
                    ui.label(
                        RichText::new(tr(Msg::PressKeys))
                            .font(font(12.0, Weight::Medium))
                            .color(t.accent),
                    );
                    return;
                }
                if keys != default_keys(shot)
                    && kit::button(ui, t, BtnKind::Ghost, None, tr(Msg::RestoreDefault), None)
                        .clicked()
                {
                    acts.push(MainAction::RestoreKeys(shot));
                }
                if kit::button(ui, t, BtnKind::Secondary, None, tr(Msg::ChangeKeys), None).clicked()
                {
                    acts.push(MainAction::StartRebind(shot));
                }
                kit::kbd(ui, t, &label(&keys), false);
            });
        }
        divider(ui, t);
        let text = match &status {
            Ok(s) | Err(s) => s.clone(),
        };
        let (state, chip_text) = chip(home.hotkey_status.as_ref(), &shared.settings);
        kit::setting_row(ui, t, tr(Msg::ShortcutStatus), Some(&text), |ui| {
            kit::status_chip(ui, t, state, &chip_text);
        });
        let prtsc_busy = matches!(&home.hotkey_status, Some(Status::Busy { keys }) if keys.iter().any(|k| k.contains("PrtSc")));
        if cfg!(windows) {
            if prtsc_busy {
                divider(ui, t);
                kit::setting_row(
                    ui,
                    t,
                    tr(Msg::PrtScBusy),
                    Some(tr(Msg::PrtScConflictHelp)),
                    |ui| {
                        if kit::button(
                            ui,
                            t,
                            BtnKind::Secondary,
                            None,
                            tr(Msg::OpenWindowsSettings),
                            None,
                        )
                        .clicked()
                        {
                            acts.push(MainAction::OpenWindowsSettings);
                        }
                    },
                );
            }
            divider(ui, t);
            switch_row(
                ui,
                t,
                tr(Msg::PrtScPriority),
                Some(tr(Msg::PrtScPriorityHint)),
                &mut shared.settings.prtsc_priority,
            );
        }
    });
    if cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        let exe = crate::system::autostart::program()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "takeshot".into());
        ui.add_space(12.0);
        for args in ["--capture", "--capture --full"] {
            ui.label(
                RichText::new(tf(Msg::WaylandHotkeyHelp, &[&format!("{exe} {args}")]))
                    .font(font(12.0, Weight::Regular))
                    .color(t.muted),
            );
        }
    }
    group(ui, t, tr(Msg::InCapture), |ui| {
        let rows = [
            (tr(Msg::KeyShiftDrag), Msg::DescStraight),
            (tr(Msg::KeyAltDrag), Msg::DescFromCenter),
            ("P L A R O M T B N", Msg::DescTools),
            (tr(Msg::KeyWheel), Msg::DescWidth),
            ("Ctrl+C / Ctrl+S", Msg::DescCopySave),
            ("Enter", Msg::DescOpenEditor),
            ("Ctrl+Z / Ctrl+Y", Msg::DescUndoRedo),
            (tr(Msg::KeyArrows), Msg::DescMoveResize),
            (tr(Msg::KeyPickColor), Msg::DescPickColor),
        ];
        for (i, (keys, desc)) in rows.into_iter().enumerate() {
            if i > 0 {
                divider(ui, t);
            }
            kit::setting_row(ui, t, tr(desc), None, |ui| {
                kit::kbd(ui, t, keys, false);
            });
        }
    });
}

fn about(ui: &mut Ui, t: &Tokens, home: &mut Home, acts: &mut Vec<MainAction>) {
    let info = AboutInfo::current();
    heading(ui, t, tr(Msg::About), None);
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        let mark = home.mark(ui.ctx(), t);
        ui.add(egui::Image::new((mark, vec2(56.0, 56.0))));
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.label(crate::ui::brand::wordmark(t.theme, 22.0).color(t.text));
            ui.label(
                RichText::new(format!(
                    "{} · {} {}",
                    tf(Msg::VersionN, &[info.version]),
                    info.os,
                    info.arch
                ))
                .font(font(12.0, Weight::Regular))
                .color(t.faint),
            );
        });
    });
    ui.add_space(14.0);
    ui.label(
        RichText::new(tr(Msg::AboutPhrase))
            .font(font(13.0, Weight::Regular))
            .color(t.muted),
    );
    ui.add_space(10.0);
    ui.label(
        RichText::new(format!("{}  ·  {}", tr(Msg::MadeBy), tr(Msg::LicenseAgpl)))
            .font(font(12.0, Weight::Medium))
            .color(t.muted),
    );
    ui.add_space(14.0);
    if kit::button(
        ui,
        t,
        BtnKind::Secondary,
        Some(Ico::ExternalLink),
        tr(Msg::ViewOnGitHub),
        None,
    )
    .clicked()
    {
        acts.push(MainAction::ViewOnGitHub);
    }
}

#[cfg(test)]
mod tests {
    use crate::screens::home::{Home, MainAction, SettingsSection};
    use crate::system::instance::Page;
    use crate::ui::i18n::{Lang, LangPref, Msg, apply_language, every, lang, set_lang};
    use crate::ui::tokens::{ThemeId, UiMode};
    use crate::views::testing::{click, content_box, painted_at, run, run_painted};
    use egui::vec2;

    #[test]
    fn the_theme_cards_sit_in_the_middle_of_their_row() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        // A card's ring is drawn 1 px inside the card.
        let card = blocks
            .iter()
            .map(|(r, _)| r.expand(1.0))
            .find(|r| r.size() == vec2(148.0, 112.0))
            .expect("a theme card");
        let group = blocks
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| r.contains_rect(card) && r.width() > 3.0 * card.width())
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("the Appearance group");
        let line = blocks
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| r.height() == 1.0 && r.width() > 100.0 && r.top() >= card.bottom())
            .min_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("the line under the row");
        let above = card.top() - group.top();
        let below = line.top() - card.bottom();
        assert!((above - below).abs() <= 1.0, "above {above}, below {below}");
        let label = painted_at(&painted, &every(Msg::Theme), |r| r.right() < card.left())
            .expect("the Theme label");
        assert!(
            (label.y - card.center().y).abs() <= 1.0,
            "{label:?} beside {card:?}"
        );
    }

    #[test]
    fn the_about_section_shows_the_theme_s_wordmark() {
        use crate::ui::tokens::ThemeId;
        for (theme, word) in [(ThemeId::TakeUi, "Takeshot"), (ThemeId::Coral, "takeshot")] {
            let (mut home, mut shared) = Home::for_tests(vec![]);
            shared.settings.ui_theme = theme;
            home.open(Page::Settings);
            home.apply(
                MainAction::Section(SettingsSection::About),
                &egui::Context::default(),
                &mut shared,
            );
            let (_, painted) =
                crate::views::testing::run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
            let bar = crate::ui::chrome::BAR_HEIGHT;
            assert!(
                painted.iter().any(|(s, r)| s == word && r.top() > bar),
                "{theme:?}: {word}"
            );
        }
    }

    #[test]
    fn every_section_fits_at_the_minimum_size() {
        // Nothing past the box's padding, and no text running into another (a hint under its
        // row's control, a label under a chip).
        let before = lang();
        for l in [Lang::Pt, Lang::En] {
            set_lang(l);
            for s in [
                SettingsSection::General,
                SettingsSection::Capture,
                SettingsSection::Save,
                SettingsSection::Shortcuts,
                SettingsSection::About,
            ] {
                let (mut home, mut shared) = Home::for_tests(vec![]);
                home.open(Page::Settings);
                home.apply(
                    MainAction::Section(s),
                    &egui::Context::default(),
                    &mut shared,
                );
                let (_, painted, blocks) =
                    run_painted(&mut home, &mut shared, (900.0, 600.0), true, vec![vec![]]);
                let b = content_box(&blocks, &shared.tokens(None));
                // A row too wide for the room would push the box past the window's 8 px margin.
                assert!(
                    (b.right() - (900.0 - 8.0)).abs() <= 0.5,
                    "{l:?} {s:?}: {b:?}"
                );
                let texts: Vec<_> = painted.iter().filter(|(_, r)| r.top() > b.top()).collect();
                for (i, (text, r)) in texts.iter().enumerate() {
                    // The content's 40 px right padding, which holds the scroll bar.
                    assert!(
                        r.right() <= b.right() - 40.0 + 1.0,
                        "{l:?} {s:?}: {text:?} {r:?} outside {b:?}"
                    );
                    for (other, o) in &texts[i + 1..] {
                        assert!(
                            !r.shrink(0.5).intersects(o.shrink(0.5)),
                            "{l:?} {s:?}: {text:?} {r:?} runs into {other:?} {o:?}"
                        );
                    }
                }
            }
        }
        set_lang(before);
    }

    #[test]
    fn groups_sit_24_px_apart() {
        // "Ajustes · escuro": 24 px from a group's box (or the heading) to the next title.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let title = |m: Msg| {
            painted
                .iter()
                .find(|(s, _)| every(m).contains(&s.as_str()))
                .map(|(_, r)| *r)
                .expect("a group title")
        };
        let hint = title(Msg::SettingsGeneralHint);
        let startup = title(Msg::Startup);
        let appearance = title(Msg::Appearance);
        // The Startup group's box: the bordered frame right under its title.
        let startup_box = blocks
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| {
                r.top() > startup.bottom() && r.bottom() < appearance.top() && r.width() > 300.0
            })
            .max_by(|a, b| a.height().total_cmp(&b.height()))
            .expect("the Startup box");
        let gap = startup.top() - hint.bottom();
        assert!((gap - 24.0).abs() <= 1.0, "heading to Startup: {gap}");
        let gap = appearance.top() - startup_box.bottom();
        assert!(
            (gap - 24.0).abs() <= 1.0,
            "Startup box to Appearance: {gap}"
        );
    }

    #[test]
    fn the_scroll_bar_keeps_off_the_rounded_corners() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, _, blocks) =
            run_painted(&mut home, &mut shared, (900.0, 600.0), true, vec![vec![]]);
        let b = content_box(&blocks, &shared.tokens(None));
        let bars: Vec<_> = blocks
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| r.width() <= 6.5 && r.height() > 40.0 && r.right() > b.right() - 8.0)
            .collect();
        assert!(!bars.is_empty(), "the bar");
        for r in bars {
            assert!(
                r.top() >= b.top() + 12.0 - 0.5 && r.bottom() <= b.bottom() - 12.0 + 0.5,
                "{r:?} in {b:?}"
            );
        }
    }

    #[test]
    fn the_theme_cards_keep_their_spacing() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, _, blocks) = run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let mut cards: Vec<_> = blocks
            .iter()
            .map(|(r, _)| r.expand(1.0))
            .filter(|r| r.size() == vec2(148.0, 112.0))
            .collect();
        cards.sort_by(|a, b| a.left().total_cmp(&b.left()));
        assert_eq!(cards.len(), 2, "{cards:?}");
        let gap = cards[1].left() - cards[0].right();
        assert!(gap >= 8.0, "gap {gap}");
    }

    #[test]
    fn the_section_list_matches_the_design() {
        // "Ajustes · escuro": a 220 px list flush to the box, 196 × 36 items 12 px in from both
        // sides, a line from the box's top to its bottom, the content 40 px past it.
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, painted, blocks) =
            run_painted(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let t = shared.tokens(None);
        let b = content_box(&blocks, &t);
        let line = blocks
            .iter()
            .map(|(r, _)| *r)
            .find(|r| r.width() == 1.0 && r.height() > 300.0)
            .expect("the line beside the list");
        let open = blocks
            .iter()
            .find(|(r, c)| *c == t.tint && r.right() <= line.left())
            .map(|(r, _)| *r)
            .expect("the open section");
        assert_eq!(open.size(), vec2(196.0, 36.0), "{open:?}");
        assert_eq!(open.left() - b.left(), 12.0, "{open:?} in {b:?}");
        assert_eq!(line.left() - open.right(), 12.0, "{open:?} beside {line:?}");
        assert!(
            (line.top() - b.top()).abs() <= 1.0 && (line.bottom() - b.bottom()).abs() <= 1.0,
            "{line:?} spans {b:?}"
        );
        let heading = painted
            .iter()
            .find(|(s, r)| {
                every(Msg::GeneralSection).contains(&s.as_str()) && r.left() > line.left()
            })
            .map(|(_, r)| *r)
            .expect("the General heading");
        assert!(
            (heading.left() - line.right() - 40.0).abs() <= 1.0,
            "{heading:?} after {line:?}"
        );
    }

    #[test]
    fn choosing_coral_switches_the_look_at_once() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let coral =
            painted_at(&painted, &every(Msg::ThemeCoral), |_| true).expect("the Coral card");
        let (acts, _) = run(&mut home, &mut shared, (1160.0, 740.0), true, click(coral));
        assert!(
            acts.contains(&MainAction::SetTheme(ThemeId::Coral)),
            "{acts:?}"
        );
        for a in acts {
            home.apply(a, &egui::Context::default(), &mut shared);
        }
        assert_eq!(shared.settings.ui_theme, ThemeId::Coral);
        assert_eq!(
            shared.tokens(None).theme,
            ThemeId::Coral,
            "the next frame draws Coral"
        );
        assert!(shared.settings_dirty);
    }

    #[test]
    fn mode_and_language_apply_at_once() {
        let before = lang();
        set_lang(Lang::Pt);
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let light = painted_at(&painted, &every(Msg::ModeLight), |_| true).expect("Light");
        run(&mut home, &mut shared, (1160.0, 740.0), true, click(light));
        assert_eq!(shared.settings.ui_mode, UiMode::Light);
        // The language goes from the click through the action to the strings on screen.
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let english = painted_at(&painted, &every(Msg::LangEn), |_| true).expect("English");
        let (acts, _) = run(
            &mut home,
            &mut shared,
            (1160.0, 740.0),
            true,
            click(english),
        );
        assert!(
            acts.contains(&MainAction::SetLanguage(LangPref::En)),
            "{acts:?}"
        );
        for a in acts {
            home.apply(a, &egui::Context::default(), &mut shared);
        }
        assert_eq!(shared.settings.lang, LangPref::En);
        assert_eq!(lang(), Lang::En);
        apply_language(LangPref::Pt);
        assert_eq!(lang(), Lang::Pt);
        set_lang(before);
    }

    #[test]
    fn the_subnav_switches_section() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        home.open(Page::Settings);
        let (_, painted) = run(&mut home, &mut shared, (1160.0, 740.0), true, vec![]);
        let keys =
            painted_at(&painted, &every(Msg::ShortcutsSection), |_| true).expect("Shortcuts");
        let (acts, _) = run(&mut home, &mut shared, (1160.0, 740.0), true, click(keys));
        assert!(
            acts.contains(&MainAction::Section(SettingsSection::Shortcuts)),
            "{acts:?}"
        );
        for a in acts {
            home.apply(a, &egui::Context::default(), &mut shared);
        }
        assert_eq!(home.section(), SettingsSection::Shortcuts);
    }
}
