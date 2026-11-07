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

    #[test]
    fn ico_has_all_sizes() {
        let f = std::fs::File::open("assets/brand/takeshot.ico").unwrap();
        let dir = ico::IconDir::read(f).unwrap();
        let mut sizes: Vec<u32> = dir.entries().iter().map(|e| e.width()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![16, 20, 24, 32, 40, 48, 64, 256]);
    }

    /// RGB of pixel (x, y) in a straight-alpha RGBA buffer `width` pixels wide.
    fn rgb_at(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * width + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2]]
    }

    fn near(got: [u8; 3], want: [u8; 3]) -> bool {
        got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 3)
    }

    #[test]
    fn window_icon_is_the_theme_icon() {
        // (128, 20) is plain background above the glyph: TakeUI #733FA2, Coral #FF6B47.
        for (th, bg) in [
            (ThemeId::TakeUi, [0x73, 0x3F, 0xA2]),
            (ThemeId::Coral, [0xFF, 0x6B, 0x47]),
        ] {
            let i = window_icon(th);
            assert_eq!((i.width, i.height), (256, 256));
            assert_eq!(i.rgba.len(), 256 * 256 * 4);
            let got = rgb_at(&i.rgba, 256, 128, 20);
            assert!(near(got, bg), "{th:?}: background {got:?}, want {bg:?}");
        }
    }

    #[test]
    fn tray_icons_follow_theme_and_taskbar() {
        for th in [ThemeId::TakeUi, ThemeId::Coral] {
            for dark in [true, false] {
                for size in [16, 20, 24, 32] {
                    let cell = format!("{th:?} dark={dark} {size}px");
                    let (rgba, w, h) = tray_rgba(th, dark, size);
                    assert_eq!((w, h), (size, size), "{cell}");
                    assert_eq!(rgba.len() as u32, size * size * 4, "{cell}");
                    // A dark taskbar gets a white glyph, a light one a near-black glyph.
                    let red = rgba
                        .chunks(4)
                        .filter(|p| p[3] > 200)
                        .map(|p| p[0])
                        .max()
                        .unwrap();
                    if dark {
                        assert!(red > 240, "{cell}: brightest red {red}");
                    } else {
                        assert!(red < 60, "{cell}: brightest red {red}");
                    }
                    // The centre is TakeUI's green dot or Coral's neutral square.
                    let [r, g, b] = rgb_at(&rgba, w, size / 2, size / 2);
                    let green = g > r.saturating_add(40) && g > b.saturating_add(40);
                    assert_eq!(green, th == ThemeId::TakeUi, "{cell}: centre {r},{g},{b}");
                }
            }
        }
    }
}
