//! The skins: how each theme draws the screens. Behaviour stays in `screens`, `engine` and
//! `system`; a skin draws from that state and returns what the user asked for.

pub mod takeui;

use crate::Shared;
use crate::screens::home::{Home, MainAction};
use crate::ui::tokens::{ThemeId, Tokens};

/// The main window in the current theme: TakeUI for both themes until the Coral UI skin exists
/// (the Coral tokens already apply). Returns what the user asked for; `Home::apply` does it.
pub fn main_window(
    ui: &mut egui::Ui,
    t: &Tokens,
    home: &mut Home,
    shared: &mut Shared,
) -> Vec<MainAction> {
    let mut acts = home.begin(ui, shared);
    crate::ui::chrome::adopt_window("Takeshot");
    acts.extend(takeui::shell::main_window(ui, t, home, shared));
    acts
}

/// The editor in the current theme (TakeUI for both until the Coral UI skin exists). Returns what
/// the user asked for; `Editor::apply` does it.
pub fn editor(
    ui: &mut egui::Ui,
    t: &Tokens,
    ed: &mut crate::screens::editor::Editor,
    shared: &mut Shared,
) -> Vec<crate::screens::editor::EditorAction> {
    takeui::editor::editor(ui, t, ed, shared)
}

/// The capture's bars and hint in the current theme (TakeUI for both until the Coral UI skin
/// exists), over `screen`. Returns what the user asked for; `Overlay::apply` does it.
pub fn capture(
    ui: &mut egui::Ui,
    t: &Tokens,
    ov: &crate::screens::overlay::Overlay,
    shared: &Shared,
    screen: egui::Rect,
) -> Vec<crate::screens::overlay::CaptureAction> {
    takeui::capture::capture(ui, t, ov, shared, screen)
}

/// Applies the egui style for this look when it changed since the last frame (theme or light and
/// dark); true when it did.
pub fn begin_frame(ctx: &egui::Context, t: &Tokens) -> bool {
    let id = egui::Id::unique("takeshot-look");
    let look = (t.theme, t.dark);
    if ctx.data(|d| d.get_temp::<(ThemeId, bool)>(id)) == Some(look) {
        return false;
    }
    crate::ui::style::apply(ctx, t);
    ctx.data_mut(|d| d.insert_temp(id, look));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tokens::{ThemeId, tokens};

    #[test]
    fn style_is_applied_once_per_look() {
        let ctx = egui::Context::default();
        assert!(
            begin_frame(&ctx, &tokens(ThemeId::TakeUi, true)),
            "first frame"
        );
        assert!(
            !begin_frame(&ctx, &tokens(ThemeId::TakeUi, true)),
            "same look"
        );
        assert!(
            begin_frame(&ctx, &tokens(ThemeId::TakeUi, false)),
            "light now"
        );
        assert!(
            begin_frame(&ctx, &tokens(ThemeId::Coral, false)),
            "other theme"
        );
    }
}
