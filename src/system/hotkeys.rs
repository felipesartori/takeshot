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

/// Why a shortcut could not be set.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RebindError {
    /// Another app holds the keys.
    InUse,
    /// The keys do not make a shortcut.
    Invalid,
    /// The system refused it (its error text, for the log).
    Refused(String),
}

/// What a `global-hotkey` error means for the user.
fn rebind_error(e: &global_hotkey::Error) -> RebindError {
    use global_hotkey::Error as E;
    match e {
        E::AlreadyRegistered(_) => RebindError::InUse,
        E::HotKeyParseError(_)
        | E::UnrecognizedHotKeyCode(_)
        | E::EmptyHotKeyToken(_)
        | E::UnexpectedHotKeyFormat(_) => RebindError::Invalid,
        E::FailedToRegister(why) => RebindError::Refused(why.clone()),
        other => RebindError::Refused(other.to_string()),
    }
}

/// Whether a shortcut another app holds still works through Takeshot's PrtSc priority: the hook
/// sees PrtSc and Shift+PrtSc, so with it on such keys are kept without registering them.
pub(crate) fn kept_by_hook(e: &RebindError, keys: &str, hook_on: bool) -> bool {
    hook_on && *e == RebindError::InUse && matches!(label(keys).as_str(), "PrtSc" | "Shift+PrtSc")
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

/// egui sends Shift, Ctrl, Alt and Cmd as keys of their own. Such a key only starts a shortcut,
/// and Change skips it to reach the key that ends it.
pub fn is_modifier_key(key: egui::Key) -> bool {
    use egui::Key as K;
    matches!(
        key,
        K::ShiftLeft
            | K::ShiftRight
            | K::ControlLeft
            | K::ControlRight
            | K::AltLeft
            | K::AltRight
            | K::SuperLeft
            | K::SuperRight
    )
}

/// A global shortcut needs Ctrl, Alt or the Command key, or an F key: a plain or Shift+ key
/// would be taken from every app while typing. Change keeps waiting on such a key.
pub fn needs_modifier(key: egui::Key, mods: egui::Modifiers) -> bool {
    let f_key = key
        .name()
        .strip_prefix('F')
        .is_some_and(|n| n.parse::<u8>().is_ok());
    !(f_key || mods.ctrl || mods.alt || mods.command || mods.mac_cmd)
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
    /// The Settings switch; off leaves the keys free for other apps.
    enabled: bool,
    /// Why the system refused global shortcuts, if it did.
    refused: Option<String>,
}

fn slot(s: Shot) -> usize {
    match s {
        Shot::Area => 0,
        Shot::Full => 1,
    }
}

impl Hotkeys {
    /// Registers the shortcuts from `settings` (when switched on) and calls `on_shot` on the hotkey
    /// thread when one fires. The manager is made either way, so the switch can act at once.
    pub fn start(settings: &Settings, on_shot: impl Fn(Shot) + Send + Sync + 'static) -> Self {
        let mut h = Self {
            manager: None,
            keys: [settings.hotkey_area.clone(), settings.hotkey_full.clone()],
            registered: [None, None],
            ids: Arc::new([AtomicU32::new(0), AtomicU32::new(0)]),
            status: Status::Off,
            enabled: settings.hotkeys,
            refused: None,
        };
        let manager = match GlobalHotKeyManager::new() {
            Ok(m) => m,
            Err(e) => {
                h.refused = Some(e.to_string());
                h.refresh_status();
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
        if h.enabled {
            for s in [Shot::Area, Shot::Full] {
                let keys = h.keys[slot(s)].clone();
                let _ = h.register(s, &keys);
            }
        }
        h.refresh_status();
        h
    }

    /// A set of shortcuts the system refused, for tests of what does not need the system.
    #[cfg(test)]
    fn refused(keys: [&str; 2], enabled: bool, error: &str) -> Self {
        let mut h = Self {
            manager: None,
            keys: keys.map(str::to_string),
            registered: [None, None],
            ids: Arc::new([AtomicU32::new(0), AtomicU32::new(0)]),
            status: Status::Off,
            enabled,
            refused: Some(error.to_string()),
        };
        h.refresh_status();
        h
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Turns the global shortcuts on or off now: off unregisters them, on registers them again.
    pub fn set_enabled(&mut self, on: bool) {
        if on == self.enabled {
            return;
        }
        self.enabled = on;
        for s in [Shot::Area, Shot::Full] {
            if on {
                let keys = self.keys[slot(s)].clone();
                let _ = self.register(s, &keys);
            } else if let (Some(m), Some(hk)) = (&self.manager, self.registered[slot(s)].take()) {
                let _ = m.unregister(hk);
            }
        }
        self.refresh_status();
    }

    /// Swaps the shortcut of a capture; on failure the old one stays registered. While the
    /// shortcuts are off the new keys are only kept, and so are keys another app holds that the
    /// PrtSc priority hook (`hook_on`) catches anyway.
    pub fn rebind(&mut self, s: Shot, keys: &str, hook_on: bool) -> Result<(), RebindError> {
        HotKey::from_str(keys).map_err(|_| RebindError::Invalid)?;
        if !self.enabled {
            self.keys[slot(s)] = keys.to_string();
            return Ok(());
        }
        let old = self.keys[slot(s)].clone();
        if let (Some(m), Some(hk)) = (&self.manager, self.registered[slot(s)].take()) {
            let _ = m.unregister(hk);
        }
        let result = match self.register(s, keys) {
            Err(e) if kept_by_hook(&e, keys, hook_on) => {
                self.keys[slot(s)] = keys.to_string();
                Ok(())
            }
            r => r,
        };
        if result.is_err() {
            let _ = self.register(s, &old);
        }
        self.refresh_status();
        result
    }

    fn register(&mut self, s: Shot, keys: &str) -> Result<(), RebindError> {
        let hk = HotKey::from_str(keys).map_err(|_| RebindError::Invalid)?;
        let m = self
            .manager
            .as_ref()
            .ok_or_else(|| RebindError::Refused("global shortcuts are off".into()))?;
        m.register(hk).map_err(|e| rebind_error(&e))?;
        self.ids[slot(s)].store(hk.id(), Ordering::Relaxed);
        self.registered[slot(s)] = Some(hk);
        self.keys[slot(s)] = keys.to_string();
        Ok(())
    }

    fn refresh_status(&mut self) {
        self.status = if !self.enabled {
            Status::Off
        } else if let Some(e) = &self.refused {
            Status::Unavailable(e.clone())
        } else {
            let reg = [
                (self.keys[0].as_str(), self.registered[0].is_some()),
                (self.keys[1].as_str(), self.registered[1].is_some()),
            ];
            status(&reg, snipping_tool_owns_prtsc())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prtsc_key_the_hook_covers_is_kept_unregistered() {
        let in_use = RebindError::InUse;
        assert!(kept_by_hook(&in_use, "PrintScreen", true));
        assert!(kept_by_hook(&in_use, "shift+PrintScreen", true));
        assert!(!kept_by_hook(&in_use, "PrintScreen", false), "hook off");
        assert!(
            !kept_by_hook(&in_use, "F8", true),
            "a key the hook does not see"
        );
        assert!(!kept_by_hook(&RebindError::Invalid, "PrintScreen", true));
    }

    #[test]
    fn a_taken_key_is_reported_as_in_use() {
        use global_hotkey::Error as E;
        let prtsc = HotKey::from_str("PrintScreen").unwrap();
        assert_eq!(
            rebind_error(&E::AlreadyRegistered(prtsc)),
            RebindError::InUse
        );
        assert_eq!(
            rebind_error(&E::HotKeyParseError("x".into())),
            RebindError::Invalid
        );
        assert_eq!(
            rebind_error(&E::FailedToRegister("denied".into())),
            RebindError::Refused("denied".into())
        );
    }

    #[test]
    fn a_global_shortcut_needs_ctrl_alt_or_an_f_key() {
        use egui::{Key, Modifiers};
        // A plain letter or Shift+letter would take that key from every app while typing.
        assert!(needs_modifier(Key::A, Modifiers::NONE));
        assert!(needs_modifier(Key::A, Modifiers::SHIFT));
        assert!(needs_modifier(Key::Space, Modifiers::NONE));
        assert!(!needs_modifier(Key::A, Modifiers::CTRL));
        assert!(!needs_modifier(Key::A, Modifiers::ALT));
        assert!(!needs_modifier(Key::F9, Modifiers::NONE));
        assert!(!needs_modifier(Key::F9, Modifiers::SHIFT));
        assert!(!needs_modifier(
            Key::Num9,
            Modifiers::COMMAND | Modifiers::SHIFT
        ));
    }

    #[test]
    fn a_modifier_key_only_starts_a_shortcut() {
        use egui::Key;
        for key in [
            Key::ShiftLeft,
            Key::ShiftRight,
            Key::ControlLeft,
            Key::ControlRight,
            Key::AltLeft,
            Key::AltRight,
            Key::SuperLeft,
            Key::SuperRight,
        ] {
            assert!(is_modifier_key(key), "{key:?}");
        }
        for key in [Key::A, Key::Num9, Key::F9, Key::Escape, Key::PrintScreen] {
            assert!(!is_modifier_key(key), "{key:?}");
        }
    }
    use global_hotkey::hotkey::HotKey;
    use std::str::FromStr;

    #[test]
    fn the_switch_applies_at_once() {
        // Without a system manager (as on Wayland) the switch still reports what was chosen.
        let mut h = Hotkeys::refused(["PrintScreen", "shift+PrintScreen"], true, "no display");
        assert_eq!(h.status(), &Status::Unavailable("no display".into()));
        h.set_enabled(false);
        assert!(!h.enabled());
        assert_eq!(h.status(), &Status::Off);
        // A shortcut changed while they are off is kept for when they come back.
        assert_eq!(h.rebind(Shot::Area, "F9", false), Ok(()));
        h.set_enabled(true);
        assert_eq!(h.status(), &Status::Unavailable("no display".into()));
    }

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
