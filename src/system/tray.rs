//! The tray: its menu entries in spec order (shared by the app-drawn and the native menu), the icon and its events.

// The tray menus use it from plan 4, Task 5; until then only the tests do.
#![allow(dead_code)]

use crate::system::hotkeys::{Shot, label};
use crate::system::instance::Page;
use crate::ui::i18n::{Msg, t};
use crate::ui::tokens::UiMode;
use std::path::PathBuf;

/// What choosing a tray entry does.
#[derive(Clone, PartialEq, Debug)]
pub enum TrayAction {
    Capture(Shot),
    CaptureIn3s,
    Show(Page),
    /// Click on a recent capture copies it; Shift+click opens it in the editor.
    CopyRecent(PathBuf),
    EditRecent(PathBuf),
    OpenCapturesFolder,
    SetMode(UiMode),
    Quit,
}

/// One row of the tray menu.
#[derive(Clone, PartialEq, Debug)]
pub enum TrayEntry {
    Item {
        label: String,
        keys: Option<String>,
        action: TrayAction,
    },
    /// The "Recent captures" submenu: these files, then "Open captures folder".
    Recents(Vec<PathBuf>),
    /// Light, Dark and System, with the current one marked.
    Mode(UiMode),
    Separator,
}

fn item(m: Msg, keys: Option<String>, action: TrayAction) -> TrayEntry {
    TrayEntry::Item {
        label: t(m).to_string(),
        keys,
        action,
    }
}

/// The menu, in the order of spec 5.1: captures, then the app and its pages, then About and Quit.
pub fn tray_entries(
    recents: &[PathBuf],
    mode: UiMode,
    area_keys: &str,
    full_keys: &str,
) -> Vec<TrayEntry> {
    vec![
        item(
            Msg::CaptureArea,
            Some(label(area_keys)),
            TrayAction::Capture(Shot::Area),
        ),
        item(
            Msg::FullScreen,
            Some(label(full_keys)),
            TrayAction::Capture(Shot::Full),
        ),
        item(Msg::CaptureIn3s, None, TrayAction::CaptureIn3s),
        TrayEntry::Separator,
        item(Msg::OpenTakeshot, None, TrayAction::Show(Page::Start)),
        TrayEntry::Recents(recents.to_vec()),
        item(Msg::StyleAndWatermark, None, TrayAction::Show(Page::Style)),
        item(Msg::Settings, None, TrayAction::Show(Page::Settings)),
        TrayEntry::Mode(mode),
        TrayEntry::Separator,
        item(Msg::About, None, TrayAction::Show(Page::About)),
        item(Msg::Quit, None, TrayAction::Quit),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions(entries: &[TrayEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|e| match e {
                TrayEntry::Item { action, .. } => format!("{action:?}"),
                TrayEntry::Recents(r) => format!("Recents({})", r.len()),
                TrayEntry::Mode(m) => format!("Mode({m:?})"),
                TrayEntry::Separator => "-".into(),
            })
            .collect()
    }

    #[test]
    fn tray_entries_follow_the_spec_order() {
        let recents = vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        let e = tray_entries(&recents, UiMode::Dark, "PrintScreen", "shift+PrintScreen");
        assert_eq!(
            actions(&e),
            vec![
                format!("{:?}", TrayAction::Capture(Shot::Area)),
                format!("{:?}", TrayAction::Capture(Shot::Full)),
                format!("{:?}", TrayAction::CaptureIn3s),
                "-".into(),
                format!("{:?}", TrayAction::Show(Page::Start)),
                "Recents(2)".into(),
                format!("{:?}", TrayAction::Show(Page::Style)),
                format!("{:?}", TrayAction::Show(Page::Settings)),
                "Mode(Dark)".into(),
                "-".into(),
                format!("{:?}", TrayAction::Show(Page::About)),
                format!("{:?}", TrayAction::Quit),
            ]
        );
        // The capture items show their shortcuts the way the keyboard reads them.
        let keys: Vec<Option<String>> = e
            .iter()
            .take(3)
            .map(|x| match x {
                TrayEntry::Item { keys, .. } => keys.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(
            keys,
            vec![Some("PrtSc".into()), Some("Shift+PrtSc".into()), None]
        );
        // With no captures yet the submenu stays, holding only "Open captures folder".
        let empty = tray_entries(&[], UiMode::System, "PrintScreen", "shift+PrintScreen");
        assert!(empty.contains(&TrayEntry::Recents(Vec::new())));
    }
}
