//! Main window state and behaviour, shared by both themes: the page on screen, the captures
//! library (filter, search, selection, thumbnails), what the user asked for (`MainAction`) and
//! carrying it out. The skins draw it; the style and settings pages still draw here until their
//! TakeUI pages land.

use crate::Shared;
use crate::engine::render;
use crate::screens::overlay::color_image;
use crate::screens::widgets::{self, ACCENT, MUTED, TEXT};
use crate::system::hotkeys::{Shot, Status, default_keys, keys_from_egui, label, needs_modifier};
use crate::system::library::{self, Entry, Period, Thumbs};
use crate::system::settings::QuickStyle;
use crate::system::settings::{OnClose, TrayClick};
use crate::ui::i18n::{Msg, t as tr, tf};
use crate::ui::kit::{CardThumb, WindowControl};
use crate::ui::tokens::{ThemeId, Tokens, UiMode};
use egui::{Color32, RichText, Ui, vec2};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

#[derive(PartialEq, Clone, Copy)]
enum Page {
    Start,
    Style,
    Settings,
}

/// What the user asked for in the main window; `Home::apply` carries it out.
#[derive(Clone, PartialEq, Debug)]
pub enum MainAction {
    Page(crate::system::instance::Page),
    Capture(Shot),
    CaptureIn3s,
    OpenImage,
    OpenFolder,
    ChooseFolder,
    Filter(Period),
    Search(String),
    /// A picture pasted into the window (outside a text field): it opens in the editor.
    Paste,
    /// Style's "Restore default".
    RestoreStyle,
    /// Pick the watermark's logo file.
    ChooseLogo,
    Select(PathBuf),
    Copy(PathBuf),
    Edit(PathBuf),
    ShowInFolder(PathBuf),
    Pin(PathBuf),
    Trash(PathBuf),
    SetMode(UiMode),
    About,
    Window(WindowControl),
}

/// A capture's thumbnail in the cache.
enum Thumbnail {
    Pending,
    Ready {
        tex: egui::TextureHandle,
        size: [u32; 2],
        used: u64,
    },
    Failed,
}

/// Thumbnails kept as textures; the least recently drawn go first.
const THUMB_CACHE: usize = 300;
/// How often the folder is read again while Home is on screen.
const RESCAN_SECONDS: f64 = 5.0;

pub struct Home {
    page: Page,
    entries: Vec<Entry>,
    /// The positions in `entries` the filter and the search let through, and the filters' counts,
    /// worked out when the list, the filter or the search changes.
    shown: Vec<usize>,
    counts: [usize; 3],
    filter: Period,
    query: String,
    selected: Option<PathBuf>,
    folder_missing: bool,
    thumbs: Thumbs,
    textures: HashMap<PathBuf, Thumbnail>,
    /// The cards drawn in the last frame; a waiting thumbnail not among them is dropped.
    drawn: std::collections::HashSet<PathBuf>,
    frame: u64,
    scan_rx: Option<Receiver<(bool, Vec<Entry>)>>,
    last_scan: f64,
    mark: Option<((ThemeId, bool), egui::TextureHandle)>,
    sample: RgbaImage,
    preview: Option<egui::TextureHandle>,
    /// What the preview was drawn from (style and watermark), to draw it again only on a change.
    preview_key: Option<String>,
    #[cfg(test)]
    renders: usize,
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
    pub fn new(ctx: &egui::Context, shared: &Shared) -> Self {
        let mut h = Self::empty(ctx);
        h.scan(ctx, shared.captures_dir(), 0.0);
        h
    }

    fn empty(ctx: &egui::Context) -> Self {
        Self {
            page: Page::Start,
            entries: Vec::new(),
            shown: Vec::new(),
            counts: [0; 3],
            filter: Period::All,
            query: String::new(),
            selected: None,
            folder_missing: false,
            thumbs: Thumbs::new(ctx),
            textures: HashMap::new(),
            drawn: std::collections::HashSet::new(),
            frame: 0,
            scan_rx: None,
            last_scan: -100.0,
            mark: None,
            sample: crate::system::capture::sample_window(1000, 620),
            preview: None,
            preview_key: None,
            #[cfg(test)]
            renders: 0,
            hotkey_status: None,
            requests: HomeRequests::default(),
            capturing: None,
        }
    }

    /// Home over these entries, never reading a folder, with a `Shared` for tests.
    #[cfg(test)]
    pub fn for_tests(entries: Vec<Entry>) -> (Self, Shared) {
        let mut h = Self::empty(&egui::Context::default());
        h.last_scan = f64::INFINITY;
        h.set_entries(entries);
        (h, Shared::for_tests())
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
    pub fn page(&self) -> crate::system::instance::Page {
        use crate::system::instance::Page as P;
        match self.page {
            Page::Start => P::Start,
            Page::Style => P::Style,
            Page::Settings => P::Settings,
        }
    }

    pub fn filter(&self) -> Period {
        self.filter
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// How many captures each filter shows: all, today, this week.
    pub fn counts(&self) -> [usize; 3] {
        self.counts
    }

    /// The captures on screen (positions for `entry`), newest first.
    pub fn shown(&self) -> &[usize] {
        &self.shown
    }

    pub fn entry(&self, i: usize) -> &Entry {
        &self.entries[i]
    }

    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    pub fn folder_missing(&self) -> bool {
        self.folder_missing
    }

    pub fn has_captures(&self) -> bool {
        !self.entries.is_empty()
    }

    /// A card's picture and the capture's size in pixels; the first call asks for the thumbnail.
    pub fn card_thumb(&mut self, path: &Path) -> (CardThumb, Option<[u32; 2]>) {
        let frame = self.frame;
        self.drawn.insert(path.to_path_buf());
        match self.textures.get_mut(path) {
            Some(Thumbnail::Ready { tex, size, used }) => {
                *used = frame;
                (CardThumb::Ready(tex.id(), tex.size_vec2()), Some(*size))
            }
            Some(Thumbnail::Pending) => (CardThumb::Loading, None),
            Some(Thumbnail::Failed) => (CardThumb::Failed, None),
            None => {
                self.textures.insert(path.to_path_buf(), Thumbnail::Pending);
                self.thumbs.request(path.to_path_buf());
                (CardThumb::Loading, None)
            }
        }
    }

    /// How many thumbnails were asked for (tests).
    #[cfg(test)]
    pub fn thumb_requests(&self) -> usize {
        self.textures.len()
    }

    /// The gallery thumbnail of a capture, when it is loaded (the tray menu shows it).
    pub fn thumbnail(&self, path: &Path) -> Option<egui::TextureId> {
        match self.textures.get(path) {
            Some(Thumbnail::Ready { tex, .. }) => Some(tex.id()),
            _ => None,
        }
    }

    /// The theme's mark for the title bar, drawn once per look.
    pub fn mark(&mut self, ctx: &egui::Context, t: &Tokens) -> egui::TextureId {
        let look = (t.theme, t.dark);
        match &self.mark {
            Some((l, tex)) if *l == look => tex.id(),
            _ => {
                let tex = crate::ui::brand::mark_texture(ctx, t.theme, t.dark);
                let id = tex.id();
                self.mark = Some((look, tex));
                id
            }
        }
    }

    /// The live style preview: the sample picture through the current style and watermark, drawn
    /// again only when one of them (or the logo) changed.
    pub fn style_preview(
        &mut self,
        ctx: &egui::Context,
        shared: &Shared,
    ) -> Option<(egui::TextureId, egui::Vec2)> {
        let s = &shared.settings;
        let key = serde_json::to_string(&(&s.style, &s.watermark, shared.logo.image.is_some()))
            .unwrap_or_default();
        if self.preview.is_none() || self.preview_key.as_deref() != Some(key.as_str()) {
            let (img, _) = render::compose(
                &self.sample,
                &s.style,
                &s.watermark,
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
                    self.preview = Some(ctx.load_texture(
                        "style-preview",
                        color_image(&small),
                        egui::TextureOptions::LINEAR,
                    ))
                }
            }
            self.preview_key = Some(key);
            #[cfg(test)]
            {
                self.renders += 1;
            }
        }
        self.preview.as_ref().map(|t| (t.id(), t.size_vec2()))
    }

    #[cfg(test)]
    pub fn preview_renders(&self) -> usize {
        self.renders
    }

    fn set_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
        let present: std::collections::HashSet<&Path> =
            self.entries.iter().map(|e| e.path.as_path()).collect();
        self.textures.retain(|p, _| present.contains(p.as_path()));
        if self
            .selected
            .as_deref()
            .is_some_and(|s| !present.contains(s))
        {
            self.selected = None;
        }
        self.refilter();
    }

    fn refilter(&mut self) {
        let now = chrono::Local::now();
        self.shown = library::visible(&self.entries, self.filter, &self.query, now);
        self.counts = library::counts(&self.entries, now);
    }

    fn scan(&mut self, ctx: &egui::Context, dir: &str, now: f64) {
        self.last_scan = now;
        let dir = PathBuf::from(dir);
        let (tx, rx) = channel();
        self.scan_rx = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send((dir.is_dir(), library::scan(&dir)));
            ctx.request_repaint();
        });
    }

    /// Drops the least recently drawn thumbnails past the cache size; they are asked again when
    /// they come back on screen.
    fn evict(&mut self) {
        let ready = |t: &Thumbnail| matches!(t, Thumbnail::Ready { .. });
        let n = self.textures.values().filter(|t| ready(t)).count();
        if n <= THUMB_CACHE {
            return;
        }
        let mut used: Vec<(u64, PathBuf)> = self
            .textures
            .iter()
            .filter_map(|(p, t)| match t {
                Thumbnail::Ready { used, .. } => Some((*used, p.clone())),
                _ => None,
            })
            .collect();
        used.sort();
        for (_, p) in used.into_iter().take(n - THUMB_CACHE) {
            self.textures.remove(&p);
        }
    }

    /// Once per frame, before the skin draws: the folder's new listing, finished thumbnails,
    /// dropped files, pasted pictures and Ctrl+O.
    pub fn begin(&mut self, ui: &Ui, shared: &mut Shared) -> Vec<MainAction> {
        let ctx = ui.ctx().clone();
        self.frame += 1;
        let now = ui.input(|i| i.time);
        if let Some(rx) = &self.scan_rx
            && let Ok((exists, list)) = rx.try_recv()
        {
            self.scan_rx = None;
            self.folder_missing = !exists;
            self.set_entries(list);
        }
        if self.page == Page::Start {
            if self.scan_rx.is_none() && now - self.last_scan > RESCAN_SECONDS {
                self.scan(&ctx, shared.captures_dir(), now);
            }
            // The folder is read again while Home is on screen, input or not.
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(RESCAN_SECONDS));
        }
        // Thumbnails still waiting for cards that left the screen are not made; they are asked
        // again when the cards come back.
        self.thumbs.keep(&self.drawn);
        let drawn = std::mem::take(&mut self.drawn);
        self.textures
            .retain(|p, t| !matches!(t, Thumbnail::Pending) || drawn.contains(p));
        for (path, result) in self.thumbs.poll() {
            if !self.textures.contains_key(&path) {
                continue;
            }
            let thumb = match result {
                Ok((img, size)) => Thumbnail::Ready {
                    tex: ctx.load_texture(
                        format!("thumb-{}", path.display()),
                        color_image(&img),
                        egui::TextureOptions::LINEAR,
                    ),
                    size,
                    used: self.frame,
                },
                Err(_) => Thumbnail::Failed,
            };
            self.textures.insert(path, thumb);
        }
        self.evict();
        shared.logo.sync(&ctx, &shared.settings.watermark);

        let mut acts = Vec::new();
        let dropped: Vec<PathBuf> = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if let Some(p) = dropped.into_iter().next() {
            acts.push(MainAction::Edit(p));
        }
        // A text field keeps its own paste and shortcuts.
        if !ctx.text_edit_focused() {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V))
                || ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))))
            {
                acts.push(MainAction::Paste);
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
                acts.push(MainAction::OpenImage);
            }
        }
        acts
    }

    /// Carries out what the user asked for. Resident matters (About) go to `requests`.
    pub fn apply(&mut self, a: MainAction, ctx: &egui::Context, shared: &mut Shared) {
        use crate::system::actions;
        match a {
            MainAction::Page(p) => self.open(p),
            MainAction::Capture(Shot::Area) => self.capture(ctx, shared, &["--capture", "--wait"]),
            MainAction::Capture(Shot::Full) => {
                self.capture(ctx, shared, &["--capture", "--full", "--wait"])
            }
            MainAction::CaptureIn3s => self.capture(ctx, shared, &["--capture", "--delay", "3"]),
            MainAction::OpenImage => {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter(tr(Msg::OpenImage), &["png", "jpg", "jpeg", "webp", "bmp"])
                    .pick_file()
                {
                    self.apply(MainAction::Edit(p), ctx, shared);
                }
            }
            MainAction::OpenFolder => actions::open_folder(Path::new(shared.captures_dir())),
            MainAction::Paste => open_clipboard(shared),
            MainAction::ChooseFolder => {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    shared.settings.save_dir = d.to_string_lossy().into_owned();
                    shared.settings_dirty = true;
                    self.last_scan = f64::NEG_INFINITY;
                }
            }
            MainAction::Filter(p) => {
                self.filter = p;
                self.refilter();
            }
            MainAction::Search(q) => {
                self.query = q;
                self.refilter();
            }
            MainAction::Select(p) => self.selected = Some(p),
            MainAction::RestoreStyle => {
                crate::system::settings::restore_default_style(&mut shared.settings);
                shared.settings_dirty = true;
            }
            MainAction::ChooseLogo => {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Logo", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
                {
                    shared.settings.watermark.logo_path = Some(p.to_string_lossy().into_owned());
                    shared.settings_dirty = true;
                }
            }
            MainAction::Copy(p) => match image::open(&p) {
                Ok(img) => match actions::copy_image(&img.to_rgba8()) {
                    Ok(()) => shared.toast(tr(Msg::Copied).to_string()),
                    Err(e) => shared.toast(e),
                },
                Err(e) => shared.toast(e.to_string()),
            },
            MainAction::Edit(p) => {
                if let Err(e) = actions::spawn_self(&["--edit", &p.to_string_lossy()]) {
                    shared.toast(e);
                }
            }
            MainAction::ShowInFolder(p) => actions::show_in_folder(&p),
            // `--pin` deletes the file it gets, so it gets a copy.
            MainAction::Pin(p) => match image::open(&p) {
                Ok(img) => match actions::temp_png(&img.to_rgba8()) {
                    Some(tmp) => {
                        if let Err(e) = actions::spawn_self(&["--pin", &tmp.to_string_lossy()]) {
                            shared.toast(e);
                        }
                    }
                    None => shared.toast(tf(Msg::CannotPin, &[&p.to_string_lossy()])),
                },
                Err(e) => shared.toast(e.to_string()),
            },
            MainAction::Trash(p) => match library::move_to_trash(&p) {
                Ok(()) => {
                    shared.toast(tr(Msg::MovedToTrash).to_string());
                    let rest = self
                        .entries
                        .iter()
                        .filter(|e| e.path != p)
                        .cloned()
                        .collect();
                    self.set_entries(rest);
                }
                Err(e) => shared.toast(e),
            },
            MainAction::SetMode(m) => {
                shared.settings.ui_mode = m;
                shared.settings_dirty = true;
            }
            MainAction::About => self.requests.about = true,
            MainAction::Window(c) => crate::ui::chrome::act(ctx, c),
        }
    }

    fn capture(&mut self, ctx: &egui::Context, shared: &mut Shared, args: &[&str]) {
        // The new capture shows as soon as the window is back.
        self.last_scan = f64::NEG_INFINITY;
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        if let Err(e) = crate::system::actions::spawn_self(args) {
            shared.toast(e);
        }
    }

    /// The settings page as it was, until its TakeUI page lands.
    pub fn legacy_page(&mut self, ui: &mut Ui, shared: &mut Shared) {
        if self.page == Page::Settings {
            self.settings_page(ui, shared);
        }
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
pub(crate) fn hotkey_text(
    status: Option<&Status>,
    settings: &crate::system::settings::Settings,
) -> Result<String, String> {
    let ready = || {
        Ok(tf(
            Msg::HotkeysReady,
            &[&label(&settings.hotkey_area), &label(&settings.hotkey_full)],
        ))
    };
    match status {
        None if settings.hotkeys => ready(),
        None | Some(Status::Off) => Err(tr(Msg::HotkeysOff).to_string()),
        Some(Status::Ready) => ready(),
        Some(Status::Busy { keys }) => Err(tf(Msg::HotkeysBusy, &[&keys.join(" + ")])),
        Some(Status::Unavailable(e)) => Err(tf(Msg::HotkeysUnavailable, &[e])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumbnail_cache_keeps_the_most_recently_drawn() {
        let (mut home, _shared) = Home::for_tests(vec![]);
        let ctx = egui::Context::default();
        for i in 0..THUMB_CACHE + 10 {
            let tex = ctx.load_texture(
                format!("t{i}"),
                egui::ColorImage::new([1, 1], vec![Color32::WHITE]),
                egui::TextureOptions::LINEAR,
            );
            home.textures.insert(
                PathBuf::from(format!("{i}.png")),
                Thumbnail::Ready {
                    tex,
                    size: [1, 1],
                    used: i as u64,
                },
            );
        }
        home.evict();
        assert_eq!(home.textures.len(), THUMB_CACHE);
        for i in 0..10 {
            assert!(
                !home
                    .textures
                    .contains_key(&PathBuf::from(format!("{i}.png"))),
                "{i} is the oldest"
            );
        }
        assert!(
            home.textures
                .contains_key(&PathBuf::from(format!("{}.png", THUMB_CACHE + 9)))
        );
    }

    #[test]
    fn home_wakes_up_to_read_the_folder_again() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let ctx = egui::Context::default();
        let mut full = ctx.run_ui(egui::RawInput::default(), |ui| {
            home.begin(ui, &mut shared);
        });
        full.textures_delta.clear();
        let delay = full.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        assert!(
            delay <= std::time::Duration::from_secs_f64(RESCAN_SECONDS),
            "{delay:?}"
        );
    }

    #[test]
    fn the_preview_renders_once_per_change() {
        let (mut home, mut shared) = Home::for_tests(vec![]);
        let ctx = egui::Context::default();
        home.style_preview(&ctx, &shared);
        home.style_preview(&ctx, &shared);
        assert_eq!(home.preview_renders(), 1, "nothing changed");
        shared.settings.style.padding += 3.0;
        home.style_preview(&ctx, &shared);
        assert_eq!(home.preview_renders(), 2, "the margin changed");
        shared.settings.watermark.enabled = !shared.settings.watermark.enabled;
        home.style_preview(&ctx, &shared);
        assert_eq!(home.preview_renders(), 3, "the watermark changed");
    }
}
