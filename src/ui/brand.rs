//! Brand images embedded in the binary: the app icon per theme, tray icons and the mark.
//! The PNGs come from `cargo run --example brand`, which renders the SVG masters in assets/brand/.

use crate::ui::tokens::ThemeId;

macro_rules! png {
    ($name:literal) => {
        include_bytes!(concat!("../../assets/brand/png/", $name, ".png"))
    };
}

fn decode(bytes: &[u8]) -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("embedded brand PNG")
        .to_rgba8();
    let (w, h) = img.dimensions();
    (img.into_raw(), w, h)
}

/// App icon for the window, taskbar and Dock, in the active theme (the .exe keeps the TakeUI icon).
pub fn window_icon(theme: ThemeId) -> egui::IconData {
    let bytes: &[u8] = match theme {
        ThemeId::TakeUi => png!("takeui-app-256"),
        ThemeId::Coral => png!("coral-app-256"),
    };
    let (rgba, width, height) = decode(bytes);
    egui::IconData {
        rgba,
        width,
        height,
    }
}

/// Tray glyph for a dark or light taskbar; sizes 16, 20, 24 and 32 (anything else gets 32).
pub fn tray_rgba(theme: ThemeId, dark_taskbar: bool, size: u32) -> (Vec<u8>, u32, u32) {
    let bytes: &[u8] = match (theme, dark_taskbar, size) {
        (ThemeId::TakeUi, true, 16) => png!("takeui-tray-dark-16"),
        (ThemeId::TakeUi, true, 20) => png!("takeui-tray-dark-20"),
        (ThemeId::TakeUi, true, 24) => png!("takeui-tray-dark-24"),
        (ThemeId::TakeUi, true, _) => png!("takeui-tray-dark-32"),
        (ThemeId::TakeUi, false, 16) => png!("takeui-tray-light-16"),
        (ThemeId::TakeUi, false, 20) => png!("takeui-tray-light-20"),
        (ThemeId::TakeUi, false, 24) => png!("takeui-tray-light-24"),
        (ThemeId::TakeUi, false, _) => png!("takeui-tray-light-32"),
        (ThemeId::Coral, true, 16) => png!("coral-tray-dark-16"),
        (ThemeId::Coral, true, 20) => png!("coral-tray-dark-20"),
        (ThemeId::Coral, true, 24) => png!("coral-tray-dark-24"),
        (ThemeId::Coral, true, _) => png!("coral-tray-dark-32"),
        (ThemeId::Coral, false, 16) => png!("coral-tray-light-16"),
        (ThemeId::Coral, false, 20) => png!("coral-tray-light-20"),
        (ThemeId::Coral, false, 24) => png!("coral-tray-light-24"),
        (ThemeId::Coral, false, _) => png!("coral-tray-light-32"),
    };
    decode(bytes)
}

/// The theme's mark (128 px) as a texture, in the colour that reads on a dark or light surface.
pub fn mark_texture(ctx: &egui::Context, theme: ThemeId, dark: bool) -> egui::TextureHandle {
    let (name, bytes): (&str, &[u8]) = match (theme, dark) {
        (ThemeId::TakeUi, true) => ("mark-takeui-dark", png!("takeui-mark-dark-128")),
        (ThemeId::TakeUi, false) => ("mark-takeui-light", png!("takeui-mark-light-128")),
        (ThemeId::Coral, _) => ("mark-coral", png!("coral-mark-128")),
    };
    let (rgba, w, h) = decode(bytes);
    let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture(name, img, egui::TextureOptions::LINEAR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tokens::ThemeId;

    #[test]
    fn ico_has_all_sizes() {
        let f = std::fs::File::open("assets/brand/takeshot.ico").unwrap();
        let dir = ico::IconDir::read(f).unwrap();
        let mut sizes: Vec<u32> = dir.entries().iter().map(|e| e.width()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![16, 20, 24, 32, 40, 48, 64, 256]);
    }

    #[test]
    fn window_icon_is_256() {
        for th in [ThemeId::TakeUi, ThemeId::Coral] {
            let i = window_icon(th);
            assert_eq!((i.width, i.height), (256, 256));
            assert_eq!(i.rgba.len(), 256 * 256 * 4);
        }
    }

    #[test]
    fn tray_icons_follow_theme_and_taskbar() {
        for th in [ThemeId::TakeUi, ThemeId::Coral] {
            for dark in [true, false] {
                for size in [16, 20, 24, 32] {
                    let (rgba, w, h) = tray_rgba(th, dark, size);
                    assert_eq!((w, h), (size, size));
                    assert_eq!(rgba.len() as u32, size * size * 4);
                }
            }
        }
        // A dark taskbar gets a light glyph: its brightest opaque pixel is near white.
        let (rgba, _, _) = tray_rgba(ThemeId::Coral, true, 32);
        let max = rgba
            .chunks(4)
            .filter(|p| p[3] > 200)
            .map(|p| p[0])
            .max()
            .unwrap();
        assert!(max > 240);
    }
}
