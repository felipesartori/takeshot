//! Component gallery: every kit component, colour token, icon and type size of the active theme,
//! rendered by `--page componentes` for visual checks against the design file.

use crate::ui::brand;
use crate::ui::i18n::{self, LangPref, Msg, t, tf};
use crate::ui::icon::{self, ALL, Ico};
use crate::ui::kit::{self, BtnKind};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::{self, ThemeId, Tokens, UiMode};
use egui::{Color32, CornerRadius, RichText, Stroke, StrokeKind, Ui, vec2};

pub struct Gallery {
    tokens: Tokens,
    theme: ThemeId,
    ui_mode: UiMode,
    lang: LangPref,
    switch_on: bool,
    switch_off: bool,
    check: bool,
    demo_mode: UiMode,
    margin: f32,
    search: String,
    ask: u8,
    mark: Option<egui::TextureHandle>,
    trays: Vec<(Color32, egui::TextureHandle)>,
}

impl Gallery {
    /// Starts with the given theme, mode and language and applies them to `ctx`.
    pub fn new(ctx: &egui::Context, theme: ThemeId, ui_mode: UiMode, lang: LangPref) -> Self {
        let mut g = Self {
            tokens: tokens::tokens(theme, true),
            theme,
            ui_mode,
            lang,
            switch_on: true,
            switch_off: false,
            check: true,
            demo_mode: UiMode::Dark,
            margin: 7.0,
            search: String::new(),
            ask: 0,
            mark: None,
            trays: Vec::new(),
        };
        g.reapply(ctx);
        g
    }

    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }

    fn reapply(&mut self, ctx: &egui::Context) {
        let dark = tokens::effective_dark(self.ui_mode, ctx.system_theme());
        self.tokens = tokens::tokens(self.theme, dark);
        crate::views::begin_frame(ctx, &self.tokens);
        i18n::set_lang(i18n::resolve(self.lang, i18n::system_locale().as_deref()));
        ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(std::sync::Arc::new(
            brand::window_icon(self.theme),
        ))));
        self.mark = Some(brand::mark_texture(ctx, self.theme, dark));
        self.trays = [(true, 0x1F1F1F), (false, 0xEDEDED)]
            .into_iter()
            .map(|(dark_bar, bar)| {
                let (rgba, w, h) = brand::tray_rgba(self.theme, dark_bar, 32);
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                let bar = Color32::from_rgb((bar >> 16) as u8, (bar >> 8) as u8, bar as u8);
                (
                    bar,
                    ctx.load_texture(
                        format!("tray-{dark_bar}"),
                        img,
                        egui::TextureOptions::NEAREST,
                    ),
                )
            })
            .collect();
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "Takeshot · {}",
            t(Msg::GalleryTitle)
        )));
    }

    /// Theme, mode and language pickers; any change, or the OS switching light/dark in System mode, re-applies the style.
    fn settings_bar(&mut self, ui: &mut Ui) {
        let tk = self.tokens.clone();
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(t(Msg::Theme)).color(tk.muted));
            changed |= kit::segmented(
                ui,
                &tk,
                &mut self.theme,
                &[
                    (ThemeId::TakeUi, t(Msg::ThemeTakeUi)),
                    (ThemeId::Coral, t(Msg::ThemeCoral)),
                ],
            )
            .changed();
            ui.add_space(12.0);
            ui.label(RichText::new(t(Msg::Mode)).color(tk.muted));
            changed |= kit::segmented(
                ui,
                &tk,
                &mut self.ui_mode,
                &[
                    (UiMode::Light, t(Msg::ModeLight)),
                    (UiMode::Dark, t(Msg::ModeDark)),
                    (UiMode::System, t(Msg::ModeSystem)),
                ],
            )
            .changed();
            ui.add_space(12.0);
            ui.label(RichText::new(t(Msg::Language)).color(tk.muted));
            changed |= kit::select(
                ui,
                &tk,
                "gallery-lang",
                &mut self.lang,
                &[
                    (LangPref::Auto, t(Msg::LangAuto)),
                    (LangPref::En, t(Msg::LangEn)),
                    (LangPref::Pt, t(Msg::LangPt)),
                ],
                140.0,
            )
            .changed();
        });
        let follow_os = tokens::effective_dark(self.ui_mode, ui.ctx().system_theme()) != tk.dark;
        if changed || follow_os {
            self.reapply(ui.ctx());
        }
    }
}

fn heading(ui: &mut Ui, tk: &Tokens, m: Msg) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(t(m))
            .font(font(15.0, Weight::Bold))
            .color(tk.text),
    );
    ui.add_space(4.0);
}

fn swatches(tk: &Tokens) -> [(&'static str, Color32); 26] {
    [
        ("shell", tk.shell),
        ("surf", tk.surf),
        ("raised", tk.raised),
        ("hover", tk.hover),
        ("press", tk.press),
        ("border", tk.border),
        ("line", tk.line),
        ("text", tk.text),
        ("muted", tk.muted),
        ("faint", tk.faint),
        ("primary", tk.primary),
        ("primary_hover", tk.primary_hover),
        ("primary_press", tk.primary_press),
        ("on_primary", tk.on_primary),
        ("accent", tk.accent),
        ("tint", tk.tint),
        ("green", tk.green),
        ("warn", tk.warn),
        ("danger", tk.danger),
        ("track", tk.track),
        ("input", tk.input),
        ("seg_sel", tk.seg_sel),
        ("tip_bg", tk.tip_bg),
        ("tip_text", tk.tip_text),
        ("canvas", tk.canvas),
        ("title_bar", tk.title_bar),
    ]
}

impl Gallery {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.settings_bar(ui);
        ui.add_space(12.0);
        let tk = &self.tokens.clone();
        let theme_name = match tk.theme {
            ThemeId::TakeUi => t(Msg::ThemeTakeUi),
            ThemeId::Coral => t(Msg::ThemeCoral),
        };
        let mode_name = t(if tk.dark {
            Msg::ModeDark
        } else {
            Msg::ModeLight
        });
        ui.horizontal(|ui| {
            if let Some(m) = &self.mark {
                ui.add(egui::Image::new(m).fit_to_exact_size(vec2(28.0, 28.0)));
            }
            ui.label(brand::wordmark(tk.theme, 26.0).color(tk.text));
            ui.add_space(16.0);
            let title = tf(Msg::GalleryHeading, &[&format!("{theme_name} {mode_name}")]);
            ui.label(
                RichText::new(title)
                    .font(font(22.0, Weight::Bold))
                    .color(tk.text),
            );
        });
        ui.add_space(8.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.columns(3, |cols| {
                self.left(&mut cols[0], tk);
                self.middle(&mut cols[1], tk);
                self.right(&mut cols[2], tk);
            });
        });
    }

    fn left(&mut self, ui: &mut Ui, tk: &Tokens) {
        heading(ui, tk, Msg::SecButtons);
        for (kind, icon, label, kbd) in [
            (BtnKind::Primary, Ico::Copy, t(Msg::Copy), Some("Ctrl+C")),
            (
                BtnKind::Secondary,
                Ico::Download,
                t(Msg::Save),
                Some("Ctrl+S"),
            ),
            (BtnKind::Ghost, Ico::FolderOpen, t(Msg::Open), None),
            (BtnKind::Danger, Ico::Trash, t(Msg::Delete), None),
        ] {
            ui.horizontal(|ui| {
                kit::button(ui, tk, kind, Some(icon), label, kbd);
                ui.add_enabled_ui(false, |ui| {
                    kit::button(ui, tk, kind, Some(icon), t(Msg::Disabled), None);
                });
            });
        }

        heading(ui, tk, Msg::SecTools);
        ui.horizontal(|ui| {
            kit::tool_button(ui, tk, Some(Ico::Select), true, "V");
            for i in [
                Ico::Pen,
                Ico::Line,
                Ico::Arrow,
                Ico::Rect,
                Ico::Ellipse,
                Ico::Text,
            ] {
                kit::tool_button(ui, tk, Some(i), false, "");
            }
            kit::tool_button(ui, tk, None, false, "N");
            ui.add_space(8.0);
            kit::icon_button(ui, tk, Ico::Undo, false, "Ctrl+Z");
            kit::icon_button(ui, tk, Ico::Pin, true, "Ctrl+T");
        });
        ui.horizontal(|ui| {
            kit::nav_pill(ui, tk, Ico::Home, t(Msg::Home), true);
            kit::nav_pill(ui, tk, Ico::Palette, t(Msg::Style), false);
            kit::nav_pill(ui, tk, Ico::Settings, t(Msg::Settings), false);
            ui.add_space(8.0);
            kit::status_chip(
                ui,
                tk,
                kit::ChipState::Ready,
                &tf(Msg::ShortcutReady, &["PrtSc"]),
            );
        });
        ui.allocate_ui(vec2(196.0, 80.0), |ui| {
            ui.vertical(|ui| {
                kit::section_item(ui, tk, Ico::Sliders, t(Msg::GeneralSection), true);
                kit::section_item(ui, tk, Ico::Keyboard, t(Msg::ShortcutsSection), false);
            });
        });
        ui.horizontal(|ui| {
            kit::status_chip(
                ui,
                tk,
                kit::ChipState::Busy,
                &tf(Msg::ShortcutBusy, &["PrtSc"]),
            );
            kit::status_chip(ui, tk, kit::ChipState::Off, t(Msg::ShortcutsOff));
        });

        heading(ui, tk, Msg::SecMenu);
        egui::Frame::new()
            .fill(tk.surf)
            .stroke(Stroke::new(1.0, tk.border))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.set_width(300.0);
                kit::menu_item(
                    ui,
                    tk,
                    Some(Ico::Scan),
                    t(Msg::CaptureArea),
                    Some("PrtSc"),
                    false,
                );
                kit::menu_item(
                    ui,
                    tk,
                    Some(Ico::Monitor),
                    t(Msg::FullScreen),
                    Some("Shift+PrtSc"),
                    false,
                );
                kit::menu_item(ui, tk, Some(Ico::Images), t(Msg::Recent), None, true);
                kit::menu_item(ui, tk, Some(Ico::Info), t(Msg::About), None, false);
                kit::menu_item(ui, tk, Some(Ico::Power), t(Msg::Quit), None, false);
            });
    }

    fn middle(&mut self, ui: &mut Ui, tk: &Tokens) {
        heading(ui, tk, Msg::SecControls);
        ui.horizontal(|ui| {
            kit::switch(ui, tk, &mut self.switch_on);
            kit::switch(ui, tk, &mut self.switch_off);
            ui.label(RichText::new(t(Msg::StartWithSystem)).color(tk.text));
        });
        kit::checkbox(ui, tk, &mut self.check, t(Msg::ShowNotice));
        kit::segmented(
            ui,
            tk,
            &mut self.demo_mode,
            &[
                (UiMode::Light, t(Msg::ModeLight)),
                (UiMode::Dark, t(Msg::ModeDark)),
                (UiMode::System, t(Msg::ModeSystem)),
            ],
        );
        ui.allocate_ui(vec2(360.0, 30.0), |ui| {
            kit::slider_row(ui, tk, t(Msg::Margin), &mut self.margin, 0.0..=20.0, "%");
        });
        kit::text_field(
            ui,
            tk,
            &mut self.search,
            t(Msg::Search),
            Some(Ico::Search),
            240.0,
        );
        ui.horizontal(|ui| {
            kit::select(
                ui,
                tk,
                "gallery-ask",
                &mut self.ask,
                &[(0, t(Msg::AskOnClose)), (1, t(Msg::Quit))],
                160.0,
            );
        });
        ui.horizontal(|ui| {
            kit::kbd(ui, tk, "Ctrl", false);
            kit::kbd(ui, tk, "Shift", false);
            kit::kbd(ui, tk, "PrtSc", false);
        });

        heading(ui, tk, Msg::SecSelection);
        ui.horizontal(|ui| {
            for selected in [true, false] {
                let (r, _) = ui.allocate_exact_size(vec2(120.0, 76.0), egui::Sense::hover());
                let r = r.shrink(6.0);
                ui.painter()
                    .rect_filled(r, CornerRadius::same(6), tk.raised);
                if selected {
                    kit::selection_marker(ui.painter(), tk, r);
                }
            }
        });

        heading(ui, tk, Msg::SecBrand);
        ui.horizontal(|ui| {
            for (bar, tex) in &self.trays {
                let (r, _) = ui.allocate_exact_size(vec2(56.0, 44.0), egui::Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(8), *bar);
                ui.painter().image(
                    tex.id(),
                    egui::Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        });

        heading(ui, tk, Msg::SecType);
        for (m, size, w) in [
            (Msg::TypeTitle, 22.0, Weight::Bold),
            (Msg::TypeSection, 15.0, Weight::Bold),
            (Msg::TypeBody, 13.0, Weight::Regular),
            (Msg::TypeLabel, 13.0, Weight::Medium),
            (Msg::TypeCaption, 12.0, Weight::Regular),
        ] {
            ui.label(RichText::new(t(m)).font(font(size, w)).color(tk.text));
        }
        ui.label(
            RichText::new("Ctrl+Shift+PrtSc")
                .font(font(11.0, Weight::Mono))
                .color(tk.text),
        );
    }

    fn right(&mut self, ui: &mut Ui, tk: &Tokens) {
        heading(ui, tk, Msg::SecColors);
        egui::Grid::new("gallery-colors")
            .num_columns(2)
            .spacing(vec2(16.0, 6.0))
            .show(ui, |ui| {
                for (i, (name, c)) in swatches(tk).into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::same(4), c);
                        ui.painter().rect_stroke(
                            r,
                            CornerRadius::same(4),
                            Stroke::new(1.0, tk.border),
                            StrokeKind::Inside,
                        );
                        ui.label(
                            RichText::new(name)
                                .font(font(11.0, Weight::Mono))
                                .color(tk.muted),
                        );
                    });
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });

        heading(ui, tk, Msg::SecIcons);
        ui.horizontal_wrapped(|ui| {
            for &i in ALL {
                ui.label(icon::text(i, 18.0, tk.text))
                    .on_hover_text(format!("{i:?}"));
            }
        });
    }
}
