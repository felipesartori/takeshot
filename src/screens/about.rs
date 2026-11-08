//! The About dialog: mark, name, version, system, what Takeshot is, who makes it and its license.

/// What the About dialog shows about this build.
pub struct AboutInfo {
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub github: &'static str,
}

impl AboutInfo {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            github: "https://github.com/takeflow-oficial/takeshot",
        }
    }
}

/// What a button of the About dialog asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AboutAnswer {
    GitHub,
    Close,
}

/// The About dialog, as the design draws it: mark, name, version and system, what it is, who makes
/// it and its license, then "View on GitHub" and "Close". Esc or a click outside closes it.
pub fn dialog(
    ctx: &egui::Context,
    t: &crate::ui::tokens::Tokens,
    info: &AboutInfo,
    mark: Option<egui::TextureId>,
) -> Option<AboutAnswer> {
    use crate::ui::i18n::{Msg, t as tr, tf};
    use crate::ui::kit::{self, BtnKind, Ico};
    use crate::ui::style::{Weight, font};
    let mut answer = None;
    let modal = egui::Modal::new(egui::Id::unique("takeshot-about"))
        .frame(
            egui::Frame::new()
                .fill(t.surf)
                .stroke(egui::Stroke::new(1.0, t.border))
                .corner_radius(12)
                .inner_margin(egui::Margin::same(24)),
        )
        .show(ctx, |ui| {
            ui.set_width(340.0);
            ui.vertical_centered(|ui| {
                if let Some(id) = mark {
                    ui.image((id, egui::vec2(56.0, 56.0)));
                }
                ui.add_space(10.0);
                ui.label(crate::ui::brand::wordmark(t.theme, 22.0).color(t.text));
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {} {}",
                        tf(Msg::VersionN, &[info.version]),
                        info.os,
                        info.arch
                    ))
                    .font(font(12.0, Weight::Regular))
                    .color(t.faint),
                );
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(tr(Msg::AboutPhrase))
                        .font(font(13.0, Weight::Regular))
                        .color(t.muted),
                );
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(format!(
                        "{}  ·  {}",
                        tr(Msg::MadeBy),
                        tr(Msg::LicenseAgpl)
                    ))
                    .font(font(12.0, Weight::Medium))
                    .color(t.muted),
                );
            });
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if kit::button(
                    ui,
                    t,
                    BtnKind::Ghost,
                    Some(Ico::ExternalLink),
                    tr(Msg::ViewOnGitHub),
                    None,
                )
                .clicked()
                {
                    answer = Some(AboutAnswer::GitHub);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if kit::button(ui, t, BtnKind::Secondary, None, tr(Msg::CloseWord), None)
                        .clicked()
                    {
                        answer = Some(AboutAnswer::Close);
                    }
                });
            });
        });
    if answer.is_none() && modal.should_close() {
        answer = Some(AboutAnswer::Close);
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Texts, rectangle fills, rectangle strokes and circle fills one frame painted.
    #[derive(Default)]
    struct Shapes {
        texts: Vec<(String, egui::Rect)>,
        fills: Vec<egui::Color32>,
        strokes: Vec<egui::Color32>,
        dots: Vec<egui::Color32>,
    }

    fn shapes(clipped: &[egui::epaint::ClippedShape]) -> Shapes {
        fn walk(s: &egui::Shape, out: &mut Shapes) {
            match s {
                egui::Shape::Text(t) => out.texts.push((
                    t.galley.text().to_owned(),
                    t.galley.rect.translate(t.pos.to_vec2()),
                )),
                egui::Shape::Rect(r) => {
                    out.fills.push(r.fill);
                    out.strokes.push(r.stroke.color);
                }
                egui::Shape::Circle(c) => out.dots.push(c.fill),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut out = Shapes::default();
        clipped.iter().for_each(|c| walk(&c.shape, &mut out));
        out
    }

    /// Frames of `draw` at 800 × 600 in `t`'s look, one per event list after a first empty one;
    /// returns what the last frame painted.
    fn frames(
        t: &crate::ui::tokens::Tokens,
        events: Vec<Vec<egui::Event>>,
        mut draw: impl FnMut(&mut egui::Ui),
    ) -> Shapes {
        let ctx = egui::Context::default();
        crate::system::fonts::FontLib::load().install(&ctx);
        crate::ui::style::apply(&ctx, t);
        let mut last = Shapes::default();
        for (i, ev) in std::iter::once(Vec::new()).chain(events).enumerate() {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                time: Some(i as f64 * 0.05),
                events: ev,
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| draw(ui));
            full.textures_delta.clear();
            last = shapes(&full.shapes);
        }
        last
    }

    #[test]
    fn the_about_dialog_shows_the_theme_s_wordmark() {
        use crate::ui::tokens::{ThemeId, tokens};
        for (theme, word) in [(ThemeId::TakeUi, "Takeshot"), (ThemeId::Coral, "takeshot")] {
            let t = tokens(theme, true);
            let info = AboutInfo::current();
            let painted = frames(&t, vec![vec![]], |ui| {
                dialog(ui.ctx(), &t, &info, None);
            });
            assert!(
                painted.texts.iter().any(|(s, _)| s == word),
                "{theme:?}: {:?}",
                painted.texts
            );
        }
    }

    #[test]
    fn about_shows_the_crate_version_and_platform() {
        let a = AboutInfo::current();
        assert_eq!(a.version, env!("CARGO_PKG_VERSION"));
        assert!(!a.os.is_empty() && !a.arch.is_empty());
        assert_eq!(a.github, "https://github.com/takeflow-oficial/takeshot");
    }
}
