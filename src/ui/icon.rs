//! Lucide icons drawn from the embedded icon font, one glyph per `Ico`.

use egui::{Color32, FontFamily, FontId, RichText};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ico {
    Select,
    Pen,
    Line,
    Arrow,
    Rect,
    Ellipse,
    Marker,
    Text,
    Blur,
    Undo,
    Redo,
    Pin,
    Download,
    FileDown,
    Copy,
    CopyPlus,
    Edit,
    Close,
    Scan,
    Monitor,
    Timer,
    ImagePlus,
    Images,
    FolderOpen,
    FolderSearch,
    Trash,
    Search,
    Home,
    Grid,
    Palette,
    Settings,
    Settings2,
    Sliders,
    Keyboard,
    HardDrive,
    Info,
    Sun,
    Moon,
    SunMoon,
    ChevronDown,
    ChevronRight,
    ChevronLeft,
    ChevronUp,
    Check,
    Plus,
    RotateCcw,
    Maximize,
    ZoomIn,
    ZoomOut,
    Eye,
    EyeOff,
    BringFront,
    SendBack,
    PaintBucket,
    Alert,
    Power,
    ArrowDownToLine,
    ExternalLink,
    Move,
    TextCursor,
    Hand,
    Minimize,
    Square,
}

pub const ALL: &[Ico] = &[
    Ico::Select,
    Ico::Pen,
    Ico::Line,
    Ico::Arrow,
    Ico::Rect,
    Ico::Ellipse,
    Ico::Marker,
    Ico::Text,
    Ico::Blur,
    Ico::Undo,
    Ico::Redo,
    Ico::Pin,
    Ico::Download,
    Ico::FileDown,
    Ico::Copy,
    Ico::CopyPlus,
    Ico::Edit,
    Ico::Close,
    Ico::Scan,
    Ico::Monitor,
    Ico::Timer,
    Ico::ImagePlus,
    Ico::Images,
    Ico::FolderOpen,
    Ico::FolderSearch,
    Ico::Trash,
    Ico::Search,
    Ico::Home,
    Ico::Grid,
    Ico::Palette,
    Ico::Settings,
    Ico::Settings2,
    Ico::Sliders,
    Ico::Keyboard,
    Ico::HardDrive,
    Ico::Info,
    Ico::Sun,
    Ico::Moon,
    Ico::SunMoon,
    Ico::ChevronDown,
    Ico::ChevronRight,
    Ico::ChevronLeft,
    Ico::ChevronUp,
    Ico::Check,
    Ico::Plus,
    Ico::RotateCcw,
    Ico::Maximize,
    Ico::ZoomIn,
    Ico::ZoomOut,
    Ico::Eye,
    Ico::EyeOff,
    Ico::BringFront,
    Ico::SendBack,
    Ico::PaintBucket,
    Ico::Alert,
    Ico::Power,
    Ico::ArrowDownToLine,
    Ico::ExternalLink,
    Ico::Move,
    Ico::TextCursor,
    Ico::Hand,
    Ico::Minimize,
    Ico::Square,
];

/// Codepoints from `lucide-static` 1.70.0 `font/codepoints.json`; the Lucide name is in each comment.
pub fn glyph(i: Ico) -> char {
    let cp: u32 = match i {
        Ico::Select => 0xe1c3,          // mouse-pointer-2
        Ico::Pen => 0xe1f9,             // pencil
        Ico::Line => 0xe11c,            // minus
        Ico::Arrow => 0xe493,           // move-up-right
        Ico::Rect => 0xe167,            // square
        Ico::Ellipse => 0xe076,         // circle
        Ico::Marker => 0xe0f4,          // highlighter
        Ico::Text => 0xe198,            // type
        Ico::Blur => 0xe0e9,            // grid-3x3
        Ico::Undo => 0xe2a1,            // undo-2
        Ico::Redo => 0xe2a0,            // redo-2
        Ico::Pin => 0xe259,             // pin
        Ico::Download => 0xe0b2,        // download
        Ico::FileDown => 0xe318,        // file-down
        Ico::Copy => 0xe09e,            // copy
        Ico::CopyPlus => 0xe3fd,        // copy-plus
        Ico::Edit => 0xe4f1,            // pencil-ruler
        Ico::Close => 0xe1b2,           // x
        Ico::Scan => 0xe257,            // scan
        Ico::Monitor => 0xe11d,         // monitor
        Ico::Timer => 0xe1e0,           // timer
        Ico::ImagePlus => 0xe1f7,       // image-plus
        Ico::Images => 0xe5c4,          // images
        Ico::FolderOpen => 0xe247,      // folder-open
        Ico::FolderSearch => 0xe339,    // folder-search
        Ico::Trash => 0xe18e,           // trash-2
        Ico::Search => 0xe151,          // search
        Ico::Home => 0xe0f5,            // house
        Ico::Grid => 0xe0ff,            // layout-grid
        Ico::Palette => 0xe1dd,         // palette
        Ico::Settings => 0xe154,        // settings
        Ico::Settings2 => 0xe245,       // settings-2
        Ico::Sliders => 0xe29a,         // sliders-horizontal
        Ico::Keyboard => 0xe284,        // keyboard
        Ico::HardDrive => 0xe4e5,       // hard-drive-download
        Ico::Info => 0xe0f9,            // info
        Ico::Sun => 0xe178,             // sun
        Ico::Moon => 0xe11e,            // moon
        Ico::SunMoon => 0xe2b2,         // sun-moon
        Ico::ChevronDown => 0xe06d,     // chevron-down
        Ico::ChevronRight => 0xe06f,    // chevron-right
        Ico::ChevronLeft => 0xe06e,     // chevron-left
        Ico::ChevronUp => 0xe070,       // chevron-up
        Ico::Check => 0xe06c,           // check
        Ico::Plus => 0xe13d,            // plus
        Ico::RotateCcw => 0xe148,       // rotate-ccw
        Ico::Maximize => 0xe112,        // maximize
        Ico::ZoomIn => 0xe1b6,          // zoom-in
        Ico::ZoomOut => 0xe1b7,         // zoom-out
        Ico::Eye => 0xe0ba,             // eye
        Ico::EyeOff => 0xe0bb,          // eye-off
        Ico::BringFront => 0xe4ef,      // bring-to-front
        Ico::SendBack => 0xe4f3,        // send-to-back
        Ico::PaintBucket => 0xe2e6,     // paint-bucket
        Ico::Alert => 0xe193,           // triangle-alert
        Ico::Power => 0xe140,           // power
        Ico::ArrowDownToLine => 0xe455, // arrow-down-to-line
        Ico::ExternalLink => 0xe0b9,    // external-link
        Ico::Move => 0xe121,            // move
        Ico::TextCursor => 0xe264,      // text-cursor
        Ico::Hand => 0xe1d7,            // hand
        Ico::Minimize => 0xe11c,        // minus
        Ico::Square => 0xe167,          // square
    };
    char::from_u32(cp).unwrap_or('?')
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("icons".into()))
}

pub fn text(i: Ico, size: f32, color: Color32) -> RichText {
    RichText::new(glyph(i).to_string())
        .font(font(size))
        .color(color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ab_glyph::Font;

    #[test]
    fn every_icon_exists_in_lucide() {
        let f = ab_glyph::FontRef::try_from_slice(crate::ui::assets::LUCIDE).unwrap();
        for &i in ALL {
            assert_ne!(f.glyph_id(glyph(i)).0, 0, "{i:?} sem glifo");
        }
    }
}
