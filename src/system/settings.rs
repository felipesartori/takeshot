//! User settings saved to settings.json: style, watermark, drawing options and app behavior.

use crate::engine::model::{BlurMode, DEFAULT_COLOR, DEFAULT_WIDTH, Tool};
use crate::system::hotkeys::{Shot, default_keys};
use crate::ui::i18n::LangPref;
use crate::ui::tokens::{ThemeId, UiMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Background {
    None,
    Preset(usize),
    Custom,
    Solid,
    Blurred,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Aspect {
    Auto,
    Wide,
    Classic,
    Square,
    Story,
}

impl Aspect {
    pub const ALL: [Aspect; 5] = [
        Aspect::Auto,
        Aspect::Wide,
        Aspect::Classic,
        Aspect::Square,
        Aspect::Story,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Aspect::Auto => "Livre",
            Aspect::Wide => "16:9",
            Aspect::Classic => "4:3",
            Aspect::Square => "1:1",
            Aspect::Story => "9:16",
        }
    }
    pub fn ratio(self) -> Option<f32> {
        match self {
            Aspect::Auto => None,
            Aspect::Wide => Some(16.0 / 9.0),
            Aspect::Classic => Some(4.0 / 3.0),
            Aspect::Square => Some(1.0),
            Aspect::Story => Some(9.0 / 16.0),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    pub background: Background,
    pub custom_a: [u8; 4],
    pub custom_b: [u8; 4],
    pub custom_angle: f32,
    pub solid: [u8; 4],
    /// Space around the picture, in % of its longest side.
    pub padding: f32,
    /// Corner radius in pixels of the picture.
    pub radius: f32,
    /// 0..1
    pub shadow: f32,
    pub window_bar: bool,
    pub aspect: Aspect,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: Background::Preset(0),
            custom_a: [255, 94, 58, 255],
            custom_b: [255, 196, 87, 255],
            custom_angle: 135.0,
            solid: [236, 232, 225, 255],
            padding: 7.0,
            radius: 14.0,
            shadow: 0.5,
            window_bar: false,
            aspect: Aspect::Auto,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WmPos {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    BandTop,
    BandBottom,
}

impl WmPos {
    pub const GRID: [WmPos; 9] = [
        WmPos::TopLeft,
        WmPos::TopCenter,
        WmPos::TopRight,
        WmPos::MiddleLeft,
        WmPos::Center,
        WmPos::MiddleRight,
        WmPos::BottomLeft,
        WmPos::BottomCenter,
        WmPos::BottomRight,
    ];
    pub fn is_band(self) -> bool {
        matches!(self, WmPos::BandTop | WmPos::BandBottom)
    }
    /// (horizontal, vertical) anchor in 0, 0.5 or 1.
    pub fn anchor(self) -> (f32, f32) {
        match self {
            WmPos::TopLeft => (0.0, 0.0),
            WmPos::TopCenter => (0.5, 0.0),
            WmPos::TopRight => (1.0, 0.0),
            WmPos::MiddleLeft => (0.0, 0.5),
            WmPos::Center => (0.5, 0.5),
            WmPos::MiddleRight => (1.0, 0.5),
            WmPos::BottomLeft => (0.0, 1.0),
            WmPos::BottomCenter => (0.5, 1.0),
            WmPos::BottomRight | WmPos::BandTop | WmPos::BandBottom => (1.0, 1.0),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            WmPos::TopLeft => "Canto superior esquerdo",
            WmPos::TopCenter => "Topo, centro",
            WmPos::TopRight => "Canto superior direito",
            WmPos::MiddleLeft => "Meio, esquerda",
            WmPos::Center => "Centro",
            WmPos::MiddleRight => "Meio, direita",
            WmPos::BottomLeft => "Canto inferior esquerdo",
            WmPos::BottomCenter => "Base, centro",
            WmPos::BottomRight => "Canto inferior direito",
            WmPos::BandTop => "Faixa superior",
            WmPos::BandBottom => "Faixa inferior",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Watermark {
    pub enabled: bool,
    pub logo_path: Option<String>,
    pub text: String,
    pub position: WmPos,
    /// Logo width in % of the final picture width.
    pub size: f32,
    pub opacity: f32,
    /// Distance from the edge in % of the picture's shorter side.
    pub margin: f32,
    pub text_color: [u8; 4],
    pub band_color: [u8; 4],
    /// Place corner marks over the screenshot itself instead of the whole canvas.
    pub inside_image: bool,
}

impl Default for Watermark {
    fn default() -> Self {
        Self {
            enabled: false,
            logo_path: None,
            text: String::new(),
            position: WmPos::BottomRight,
            size: 12.0,
            opacity: 0.85,
            margin: 3.0,
            text_color: [255, 255, 255, 255],
            band_color: [18, 18, 20, 255],
            inside_image: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum QuickStyle {
    Full,
    CornersOnly,
    Plain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DrawOpts {
    pub color: [u8; 4],
    pub width: f32,
    pub font: usize,
    pub font_size: f32,
    pub text_box: bool,
    pub filled: bool,
    pub blur_mode: BlurMode,
    pub last_tool: Tool,
    pub recent_colors: Vec<[u8; 4]>,
}

impl Default for DrawOpts {
    fn default() -> Self {
        Self {
            color: DEFAULT_COLOR,
            width: DEFAULT_WIDTH,
            font: 0,
            font_size: 26.0,
            text_box: false,
            filled: false,
            blur_mode: BlurMode::Blur,
            last_tool: Tool::Arrow,
            recent_colors: Vec::new(),
        }
    }
}

/// What the X of the main window does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum OnClose {
    /// Ask, with "Remember my choice".
    #[default]
    Ask,
    Minimize,
    Quit,
}

/// What a left click on the tray icon does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum TrayClick {
    #[default]
    OpenHome,
    CaptureArea,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub save_dir: String,
    pub auto_save: bool,
    pub keep_selection: bool,
    pub last_selection: Option<[f32; 4]>,
    pub quick_style: QuickStyle,
    pub open_editor_after_full: bool,
    /// Show a short notice after copying and saving.
    pub show_bubbles: bool,
    pub hotkeys: bool,
    /// Capture shortcuts in the form `global_hotkey::hotkey::HotKey::from_str` reads.
    pub hotkey_area: String,
    pub hotkey_full: String,
    /// Windows only: a low-level keyboard hook takes PrtSc before other apps.
    pub prtsc_priority: bool,
    /// The PrtSc priority offer was shown once already.
    pub prtsc_offer_shown: bool,
    pub on_close: OnClose,
    pub tray_click: TrayClick,
    pub style: Style,
    pub watermark: Watermark,
    pub draw: DrawOpts,
    pub ui_theme: ThemeId,
    pub ui_mode: UiMode,
    pub lang: LangPref,
}

impl Default for Settings {
    fn default() -> Self {
        let dir = dirs::picture_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Takeshot");
        Self {
            save_dir: dir.to_string_lossy().into_owned(),
            auto_save: false,
            keep_selection: false,
            last_selection: None,
            quick_style: QuickStyle::Full,
            open_editor_after_full: false,
            show_bubbles: true,
            hotkeys: true,
            hotkey_area: default_keys(Shot::Area).to_string(),
            hotkey_full: default_keys(Shot::Full).to_string(),
            prtsc_priority: false,
            prtsc_offer_shown: false,
            on_close: OnClose::default(),
            tray_click: TrayClick::default(),
            style: Style::default(),
            watermark: Watermark::default(),
            draw: DrawOpts::default(),
            ui_theme: ThemeId::default(),
            ui_mode: UiMode::default(),
            lang: LangPref::default(),
        }
    }
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("takeshot").join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_file_loads_with_new_defaults() {
        let old = r#"{"save_dir":"D:/caps","auto_save":true,"show_bubbles":false}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.save_dir, "D:/caps");
        assert!(s.auto_save);
        assert!(!s.show_bubbles);
        assert_eq!(s.ui_theme, ThemeId::TakeUi);
        assert_eq!(s.ui_mode, UiMode::System);
        assert_eq!(s.lang, LangPref::Auto);
    }

    #[test]
    fn old_settings_without_close_and_tray_fields_get_the_defaults() {
        let s: Settings = serde_json::from_str(r#"{"save_dir":"D:/caps"}"#).unwrap();
        assert_eq!(s.on_close, OnClose::Ask);
        assert_eq!(s.tray_click, TrayClick::OpenHome);
    }

    #[test]
    fn old_settings_without_hotkey_fields_get_the_defaults() {
        use crate::system::hotkeys::{Shot, default_keys};
        let s: Settings = serde_json::from_str(r#"{"save_dir":"D:/caps"}"#).unwrap();
        assert_eq!(s.hotkey_area, default_keys(Shot::Area));
        assert_eq!(s.hotkey_full, default_keys(Shot::Full));
        assert!(!s.prtsc_priority);
        assert!(!s.prtsc_offer_shown);
    }
}
