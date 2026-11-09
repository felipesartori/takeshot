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
    /// The corner radius of the next rectangle, in picture px.
    pub radius: f32,
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
            radius: 0.0,
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

/// How Início shows the captures.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum HomeView {
    #[default]
    Grid,
    List,
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
    /// An area capture dims every monitor and the area is chosen on any of them; off, only the
    /// monitor under the mouse.
    pub capture_all_screens: bool,
    /// The main window gets out of the way before a capture; off, it can be captured too.
    pub hide_on_capture: bool,
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
    /// Início as a grid of cards or a list.
    pub home_view: HomeView,
    /// Início's detail panel is shown.
    pub home_details: bool,
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
            capture_all_screens: false,
            hide_on_capture: true,
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
            home_view: HomeView::Grid,
            home_details: true,
            ui_theme: ThemeId::default(),
            ui_mode: UiMode::default(),
            lang: LangPref::default(),
        }
    }
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("takeshot").join("settings.json"))
}

/// What this process last read from or wrote to the settings file.
pub type Seen = Option<serde_json::Value>;

/// This process's `Seen`.
static SEEN: std::sync::Mutex<Seen> = std::sync::Mutex::new(None);

impl Settings {
    pub fn load() -> Self {
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        path()
            .map(|p| Self::load_at(&p, &mut seen))
            .unwrap_or_default()
    }

    /// The settings in the file at `p` (the defaults without one), remembered in `seen`.
    pub fn load_at(p: &std::path::Path, seen: &mut Seen) -> Self {
        let s: Self = std::fs::read_to_string(p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        *seen = serde_json::to_value(&s).ok();
        s
    }

    /// Writes to the file at `p` only what changed in this process since `seen`, over what is
    /// there: the resident, the editors and the capture each keep a copy, and one saving its
    /// whole copy would undo what the others saved meanwhile. A temporary file of this process's
    /// own, then a rename, so two saving at once never mix.
    pub fn save_at(&self, p: &std::path::Path, seen: &mut Seen) {
        let Ok(mine) = serde_json::to_value(self) else {
            return;
        };
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let disk = std::fs::read_to_string(p)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let out = match (disk, seen.as_ref()) {
            (Some(mut d), Some(base)) => {
                merge(&mut d, base, &mine);
                d
            }
            _ => mine.clone(),
        };
        let Ok(text) = serde_json::to_string_pretty(&out) else {
            return;
        };
        let tmp = p.with_extension(format!("json.{}.tmp", std::process::id()));
        if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, p).is_ok() {
            *seen = Some(mine);
        }
    }

    /// `save_at` the settings file, from what this process last read or wrote.
    pub fn save(&self) {
        let Some(p) = path() else { return };
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        self.save_at(&p, &mut seen);
    }
}

/// Writes into `disk` what `mine` changed from `base`, field by field.
fn merge(disk: &mut serde_json::Value, base: &serde_json::Value, mine: &serde_json::Value) {
    use serde_json::Value;
    if base == mine {
        return;
    }
    match (disk, base, mine) {
        (Value::Object(d), Value::Object(b), Value::Object(m)) => {
            for (k, mv) in m {
                match (d.get_mut(k), b.get(k)) {
                    (Some(dv), Some(bv)) => merge(dv, bv, mv),
                    (_, Some(bv)) if bv == mv => {}
                    _ => {
                        d.insert(k.clone(), mv.clone());
                    }
                }
            }
        }
        (d, _, m) => *d = m.clone(),
    }
}

/// Style's "Restore default": background, frame, watermark and the direct-copy style go back to
/// their defaults; the chosen logo file stays (picking it again would be a chore).
pub fn restore_default_style(s: &mut Settings) {
    let logo = s.watermark.logo_path.take();
    let d = Settings::default();
    s.style = d.style;
    s.watermark = d.watermark;
    s.watermark.logo_path = logo;
    s.quick_style = d.quick_style;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_process_saves_only_what_it_changed() {
        let dir = std::env::temp_dir().join(format!("takeshot-merge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("settings.json");
        Settings::default().save_at(&p, &mut None);
        // The resident and an editor, each with its own copy.
        let (mut seen_a, mut seen_b) = (None, None);
        let mut a = Settings::load_at(&p, &mut seen_a);
        let mut b = Settings::load_at(&p, &mut seen_b);
        a.ui_mode = UiMode::Light;
        a.save_at(&p, &mut seen_a);
        b.draw.width = 9.0;
        b.save_at(&p, &mut seen_b);
        let got = Settings::load_at(&p, &mut None);
        assert_eq!(got.ui_mode, UiMode::Light, "the resident's change stays");
        assert_eq!(got.draw.width, 9.0, "and the editor's");
        // Two fields of one group, one each.
        a.style.padding = 33.0;
        a.save_at(&p, &mut seen_a);
        b.style.radius = 21.0;
        b.save_at(&p, &mut seen_b);
        let got = Settings::load_at(&p, &mut None);
        assert_eq!((got.style.padding, got.style.radius), (33.0, 21.0));
        assert_eq!(
            got.draw.width, 9.0,
            "a later save keeps an earlier one's change"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_default_keeps_the_logo() {
        let mut s = Settings::default();
        s.style.padding = 20.0;
        s.style.background = Background::Solid;
        s.watermark.enabled = true;
        s.watermark.text = "@me".into();
        s.watermark.logo_path = Some("logo.png".into());
        s.quick_style = QuickStyle::Plain;
        restore_default_style(&mut s);
        assert_eq!(
            serde_json::to_string(&s.style).unwrap(),
            serde_json::to_string(&Style::default()).unwrap()
        );
        assert_eq!(s.quick_style, Settings::default().quick_style);
        assert!(!s.watermark.enabled);
        assert!(s.watermark.text.is_empty());
        assert_eq!(
            s.watermark.logo_path.as_deref(),
            Some("logo.png"),
            "the logo stays"
        );
    }

    #[test]
    fn old_settings_file_loads_with_new_defaults() {
        let old = r#"{"save_dir":"D:/caps","auto_save":true,"show_bubbles":false}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.save_dir, "D:/caps");
        assert!(s.auto_save);
        assert!(!s.show_bubbles);
        assert!(s.hide_on_capture, "Takeshot still hides when capturing");
        assert!(!s.capture_all_screens);
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
