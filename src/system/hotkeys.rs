//! Global capture shortcuts owned by the resident: registration, conflict status and rebinding.

use crate::system::settings::Settings;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shot {
    Area,
    Full,
}

/// Whether the capture shortcuts work.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    Ready,
    /// Another app holds these shortcuts (labels, e.g. "PrtSc").
    Busy {
        keys: Vec<String>,
    },
    /// The system refused global shortcuts (the error text from the OS).
    Unavailable(String),
    /// Turned off in Settings.
    Off,
}

/// The shortcut each capture starts with, in the form `HotKey::from_str` reads.
pub fn default_keys(s: Shot) -> &'static str {
    match (s, cfg!(target_os = "macos")) {
        (Shot::Area, false) => "PrintScreen",
        (Shot::Full, false) => "shift+PrintScreen",
        (Shot::Area, true) => "ctrl+shift+Digit2",
        (Shot::Full, true) => "ctrl+shift+Digit1",
    }
}

/// How a shortcut reads on the keyboard: `shift+PrintScreen` → `Shift+PrtSc`.
pub fn label(keys: &str) -> String {
    keys.split('+')
        .map(|t| {
            let t = t.trim();
            match t.to_ascii_lowercase().as_str() {
                "shift" => "Shift".to_string(),
                "ctrl" | "control" => "Ctrl".to_string(),
                "alt" | "option" => "Alt".to_string(),
                "meta" | "super" | "cmd" | "command" => if cfg!(target_os = "macos") {
                    "Cmd"
                } else {
                    "Win"
                }
                .to_string(),
                "printscreen" => "PrtSc".to_string(),
                k if k.starts_with("digit") && k.len() == 6 => k[5..].to_string(),
                k if k.starts_with("key") && k.len() == 4 => k[3..].to_ascii_uppercase(),
                _ => t.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// The status from what registered (`keys`, worked) and whether Windows' Snipping Tool owns the
/// plain PrtSc key, which steals it even when our registration succeeded.
pub fn status(registered: &[(&str, bool)], snipping_tool_owns_prtsc: bool) -> Status {
    let keys: Vec<String> = registered
        .iter()
        .filter(|(k, ok)| !ok || (snipping_tool_owns_prtsc && label(k) == "PrtSc"))
        .map(|(k, _)| label(k))
        .collect();
    if keys.is_empty() {
        Status::Ready
    } else {
        Status::Busy { keys }
    }
}

/// The status to show: with the PrtSc hook on, PrtSc and Shift+PrtSc reach Takeshot first, so their
/// conflict no longer counts.
pub fn effective_status(status: &Status, hook_on: bool) -> Status {
    match status {
        Status::Busy { keys } if hook_on => {
            let rest: Vec<String> = keys
                .iter()
                .filter(|k| *k != "PrtSc" && *k != "Shift+PrtSc")
                .cloned()
                .collect();
            if rest.is_empty() {
                Status::Ready
            } else {
                Status::Busy { keys: rest }
            }
        }
        other => other.clone(),
    }
}

/// The shortcut text for a key pressed in egui with its modifiers, as `HotKey::from_str` reads
/// it; Esc (which cancels) gives none. egui has no PrtSc key, so restoring the default brings it back.
pub fn keys_from_egui(key: egui::Key, mods: egui::Modifiers) -> Option<String> {
    use egui::Key;
    if key == Key::Escape {
        return None;
    }
    let name = key.name();
    let code = match key {
        Key::Num0
        | Key::Num1
        | Key::Num2
        | Key::Num3
        | Key::Num4
        | Key::Num5
        | Key::Num6
        | Key::Num7
        | Key::Num8
        | Key::Num9 => format!("Digit{name}"),
        _ if name.len() == 1 && name.chars().all(|c| c.is_ascii_alphabetic()) => {
            format!("Key{}", name.to_ascii_uppercase())
        }
        _ => name.to_string(),
    };
    let mut parts = Vec::new();
    if mods.ctrl || (mods.command && !cfg!(target_os = "macos")) {
        parts.push("ctrl");
    }
    if mods.shift {
        parts.push("shift");
    }
    if mods.alt {
        parts.push("alt");
    }
    if mods.mac_cmd {
        parts.push("meta");
    }
    parts.push(&code);
    Some(parts.join("+"))
}

/// The PrtSc priority offer appears once, on a conflict, and never with the hook already on.
/// Only a PrtSc conflict qualifies: the hook does not help a rebound shortcut such as F9.
pub fn should_offer_priority(status: &Status, offered_before: bool, hook_on: bool) -> bool {
    let prtsc = matches!(status, Status::Busy { keys } if keys.iter().any(|k| k == "PrtSc" || k == "Shift+PrtSc"));
    prtsc && !offered_before && !hook_on
}

/// Windows' "Use the Print Screen key to open screen capture" setting.
#[cfg(windows)]
pub fn snipping_tool_owns_prtsc() -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Control Panel\\Keyboard")
        .and_then(|k| k.get_value::<u32, _>("PrintScreenKeyForSnippingEnabled"))
        .is_ok_and(|v| v == 1)
}

#[cfg(not(windows))]
pub fn snipping_tool_owns_prtsc() -> bool {
    false
}

/// The registered shortcuts. Dropping it unregisters them.
pub struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    keys: [String; 2],
    registered: [Option<HotKey>; 2],
    /// Ids the event handler maps to Area and Full; they change when a shortcut is rebound.
    ids: Arc<[AtomicU32; 2]>,
    status: Status,
}

fn slot(s: Shot) -> usize {
    match s {
        Shot::Area => 0,
        Shot::Full => 1,
    }
}

impl Hotkeys {
    /// Registers the shortcuts from `settings` and calls `on_shot` on the hotkey thread when one fires.
    pub fn start(settings: &Settings, on_shot: impl Fn(Shot) + Send + Sync + 'static) -> Self {
        let mut h = Self {
            manager: None,
            keys: [settings.hotkey_area.clone(), settings.hotkey_full.clone()],
            registered: [None, None],
            ids: Arc::new([AtomicU32::new(0), AtomicU32::new(0)]),
            status: Status::Off,
        };
        if !settings.hotkeys {
            return h;
        }
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                h.status = Status::Unavailable(e.to_string());
                return h;
            }
        };
        let ids = h.ids.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if e.state != HotKeyState::Pressed {
                return;
            }
            if e.id == ids[0].load(Ordering::Relaxed) {
                on_shot(Shot::Area);
            } else if e.id == ids[1].load(Ordering::Relaxed) {
                on_shot(Shot::Full);
            }
        }));
        h.manager = Some(manager);
        for s in [Shot::Area, Shot::Full] {
            let keys = h.keys[slot(s)].clone();
            let _ = h.register(s, &keys);
        }
        h.refresh_status();
        h
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Swaps the shortcut of a capture; on failure the old one stays registered.
    pub fn rebind(&mut self, s: Shot, keys: &str) -> Result<(), String> {
        HotKey::from_str(keys).map_err(|e| e.to_string())?;
        let old = self.keys[slot(s)].clone();
        if let (Some(m), Some(hk)) = (&self.manager, self.registered[slot(s)].take()) {
            let _ = m.unregister(hk);
        }
        let result = self.register(s, keys);
        if result.is_err() {
            let _ = self.register(s, &old);
        }
        self.refresh_status();
        result
    }

    fn register(&mut self, s: Shot, keys: &str) -> Result<(), String> {
        let hk = HotKey::from_str(keys).map_err(|e| e.to_string())?;
        let m = self.manager.as_ref().ok_or("global shortcuts are off")?;
        m.register(hk).map_err(|e| e.to_string())?;
        self.ids[slot(s)].store(hk.id(), Ordering::Relaxed);
        self.registered[slot(s)] = Some(hk);
        self.keys[slot(s)] = keys.to_string();
        Ok(())
    }

    fn refresh_status(&mut self) {
        if self.manager.is_none() {
            return;
        }
        let reg = [
            (self.keys[0].as_str(), self.registered[0].is_some()),
            (self.keys[1].as_str(), self.registered[1].is_some()),
        ];
        self.status = status(&reg, snipping_tool_owns_prtsc());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use global_hotkey::hotkey::HotKey;
    use std::str::FromStr;

    #[test]
    fn labels_read_like_the_keyboard() {
        assert_eq!(label("PrintScreen"), "PrtSc");
        assert_eq!(label("shift+PrintScreen"), "Shift+PrtSc");
        assert_eq!(label("ctrl+shift+Digit2"), "Ctrl+Shift+2");
        assert_eq!(label("control+alt+KeyA"), "Ctrl+Alt+A");
        assert_eq!(label("F9"), "F9");
    }

    #[test]
    fn default_keys_parse_as_hotkeys() {
        for s in [Shot::Area, Shot::Full] {
            assert!(HotKey::from_str(default_keys(s)).is_ok(), "{s:?}");
        }
        assert_ne!(default_keys(Shot::Area), default_keys(Shot::Full));
    }

    #[test]
    fn status_reports_busy_keys_and_the_snipping_tool() {
        let prtsc = "PrintScreen";
        let full = "shift+PrintScreen";
        assert_eq!(status(&[(prtsc, true), (full, true)], false), Status::Ready);
        assert_eq!(
            status(&[(prtsc, false), (full, true)], false),
            Status::Busy {
                keys: vec!["PrtSc".into()]
            }
        );
        // The Snipping Tool takes the plain PrtSc even when the registration worked.
        assert_eq!(
            status(&[(prtsc, true), (full, true)], true),
            Status::Busy {
                keys: vec!["PrtSc".into()]
            }
        );
        // A rebound shortcut is not affected by it.
        assert_eq!(status(&[("F9", true), (full, true)], true), Status::Ready);
    }

    #[test]
    fn the_hook_clears_the_prtsc_conflict_only() {
        let busy = |k: &[&str]| Status::Busy {
            keys: k.iter().map(|s| s.to_string()).collect(),
        };
        assert_eq!(effective_status(&busy(&["PrtSc"]), true), Status::Ready);
        assert_eq!(
            effective_status(&busy(&["PrtSc", "Shift+PrtSc"]), true),
            Status::Ready
        );
        assert_eq!(effective_status(&busy(&["F9"]), true), busy(&["F9"]));
        assert_eq!(effective_status(&busy(&["PrtSc"]), false), busy(&["PrtSc"]));
        assert_eq!(effective_status(&Status::Off, true), Status::Off);
    }

    #[test]
    fn pressed_keys_become_hotkey_text() {
        use egui::{Key, Modifiers};
        let both = Modifiers::CTRL | Modifiers::SHIFT;
        assert_eq!(
            keys_from_egui(Key::A, both).as_deref(),
            Some("ctrl+shift+KeyA")
        );
        assert_eq!(
            keys_from_egui(Key::Num2, Modifiers::ALT).as_deref(),
            Some("alt+Digit2")
        );
        assert_eq!(
            keys_from_egui(Key::F9, Modifiers::NONE).as_deref(),
            Some("F9")
        );
        assert_eq!(
            keys_from_egui(Key::Escape, Modifiers::NONE),
            None,
            "Esc cancels"
        );
        for k in [Key::A, Key::Num2, Key::F9, Key::Space, Key::Home] {
            let text = keys_from_egui(k, Modifiers::SHIFT).unwrap();
            assert!(HotKey::from_str(&text).is_ok(), "{text}");
        }
    }

    #[test]
    fn priority_is_offered_once_and_never_with_the_hook_on() {
        let busy = Status::Busy {
            keys: vec!["PrtSc".into()],
        };
        assert!(should_offer_priority(&busy, false, false));
        assert!(!should_offer_priority(&busy, true, false), "offered before");
        assert!(
            !should_offer_priority(&busy, false, true),
            "hook already on"
        );
        assert!(!should_offer_priority(&Status::Ready, false, false));
        assert!(!should_offer_priority(&Status::Off, false, false));
        let f9 = Status::Busy {
            keys: vec!["F9".into()],
        };
        assert!(
            !should_offer_priority(&f9, false, false),
            "the hook cannot help F9"
        );
    }
}
