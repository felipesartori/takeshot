//! Fonts and icon font embedded in the binary.

pub const ROBOTO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Roboto-Regular.ttf");
pub const ROBOTO_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Roboto-Medium.ttf");
pub const ROBOTO_BOLD: &[u8] = include_bytes!("../../assets/fonts/Roboto-Bold.ttf");
pub const ROBOTO_MONO_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/RobotoMono-Medium.ttf");
pub const QUICKSAND_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/Quicksand-Medium.ttf");
pub const LUCIDE: &[u8] = include_bytes!("../../assets/fonts/lucide.ttf");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_fonts_parse() {
        for (name, bytes) in [
            ("roboto-regular", ROBOTO_REGULAR),
            ("roboto-medium", ROBOTO_MEDIUM),
            ("roboto-bold", ROBOTO_BOLD),
            ("roboto-mono", ROBOTO_MONO_MEDIUM),
            ("quicksand", QUICKSAND_MEDIUM),
            ("lucide", LUCIDE),
        ] {
            assert!(
                ab_glyph::FontRef::try_from_slice(bytes).is_ok(),
                "{name} não abre"
            );
        }
    }
}
