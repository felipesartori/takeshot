//! Main window: capture buttons, recent captures, default style and settings.
//! It also listens for the global capture keys.

use crate::engine::render;
use crate::screens::icons::{self, Icon};
use crate::screens::overlay::color_image;
use crate::screens::widgets::{self, ACCENT, BORDER, HOVER, MUTED, PANEL, RAISED, TEXT};
use crate::system::hotkeys::{Shot, Status, default_keys, keys_from_egui, label, needs_modifier};
use crate::system::settings::QuickStyle;
use crate::system::settings::{OnClose, TrayClick};
use crate::ui::i18n::{Msg, t as tr, tf};
use crate::{Shared, Switch};
use egui::{Color32, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

#[derive(PartialEq, Clone, Copy)]
enum Page {
    Start,
    Style,
    Settings,
}

struct Recent {
    path: PathBuf,
    tex: egui::TextureHandle,
}

pub struct Home {
    page: Page,
    recents: Vec<Recent>,
    rx: Option<Receiver<Vec<(PathBuf, RgbaImage)>>>,
    last_scan: f64,
    sample: RgbaImage,
    preview: Option<egui::TextureHandle>,
    preview_dirty: bool,
    /// The capture shortcuts' state, given by the resident each frame.
    pub hotkey_status: Option<Status>,
    /// What the user asked of the resident this frame (About, Quit, a new shortcut…).
    pub requests: HomeRequests,
    /// Waiting for the keys of a new shortcut.
    capturing: Option<Shot>,
}

/// Requests from Home that the resident carries out.
#[derive(Default)]
pub struct HomeRequests {
    pub about: bool,
    pub quit: bool,
    pub rebind: Option<(Shot, String)>,
    pub open_windows_settings: bool,
}

impl Home {
    pub fn new(_ctx: &egui::Context, shared: &Shared) -> Self {
        let mut h = Self {
            page: Page::Start,
            recents: Vec::new(),
            rx: None,
            last_scan: -100.0,
            sample: crate::system::capture::sample_window(1000, 620),
            preview: None,
            preview_dirty: true,
            hotkey_status: None,
            requests: HomeRequests::default(),
            capturing: None,
        };
        h.scan(&shared.settings.save_dir, 0.0);
        h
    }

    /// Shows a page of the main window (About is a dialog the resident opens).
    pub fn open(&mut self, page: crate::system::instance::Page) {
        use crate::system::instance::Page as P;
        match page {
            P::Start => self.page = Page::Start,
            P::Style => self.page = Page::Style,
            P::Settings => self.page = Page::Settings,
            P::About => self.requests.about = true,
        }
    }

    /// The page on screen (About is a dialog, never a page).
    #[allow(dead_code)] // The TakeUI page pills read it from plan 5, Task 4.
    pub fn page(&self) -> crate::system::instance::Page {
        use crate::system::instance::Page as P;
        match self.page {
            Page::Start => P::Start,
            Page::Style => P::Style,
            Page::Settings => P::Settings,
        }
    }

    /// The gallery thumbnail of a capture, when Home has one (the tray menu shows it).
    pub fn thumbnail(&self, path: &std::path::Path) -> Option<egui::TextureId> {
        self.recents
            .iter()
            .find(|r| r.path == path)
            .map(|r| r.tex.id())
    }

    fn scan(&mut self, dir: &str, now: f64) {
        self.last_scan = now;
        let dir = PathBuf::from(dir);
        let (tx, rx) = channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let out = crate::system::recents::recent_captures(&dir, 8)
                .into_iter()
                .filter_map(|p| {
                    let img = image::open(&p).ok()?.to_rgba8();
                    Some((
                        p,
                        image::imageops::thumbnail(
                            &img,
                            320,
                            (320.0 * img.height() as f32 / img.width().max(1) as f32) as u32,
                        ),
                    ))
                })
                .collect();
            let _ = tx.send(out);
        });
    }

    pub fn ui(&mut self, ui: &mut Ui, _frame: &eframe::Frame, shared: &mut Shared) -> Switch {
        let ctx = ui.ctx().clone();
        let now = ui.input(|i| i.time);
        if let Some(rx) = &self.rx
            && let Ok(list) = rx.try_recv()
        {
            self.recents = list
                .into_iter()
                .enumerate()
                .map(|(i, (path, img))| Recent {
                    path,
                    tex: ctx.load_texture(
                        format!("recent{i}"),
                        color_image(&img),
                        egui::TextureOptions::LINEAR,
                    ),
                })
                .collect();
            self.rx = None;
        }
        if self.page == Page::Start && now - self.last_scan > 6.0 && self.rx.is_none() {
            self.scan(&shared.settings.save_dir.clone(), now);
        }
        shared.logo.sync(&ctx, &shared.settings.watermark);

        // dropped files and pasted pictures open in the editor
        let dropped: Vec<PathBuf> = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if let Some(p) = dropped.first() {
            let _ = crate::system::actions::spawn_self(&["--edit", &p.to_string_lossy()]);
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V))
            || ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))))
        {
            open_clipboard(shared);
        }

        egui::Panel::left("nav")
            .exact_size(214.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(17, 17, 20))
                    .inner_margin(egui::Margin::symmetric(14, 18))
                    .stroke(Stroke::new(1.0, BORDER)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(r, 7.0, ACCENT);
                    icons::paint(ui.painter(), r.shrink(5.0), Icon::Full, Color32::WHITE);
                    ui.label(
                        RichText::new("Takeshot")
                            .size(17.0)
                            .family(egui::FontFamily::Name("bold".into()))
                            .color(Color32::WHITE),
                    );
                });
                ui.add_space(22.0);
                for (p, label, icon) in [
                    (Page::Start, "Início", Icon::Full),
                    (Page::Style, "Estilo e marca d'água", Icon::Edit),
                    (Page::Settings, "Ajustes", Icon::Folder),
                ] {
                    if nav_item(ui, icon, label, self.page == p).clicked() {
                        self.page = p;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    if ui
                        .add(egui::Button::new(crate::ui::icon::text(
                            crate::ui::icon::Ico::Info,
                            15.0,
                            MUTED,
                        )))
                        .on_hover_text(tr(Msg::About))
                        .clicked()
                    {
                        self.requests.about = true;
                    }
                    match hotkey_text(self.hotkey_status.as_ref(), &shared.settings) {
                        Ok(s) => ui.label(RichText::new(s).size(11.5).color(MUTED)),
                        Err(e) => ui.label(
                            RichText::new(e)
                                .size(11.5)
                                .color(Color32::from_rgb(230, 150, 90)),
                        ),
                    };
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(11.0)
                            .color(Color32::from_gray(90)),
                    );
                });
            });

        let mut switch = Switch::None;
        egui::CentralPanel::no_frame()
            .frame(
                egui::Frame::new()
                    .fill(widgets::BG)
                    .inner_margin(egui::Margin::symmetric(30, 26)),
            )
            .show(ui, |ui| match self.page {
                Page::Start => switch = self.start_page(ui, shared),
                Page::Style => self.style_page(ui, shared),
                Page::Settings => self.settings_page(ui, shared),
            });
        switch
    }

    fn start_page(&mut self, ui: &mut Ui, shared: &mut Shared) -> Switch {
        let ctx = ui.ctx().clone();
        ui.label(
            RichText::new("Capturar")
                .size(24.0)
                .family(egui::FontFamily::Name("bold".into()))
                .color(Color32::WHITE),
        );
        ui.label(
            RichText::new(format!(
                "Pressione {} em qualquer lugar, ou escolha abaixo.",
                label(&shared.settings.hotkey_area)
            ))
            .color(MUTED),
        );
        ui.add_space(14.0);
        let mut action: Option<Vec<&str>> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
            let (area_keys, full_keys) = (
                label(&shared.settings.hotkey_area),
                label(&shared.settings.hotkey_full),
            );
            if big_card(ui, Icon::Select, "Área", &area_keys, true).clicked() {
                action = Some(vec!["--capture", "--wait"]);
            }
            if big_card(ui, Icon::Full, "Tela inteira", &full_keys, false).clicked() {
                action = Some(vec!["--capture", "--full", "--wait"]);
            }
            if big_card(ui, Icon::Timer, "Em 3 segundos", "temporizador", false).clicked() {
                action = Some(vec!["--capture", "--delay", "3"]);
            }
            if big_card(
                ui,
                Icon::Open,
                "Abrir imagem",
                "ou arraste aqui · Ctrl+V",
                false,
            )
            .clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("Imagem", &["png", "jpg", "jpeg", "webp", "bmp"])
                    .pick_file()
            {
                let _ = crate::system::actions::spawn_self(&["--edit", &p.to_string_lossy()]);
            }
        });
        if let Some(args) = action {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            if let Err(e) = crate::system::actions::spawn_self(&args) {
                shared.toast(format!("Não consegui iniciar a captura: {e}"));
            }
        }

        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Estilo aplicado")
                    .size(15.0)
                    .color(Color32::WHITE)
                    .strong(),
            );
            ui.label(RichText::new("· vale para copiar e salvar direto da captura").color(MUTED));
        });
        ui.add_space(6.0);
        let avail_w = ui.available_width();
        ui.horizontal_top(|ui| {
            let pw = (avail_w * 0.56).clamp(320.0, 620.0);
            self.paint_preview(ui, shared, vec2(pw, pw * 0.62));
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.set_width((avail_w - pw - 24.0).max(200.0));
                ui.label(
                    RichText::new("Recentes")
                        .size(15.0)
                        .color(Color32::WHITE)
                        .strong(),
                );
                if self.recents.is_empty() {
                    ui.label(
                        RichText::new("As capturas salvas na sua pasta aparecem aqui.")
                            .color(MUTED),
                    );
                }
                let mut open: Option<PathBuf> = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                    for r in &self.recents {
                        let sz = r.tex.size_vec2();
                        let w = 132.0;
                        let h = (w * sz.y / sz.x).min(96.0);
                        let (rect, resp) = ui.allocate_exact_size(vec2(w, 96.0), Sense::click());
                        ui.painter().rect_filled(rect, 8.0, RAISED);
                        let img_r = Rect::from_center_size(rect.center(), vec2(h * sz.x / sz.y, h));
                        ui.painter().image(
                            r.tex.id(),
                            img_r,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                        if resp.hovered() {
                            ui.painter().rect_stroke(
                                rect,
                                8.0,
                                Stroke::new(1.5, ACCENT),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let name = r
                            .path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        if resp
                            .on_hover_text(format!("{name}\nClique para editar"))
                            .clicked()
                        {
                            open = Some(r.path.clone());
                        }
                    }
                });
                if let Some(p) = open {
                    let _ = crate::system::actions::spawn_self(&["--edit", &p.to_string_lossy()]);
                }
                ui.add_space(6.0);
                if ui.button("Abrir pasta das capturas").clicked() {
                    crate::system::actions::open_folder(std::path::Path::new(
                        &shared.settings.save_dir,
                    ));
                }
            });
        });
        Switch::None
    }

    fn paint_preview(&mut self, ui: &mut Ui, shared: &Shared, size: egui::Vec2) {
        if self.preview_dirty || self.preview.is_none() {
            let (img, _) = render::compose(
                &self.sample,
                &shared.settings.style,
                &shared.settings.watermark,
                shared.logo.image.as_ref(),
                &shared.fonts,
                1.0,
            );
            let small = image::imageops::resize(
                &img,
                (img.width() / 2).max(1),
                (img.height() / 2).max(1),
                image::imageops::FilterType::Triangle,
            );
            match &mut self.preview {
                Some(t) => t.set(color_image(&small), egui::TextureOptions::LINEAR),
                None => {
                    self.preview = Some(ui.ctx().load_texture(
                        "style-preview",
                        color_image(&small),
                        egui::TextureOptions::LINEAR,
                    ))
                }
            }
            self.preview_dirty = false;
        }
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        ui.painter().rect_filled(rect, 12.0, PANEL);
        ui.painter().rect_stroke(
            rect,
            12.0,
            Stroke::new(1.0, BORDER),
            egui::StrokeKind::Inside,
        );
        if let Some(t) = &self.preview {
            let sz = t.size_vec2();
            let k = ((rect.width() - 24.0) / sz.x).min((rect.height() - 24.0) / sz.y);
            let r = Rect::from_center_size(rect.center(), sz * k);
            ui.painter().image(
                t.id(),
                r,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    fn style_page(&mut self, ui: &mut Ui, shared: &mut Shared) {
        ui.label(
            RichText::new("Estilo e marca d'água")
                .size(24.0)
                .family(egui::FontFamily::Name("bold".into()))
                .color(Color32::WHITE),
        );
        ui.label(
            RichText::new("Padrão para novas capturas. No editor você ajusta cada uma.")
                .color(MUTED),
        );
        ui.add_space(12.0);
        let avail = ui.available_size();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(320.0);
                egui::ScrollArea::vertical()
                    .max_height(avail.y)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if widgets::style_panel(
                            ui,
                            &mut shared.settings.style,
                            &mut shared.settings.watermark,
                            &mut shared.logo,
                        ) {
                            self.preview_dirty = true;
                            shared.settings_dirty = true;
                        }
                        shared.logo.sync(ui.ctx(), &shared.settings.watermark);
                    });
            });
            ui.add_space(16.0);
            let w = (avail.x - 340.0).max(260.0);
            self.paint_preview(ui, shared, vec2(w, (w * 0.66).min(avail.y)));
        });
    }

    fn settings_page(&mut self, ui: &mut Ui, shared: &mut Shared) {
        ui.label(
            RichText::new("Ajustes")
                .size(24.0)
                .family(egui::FontFamily::Name("bold".into()))
                .color(Color32::WHITE),
        );
        ui.add_space(10.0);
        self.general_section(ui, shared);
        let s = &mut shared.settings;
        let before = serde_json::to_string(s).ok();
        widgets::section(ui, "Pasta das capturas");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut s.save_dir).desired_width(420.0));
            if ui.button("Escolher…").clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .set_directory(&s.save_dir)
                    .pick_folder()
            {
                s.save_dir = p.to_string_lossy().into_owned();
            }
            if ui.button("Abrir").clicked() {
                crate::system::actions::open_folder(std::path::Path::new(&s.save_dir));
            }
        });
        ui.checkbox(
            &mut s.auto_save,
            "Salvar direto na pasta, sem perguntar o nome",
        );
        ui.label(
            RichText::new("Sempre em PNG sem perdas, na resolução original da tela.")
                .color(MUTED)
                .size(12.0),
        );

        widgets::section(ui, "Captura");
        ui.checkbox(&mut s.keep_selection, "Lembrar a última área selecionada");
        ui.checkbox(
            &mut s.open_editor_after_full,
            "Tela inteira abre no editor (desligado: salva na hora)",
        );
        ui.checkbox(&mut s.show_bubbles, "Mostrar aviso ao copiar e salvar");
        ui.label("Ao copiar ou salvar direto da captura, aplicar:");
        ui.horizontal(|ui| {
            for (q, l) in [
                (QuickStyle::Full, "Estilo completo"),
                (QuickStyle::CornersOnly, "Só cantos arredondados"),
                (QuickStyle::Plain, "Nada (imagem pura)"),
            ] {
                if widgets::chip(ui, s.quick_style == q, l).clicked() {
                    s.quick_style = q;
                }
            }
        });

        widgets::section(ui, "Atalhos");
        ui.checkbox(&mut s.hotkeys, "Atalhos globais (vale ao reabrir o app)");
        self.shortcut_rows(ui, s);
        egui::Grid::new("keys").num_columns(2).spacing(vec2(24.0, 6.0)).show(ui, |ui| {
            for (k, d) in [
                ("Shift (arrastando)", "Linha reta a 45°, quadrado, círculo perfeito"),
                ("Alt (arrastando)", "Desenhar a partir do centro"),
                ("P L A R O M T B N", "Caneta, linha, seta, retângulo, círculo, marcador, texto, borrar, numeração"),
                ("Roda do mouse", "Espessura do traço / tamanho do texto"),
                ("Ctrl+C / Ctrl+S", "Copiar / salvar"),
                ("Enter", "Abrir no editor"),
                ("Ctrl+Z / Ctrl+Y", "Desfazer / refazer"),
                ("Setas / Shift+setas", "Mover / redimensionar a seleção"),
                ("C (antes de selecionar)", "Copiar a cor sob o cursor"),
            ] {
                ui.label(RichText::new(k).monospace().color(TEXT));
                ui.label(RichText::new(d).color(MUTED));
                ui.end_row();
            }
        });
        if serde_json::to_string(s).ok() != before {
            shared.settings_dirty = true;
        }
    }

    /// Start with the system, the X, the tray icon click, and Quit on Linux (no tray there for sure).
    fn general_section(&mut self, ui: &mut Ui, shared: &mut Shared) {
        widgets::section(ui, tr(Msg::GeneralSection));
        let mut on = crate::system::autostart::autostart_enabled();
        if ui.checkbox(&mut on, tr(Msg::StartWithSystem)).changed()
            && let Err(e) = crate::system::autostart::set_autostart(on)
        {
            shared.toast(e);
        }
        let s = &mut shared.settings;
        let (close, click) = (s.on_close, s.tray_click);
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(Msg::OnCloseLabel)).color(TEXT));
            for (v, m) in [
                (OnClose::Ask, Msg::AskOnClose),
                (OnClose::Minimize, Msg::MinimizeToTray),
                (OnClose::Quit, Msg::Quit),
            ] {
                if widgets::chip(ui, s.on_close == v, tr(m)).clicked() {
                    s.on_close = v;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr(Msg::TrayClickLabel)).color(TEXT));
            for (v, m) in [
                (TrayClick::OpenHome, Msg::OpenHome),
                (TrayClick::CaptureArea, Msg::CaptureArea),
            ] {
                if widgets::chip(ui, s.tray_click == v, tr(m)).clicked() {
                    s.tray_click = v;
                }
            }
        });
        if (close, click) != (s.on_close, s.tray_click) {
            shared.settings_dirty = true;
        }
        if cfg!(target_os = "linux") && ui.button(tr(Msg::QuitTakeshot)).clicked() {
            self.requests.quit = true;
        }
    }

    /// The two capture shortcuts with Change and Restore default, the PrtSc conflict help, the
    /// priority switch (Windows) and the Wayland note.
    fn shortcut_rows(&mut self, ui: &mut Ui, s: &mut crate::system::settings::Settings) {
        if let Some(shot) = self.capturing {
            ui.label(RichText::new(tr(Msg::PressKeys)).color(ACCENT));
            let pressed = ui.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
            });
            // Esc cancels; a key that needs Ctrl or Alt keeps the wait going.
            if let Some((key, mods)) = pressed
                && (key == egui::Key::Escape || !needs_modifier(key, mods))
            {
                self.capturing = None;
                if let Some(keys) = keys_from_egui(key, mods) {
                    self.requests.rebind = Some((shot, keys));
                }
            }
        }
        for (shot, keys, m) in [
            (Shot::Area, s.hotkey_area.clone(), Msg::CaptureArea),
            (Shot::Full, s.hotkey_full.clone(), Msg::FullScreen),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label(&keys)).monospace().color(TEXT));
                ui.label(RichText::new(tr(m)).color(MUTED));
                if ui.button(tr(Msg::ChangeKeys)).clicked() {
                    self.capturing = Some(shot);
                }
                if keys != default_keys(shot) && ui.button(tr(Msg::RestoreDefault)).clicked() {
                    self.requests.rebind = Some((shot, default_keys(shot).to_string()));
                }
            });
        }
        let prtsc_busy = matches!(&self.hotkey_status, Some(Status::Busy { keys }) if keys.iter().any(|k| k.contains("PrtSc")));
        if cfg!(windows) {
            if prtsc_busy {
                ui.label(
                    RichText::new(tr(Msg::PrtScConflictHelp))
                        .color(Color32::from_rgb(230, 150, 90)),
                );
                if ui.button(tr(Msg::OpenWindowsSettings)).clicked() {
                    self.requests.open_windows_settings = true;
                }
            }
            ui.checkbox(&mut s.prtsc_priority, tr(Msg::PrtScPriority));
            ui.label(
                RichText::new(tr(Msg::PrtScPriorityHint))
                    .color(MUTED)
                    .size(12.0),
            );
        }
        if cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some() {
            let exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "takeshot".into());
            for args in ["--capture", "--capture --full"] {
                ui.label(
                    RichText::new(tf(Msg::WaylandHotkeyHelp, &[&format!("{exe} {args}")]))
                        .color(MUTED),
                );
            }
        }
    }
}

fn nav_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 8.0, RAISED);
        p.rect_filled(
            Rect::from_min_size(rect.min + vec2(0.0, 9.0), vec2(3.0, 18.0)),
            2.0,
            ACCENT,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 8.0, Color32::from_rgb(26, 26, 30));
    }
    let col = if selected {
        Color32::WHITE
    } else {
        Color32::from_gray(175)
    };
    icons::paint(
        p,
        Rect::from_min_size(rect.min + vec2(12.0, 9.0), vec2(18.0, 18.0)),
        icon,
        col,
    );
    p.text(
        rect.min + vec2(40.0, 18.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.5),
        col,
    );
    resp
}

fn big_card(ui: &mut Ui, icon: Icon, title: &str, sub: &str, primary: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(178.0, 104.0), Sense::click());
    let p = ui.painter();
    let fill = if primary {
        Color32::from_rgb(38, 22, 18)
    } else if resp.hovered() {
        HOVER
    } else {
        RAISED
    };
    p.rect_filled(rect, 12.0, fill);
    let stroke = if primary {
        Stroke::new(1.0, ACCENT.gamma_multiply(0.7))
    } else if resp.hovered() {
        Stroke::new(1.0, Color32::from_gray(80))
    } else {
        Stroke::new(1.0, BORDER)
    };
    p.rect_stroke(rect, 12.0, stroke, egui::StrokeKind::Inside);
    let ib = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(34.0, 34.0));
    p.rect_filled(
        ib,
        9.0,
        if primary {
            ACCENT
        } else {
            Color32::from_rgb(44, 44, 50)
        },
    );
    icons::paint(p, ib.shrink(7.0), icon, Color32::WHITE);
    p.text(
        rect.min + vec2(14.0, 62.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::new(14.5, egui::FontFamily::Name("bold".into())),
        Color32::WHITE,
    );
    p.text(
        rect.min + vec2(14.0, 82.0),
        egui::Align2::LEFT_TOP,
        sub,
        egui::FontId::proportional(11.5),
        MUTED,
    );
    resp
}

fn open_clipboard(shared: &mut Shared) {
    match crate::system::actions::clipboard_image() {
        Some(img) => {
            if let Some(p) = crate::system::actions::temp_png(&img) {
                let _ = crate::system::actions::spawn_self(&["--edit", &p.to_string_lossy()]);
            }
        }
        None => shared.toast("Não há imagem na área de transferência".into()),
    }
}

/// The status line under the navigation: which keys capture, or why they do not.
fn hotkey_text(
    status: Option<&Status>,
    settings: &crate::system::settings::Settings,
) -> Result<String, String> {
    let ready = || {
        Ok(format!(
            "{} captura · {} tela inteira",
            label(&settings.hotkey_area),
            label(&settings.hotkey_full)
        ))
    };
    match status {
        None if settings.hotkeys => ready(),
        None | Some(Status::Off) => Err("Atalhos globais desligados".into()),
        Some(Status::Ready) => ready(),
        Some(Status::Busy { keys }) => {
            Err(format!("{} em uso por outro programa", keys.join(" e ")))
        }
        Some(Status::Unavailable(e)) => Err(format!("Atalhos globais indisponíveis: {e}")),
    }
}
