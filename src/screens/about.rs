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
                ui.label(
                    egui::RichText::new("Takeshot")
                        .font(font(22.0, Weight::Bold))
                        .color(t.text),
                );
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

    #[test]
    fn about_shows_the_crate_version_and_platform() {
        let a = AboutInfo::current();
        assert_eq!(a.version, env!("CARGO_PKG_VERSION"));
        assert!(!a.os.is_empty() && !a.arch.is_empty());
        assert_eq!(a.github, "https://github.com/takeflow-oficial/takeshot");
    }
}
