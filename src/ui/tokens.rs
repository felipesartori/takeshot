//! Colour tokens of the two themes (TakeUI and Coral UI) in light and dark, and how the mode is resolved.

use egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum ThemeId {
    #[default]
    TakeUi,
    Coral,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum UiMode {
    Light,
    Dark,
    #[default]
    System,
}

/// How a selected item is marked: a 2 px ring (TakeUI) or four crop ticks (Coral UI).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelStyle {
    Ring,
    CropTicks,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    pub theme: ThemeId,
    pub dark: bool,
    pub sel_style: SelStyle,
    pub radius: u8,
    pub shell: Color32,
    pub surf: Color32,
    pub raised: Color32,
    pub hover: Color32,
    pub press: Color32,
    pub border: Color32,
    pub line: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub primary: Color32,
    pub primary_hover: Color32,
    pub primary_press: Color32,
    pub on_primary: Color32,
    pub accent: Color32,
    pub tint: Color32,
    pub green: Color32,
    pub warn: Color32,
    pub danger: Color32,
    pub track: Color32,
    pub input: Color32,
    pub seg_sel: Color32,
    pub tip_bg: Color32,
    pub tip_text: Color32,
    pub canvas: Color32,
    pub title_bar: Color32,
}

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Columns of the spec table, in this order: TakeUI dark, TakeUI light, Coral dark, Coral light.
const fn pick(theme: ThemeId, dark: bool, v: [u32; 4]) -> Color32 {
    rgb(match (theme, dark) {
        (ThemeId::TakeUi, true) => v[0],
        (ThemeId::TakeUi, false) => v[1],
        (ThemeId::Coral, true) => v[2],
        (ThemeId::Coral, false) => v[3],
    })
}

pub fn tokens(theme: ThemeId, dark: bool) -> Tokens {
    let c = |v: [u32; 4]| pick(theme, dark, v);
    Tokens {
        theme,
        dark,
        sel_style: match theme {
            ThemeId::TakeUi => SelStyle::Ring,
            ThemeId::Coral => SelStyle::CropTicks,
        },
        radius: 8,
        shell: c([0x121212, 0xF5F5F5, 0x111111, 0xF6F6F5]),
        surf: c([0x1C1C1C, 0xFFFFFF, 0x1A1A1A, 0xFFFFFF]),
        raised: c([0x262626, 0xF0F0F0, 0x242424, 0xF0F0EF]),
        hover: c([0x2E2E2E, 0xE8E8E8, 0x2C2C2C, 0xE8E8E7]),
        press: c([0x363636, 0xE0E0E0, 0x343434, 0xDFDFDE]),
        border: c([0x333333, 0xE5E5E5, 0x2E2E2E, 0xE4E4E2]),
        line: c([0x2A2A2A, 0xEDEDED, 0x262626, 0xEDEDEC]),
        text: c([0xE5E5E5, 0x171717, 0xEDEDED, 0x161616]),
        muted: c([0xA3A3A3, 0x5C5C5C, 0xA1A1A1, 0x5A5A5A]),
        faint: c([0x737373, 0x8F8F8F, 0x707070, 0x8C8C8C]),
        primary: c([0x8D43B1, 0x8D43B1, 0xFF6B47, 0xFF6B47]),
        primary_hover: c([0x9A52BE, 0x7E3BA0, 0xFF805F, 0xFF805F]),
        primary_press: c([0x7B3A9C, 0x6F348D, 0xF0582F, 0xF0582F]),
        on_primary: c([0xFFFFFF, 0xFFFFFF, 0x1F0D07, 0x1F0D07]),
        accent: c([0xC79BE6, 0x7A3A9E, 0xFF8A66, 0xC2410C]),
        tint: c([0x2B2033, 0xF4ECF9, 0x33201A, 0xFFEDE6]),
        green: c([0x3FCC59, 0x22A33F, 0x3FCC59, 0x22A33F]),
        warn: c([0xF5A524, 0xB45309, 0xF5A524, 0xB45309]),
        danger: c([0xE5484D, 0xDC2626, 0xE5484D, 0xDC2626]),
        track: c([0x363636, 0xE0E0E0, 0x343434, 0xDFDFDE]),
        input: c([0x171717, 0xFFFFFF, 0x151515, 0xFFFFFF]),
        seg_sel: c([0x363636, 0xFFFFFF, 0x343434, 0xFFFFFF]),
        tip_bg: c([0xE5E5E5, 0x171717, 0xEDEDED, 0x161616]),
        tip_text: c([0x171717, 0xFFFFFF, 0x171717, 0xFFFFFF]),
        canvas: c([0x0E0E0E, 0xEBEBEB, 0x0C0C0C, 0xE9E9E8]),
        title_bar: c([0x121212, 0xF5F5F5, 0x202020, 0xF3F3F3]),
    }
}

/// "System" follows the OS when it reports a theme and falls back to dark when it does not.
pub fn effective_dark(mode: UiMode, system: Option<egui::Theme>) -> bool {
    match mode {
        UiMode::Light => false,
        UiMode::Dark => true,
        UiMode::System => system != Some(egui::Theme::Light),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    fn hex(s: &str) -> Color32 {
        Color32::from_hex(s).unwrap()
    }

    #[test]
    fn takeui_dark_matches_spec() {
        let t = tokens(ThemeId::TakeUi, true);
        assert_eq!(t.shell, hex("#121212"));
        assert_eq!(t.primary, hex("#8D43B1"));
        assert_eq!(t.on_primary, Color32::WHITE);
        assert_eq!(t.green, hex("#3FCC59"));
        assert_eq!(t.sel_style, SelStyle::Ring);
    }

    #[test]
    fn coral_light_matches_spec() {
        let t = tokens(ThemeId::Coral, false);
        assert_eq!(t.shell, hex("#F6F6F5"));
        assert_eq!(t.primary, hex("#FF6B47"));
        assert_eq!(t.on_primary, hex("#1F0D07"));
        assert_eq!(t.accent, hex("#C2410C"));
        assert_eq!(t.sel_style, SelStyle::CropTicks);
    }

    #[test]
    fn system_mode_without_report_is_dark() {
        assert!(effective_dark(UiMode::System, None));
        assert!(!effective_dark(UiMode::System, Some(egui::Theme::Light)));
        assert!(effective_dark(UiMode::Dark, Some(egui::Theme::Light)));
        assert!(!effective_dark(UiMode::Light, Some(egui::Theme::Dark)));
    }
}
