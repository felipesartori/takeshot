//! The resident's windows: the main window shown or hidden, the app-drawn tray menu, and the close, PrtSc and About dialogs.
//!
//! On Windows and macOS the root viewport is an invisible host, so egui keeps running passes and
//! can open the tray menu while the main window (a child viewport) is hidden. On Linux the root is
//! the main window itself: the desktop draws the tray menu, so nothing needs a pass while hidden.

use crate::screens::about::{self, AboutAnswer, AboutInfo};
use crate::screens::home::Home;
use crate::screens::tray_menu::{MenuOutcome, TrayMenu};
use crate::screens::widgets;
use crate::system::actions;
use crate::system::hotkeys::{Shot, Status, effective_status, label, should_offer_priority};
use crate::system::instance::Page;
use crate::system::resident::{
    Resident, ResidentEvent, XAction, hide_command, left_click, on_wayland, on_x, remembered,
};
use crate::system::tray::{TrayAction, TrayEntry, TrayEvent, tray_entries};
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, BtnKind};
use crate::ui::tokens::{Tokens, effective_dark, tokens};
use crate::{Shared, Switch};
use egui::{ViewportCommand as Cmd, ViewportId};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Whether the root is an invisible host (Windows, macOS) or the main window itself (Linux).
pub const HOSTED: bool = !cfg!(target_os = "linux");

/// The main window's viewport when the root is the invisible host.
fn main_id() -> ViewportId {
    ViewportId::from_hash_of("takeshot-main")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OfferChoice {
    Later,
    Settings,
    Priority,
}

pub struct Shell {
    /// The main window is on screen (or about to be).
    pub main_open: bool,
    /// The root is an invisible host and the main window a child viewport (Windows, macOS).
    hosted: bool,
    menu: TrayMenu,
    recents: Arc<Mutex<Vec<PathBuf>>>,
    /// The first-X question, with its "Remember my choice" box.
    close_prompt: Option<bool>,
    about: bool,
    offer: bool,
    /// The About mark, for the theme and mode it was drawn in.
    mark: Option<((crate::ui::tokens::ThemeId, bool), egui::TextureHandle)>,
    /// The main window's icon, decoded once per theme.
    icon: Option<(crate::ui::tokens::ThemeId, Arc<egui::IconData>)>,
    quitting: bool,
    /// The first frame ran (the start-up hide happens once).
    started: bool,
    /// Bumped by the recents thread, so the Linux native menu is rebuilt when the list changes.
    recents_version: Arc<std::sync::atomic::AtomicU64>,
    menu_version: u64,
    next_recents: f64,
}

impl Shell {
    pub fn new(hosted: bool, main_open: bool) -> Self {
        Self {
            main_open,
            hosted,
            menu: TrayMenu::default(),
            recents: Arc::new(Mutex::new(Vec::new())),
            close_prompt: None,
            about: false,
            offer: false,
            mark: None,
            icon: None,
            quitting: false,
            started: false,
            recents_version: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            menu_version: u64::MAX,
            next_recents: 0.0,
        }
    }

    /// Keeps the recent captures fresh (every 30 s) and the Linux native menu in step with them
    /// and the mode. Called on every wake-up, window shown or not.
    pub fn tick(&mut self, ctx: &egui::Context, resident: &Resident, shared: &Shared) {
        let now = ctx.input(|i| i.time);
        if now >= self.next_recents {
            self.next_recents = now + 30.0;
            self.refresh_recents(ctx, shared);
            ctx.request_repaint_after(std::time::Duration::from_secs(30));
        }
        let v = self
            .recents_version
            .load(std::sync::atomic::Ordering::Relaxed);
        if v != self.menu_version
            && let Some(tray) = &resident.tray
        {
            self.menu_version = v;
            tray.refresh_menu(&self.entries(shared));
        }
    }

    fn theme(ctx: &egui::Context, shared: &Shared) -> Tokens {
        tokens(
            shared.settings.ui_theme,
            effective_dark(shared.settings.ui_mode, ctx.system_theme()),
        )
    }

    /// The tray menu entries for now; capture items lose their shortcut text when shortcuts are off.
    fn entries(&self, shared: &Shared) -> Vec<TrayEntry> {
        let recents = self.recents.lock().map(|r| r.clone()).unwrap_or_default();
        let s = &shared.settings;
        let mut e = tray_entries(&recents, s.ui_mode, &s.hotkey_area, &s.hotkey_full);
        if !s.hotkeys {
            for x in &mut e {
                if let TrayEntry::Item { keys, .. } = x {
                    *keys = None;
                }
            }
        }
        e
    }

    /// Lists the recent captures off the UI thread (the folder may be large or on a network share).
    fn refresh_recents(&self, ctx: &egui::Context, shared: &Shared) {
        let dir = PathBuf::from(&shared.settings.save_dir);
        let (out, ctx) = (self.recents.clone(), ctx.clone());
        let version = self.recents_version.clone();
        std::thread::spawn(move || {
            let list = crate::system::recents::recent_captures(&dir, 5);
            if store_recents(&out, &version, list) {
                ctx.request_repaint();
            }
        });
    }

    /// Reacts to an event from the hotkeys, the hook, the tray or a second launch.
    pub fn handle(
        &mut self,
        ctx: &egui::Context,
        ev: ResidentEvent,
        home: &mut Home,
        shared: &mut Shared,
    ) {
        match ev {
            ResidentEvent::Show(p) => self.show(ctx, home, p),
            ResidentEvent::Shot(s) => self.capture(ctx, s),
            ResidentEvent::Tray(TrayEvent::LeftClick) => {
                let e = left_click(shared.settings.tray_click);
                self.handle(ctx, e, home, shared);
            }
            ResidentEvent::Tray(TrayEvent::MenuAt { icon }) => {
                if self.hosted {
                    self.refresh_recents(ctx, shared);
                    let ppp = ctx.pixels_per_point();
                    let points = |r: egui::Rect| {
                        egui::Rect::from_min_max(
                            (r.min.to_vec2() / ppp).to_pos2(),
                            (r.max.to_vec2() / ppp).to_pos2(),
                        )
                    };
                    // The menus stay off the taskbar: the work area of the icon's monitor.
                    let bounds = crate::system::capture::work_area(icon.center())
                        .map(points)
                        .unwrap_or_else(|| {
                            let size = ctx
                                .input(|i| i.viewport().monitor_size)
                                .unwrap_or(egui::vec2(1920.0, 1080.0));
                            egui::Rect::from_min_size(egui::Pos2::ZERO, size)
                        });
                    let entries = self.entries(shared);
                    self.menu.open(points(icon), bounds, &entries);
                }
            }
            ResidentEvent::Tray(TrayEvent::Action(a)) => self.run(ctx, a, home, shared),
        }
    }

    fn run(&mut self, ctx: &egui::Context, a: TrayAction, home: &mut Home, shared: &mut Shared) {
        match a {
            TrayAction::Capture(s) => self.capture(ctx, s),
            TrayAction::CaptureIn3s => {
                self.minimize_main(ctx);
                let _ = actions::spawn_self(&["--capture", "--delay", "3"]);
            }
            TrayAction::Show(p) => self.show(ctx, home, p),
            TrayAction::CopyRecent(p) => match image::open(&p) {
                Ok(img) => match actions::copy_image(&img.to_rgba8()) {
                    Ok(()) => shared.toast(tr(Msg::Copy).to_string()),
                    Err(e) => shared.toast(e),
                },
                Err(e) => shared.toast(e.to_string()),
            },
            TrayAction::EditRecent(p) => {
                let _ = actions::spawn_self(&["--edit", &p.to_string_lossy()]);
            }
            TrayAction::OpenCapturesFolder => {
                actions::open_folder(std::path::Path::new(&shared.settings.save_dir));
            }
            TrayAction::SetMode(m) => {
                shared.settings.ui_mode = m;
                shared.settings_dirty = true;
                // The native menu shows the new check on its next rebuild.
                self.menu_version = u64::MAX;
            }
            TrayAction::Quit => self.quit(ctx),
        }
    }

    /// The main window's viewport: a child of the host, or the root itself on Linux.
    fn main_viewport(&self) -> ViewportId {
        if self.hosted {
            main_id()
        } else {
            ViewportId::ROOT
        }
    }

    fn show(&mut self, ctx: &egui::Context, home: &mut Home, page: Page) {
        self.main_open = true;
        let id = self.main_viewport();
        if !self.hosted {
            ctx.send_viewport_cmd_to(id, Cmd::Visible(true));
        }
        ctx.send_viewport_cmd_to(id, Cmd::Minimized(false));
        ctx.send_viewport_cmd_to(id, Cmd::Focus);
        if page == Page::About {
            self.about = true;
        } else {
            home.open(page);
        }
    }

    fn hide(&mut self, ctx: &egui::Context) {
        self.main_open = false;
        if !self.hosted {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, hide_command(on_wayland()));
        }
    }

    fn minimize_main(&self, ctx: &egui::Context) {
        if self.main_open {
            ctx.send_viewport_cmd_to(self.main_viewport(), Cmd::Minimized(true));
        }
    }

    fn capture(&self, ctx: &egui::Context, s: Shot) {
        self.minimize_main(ctx);
        let args: &[&str] = match s {
            Shot::Area => &["--capture", "--wait"],
            Shot::Full => &["--capture", "--full", "--wait"],
        };
        let _ = actions::spawn_self(args);
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        ctx.send_viewport_cmd_to(ViewportId::ROOT, Cmd::Close);
    }

    /// Draws the resident's windows for this frame. `ui` is the root's: the host on Windows and
    /// macOS, the main window on Linux.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        frame: &eframe::Frame,
        resident: &mut Resident,
        home: &mut Home,
        shared: &mut Shared,
    ) {
        let ctx = ui.ctx().clone();
        // A window the root could not start hidden (--minimized on Wayland) leaves on the first
        // frame, once: after that the user may bring it back from the taskbar. On X11 the hidden
        // root runs no frame until it is shown.
        if !std::mem::replace(&mut self.started, true) && !self.hosted && !self.main_open {
            ctx.send_viewport_cmd_to(ViewportId::ROOT, hide_command(on_wayland()));
        }
        if cfg!(windows) && resident.priority_on() != shared.settings.prtsc_priority {
            resident.set_priority(shared.settings.prtsc_priority);
        }
        let raw = resident.hotkeys.status().clone();
        home.hotkey_status = Some(effective_status(&raw, resident.priority_on()));
        if cfg!(windows)
            && self.main_open
            && should_offer_priority(
                &raw,
                shared.settings.prtsc_offer_shown,
                resident.priority_on(),
            )
        {
            self.offer = true;
        }
        if self.hosted {
            if self.main_open {
                let theme = shared.settings.ui_theme;
                if self.icon.as_ref().is_none_or(|(t, _)| *t != theme) {
                    self.icon = Some((theme, Arc::new(crate::ui::brand::window_icon(theme))));
                }
                let icon = self
                    .icon
                    .as_ref()
                    .map(|(_, i)| i.clone())
                    .unwrap_or_default();
                let vp = egui::ViewportBuilder::default()
                    .with_title("Takeshot")
                    .with_app_id("takeshot")
                    .with_inner_size([1160.0, 740.0])
                    .with_min_inner_size([900.0, 600.0])
                    .with_drag_and_drop(true)
                    .with_icon(icon);
                ctx.show_viewport_immediate(main_id(), vp, |ui, _| {
                    self.main_contents(ui, frame, resident, home, shared);
                });
            }
            if self.menu.is_open() {
                let t = Self::theme(&ctx, shared);
                let entries = self.entries(shared);
                let ready = matches!(home.hotkey_status, Some(Status::Ready));
                let thumb = |p: &std::path::Path| home.thumbnail(p);
                if let MenuOutcome::Chosen(a) = self.menu.show(&ctx, &t, &entries, ready, &thumb) {
                    self.run(&ctx, a, home, shared);
                }
            }
        } else {
            self.main_contents(ui, frame, resident, home, shared);
        }
    }

    /// The main window: Home, its requests, the X behaviour and the dialogs over it.
    fn main_contents(
        &mut self,
        ui: &mut egui::Ui,
        frame: &eframe::Frame,
        resident: &mut Resident,
        home: &mut Home,
        shared: &mut Shared,
    ) {
        let ctx = ui.ctx().clone();
        if ui.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(Cmd::CancelClose);
            match on_x(shared.settings.on_close) {
                XAction::Ask => self.close_prompt = Some(true),
                XAction::Hide => self.hide(&ctx),
                XAction::Quit => self.quit(&ctx),
            }
        }
        let _: Switch = home.ui(ui, frame, shared);
        let req = std::mem::take(&mut home.requests);
        if req.about {
            self.about = true;
        }
        if req.quit {
            self.quit(&ctx);
        }
        if let Some((shot, keys)) = req.rebind {
            match resident.hotkeys.rebind(shot, &keys) {
                Ok(()) => {
                    match shot {
                        Shot::Area => shared.settings.hotkey_area = keys,
                        Shot::Full => shared.settings.hotkey_full = keys,
                    }
                    shared.settings_dirty = true;
                }
                Err(e) => shared.toast(format!("{}: {e}", label(&keys))),
            }
        }
        if req.open_windows_settings {
            open_windows_keyboard_settings();
        }
        let t = Self::theme(&ctx, shared);
        self.dialogs(&ctx, &t, resident, shared);
        draw_toast(&ctx, shared);
    }

    fn dialogs(
        &mut self,
        ctx: &egui::Context,
        t: &Tokens,
        resident: &mut Resident,
        shared: &mut Shared,
    ) {
        if let Some(mut remember) = self.close_prompt {
            let answer = kit::confirm_dialog(
                ctx,
                t,
                "takeshot-close-app",
                tr(Msg::CloseAppTitle),
                tr(Msg::CloseAppBody),
                &mut |ui: &mut egui::Ui| {
                    kit::checkbox(ui, t, &mut remember, tr(Msg::RememberChoice));
                },
                &[
                    (tr(Msg::Quit), BtnKind::Secondary, XAction::Quit),
                    (tr(Msg::MinimizeToTray), BtnKind::Primary, XAction::Hide),
                ],
                XAction::Ask,
            );
            self.close_prompt = Some(remember);
            if let Some(choice) = answer {
                self.close_prompt = None;
                if let Some(on_close) = remembered(choice, remember) {
                    shared.settings.on_close = on_close;
                    shared.settings_dirty = true;
                }
                match choice {
                    XAction::Quit => self.quit(ctx),
                    XAction::Hide => self.hide(ctx),
                    XAction::Ask => {}
                }
            }
        }
        if self.about {
            let look = (
                shared.settings.ui_theme,
                effective_dark(shared.settings.ui_mode, ctx.system_theme()),
            );
            if self.mark.as_ref().is_none_or(|(l, _)| *l != look) {
                self.mark = Some((look, crate::ui::brand::mark_texture(ctx, look.0, look.1)));
            }
            let info = AboutInfo::current();
            match about::dialog(ctx, t, &info, self.mark.as_ref().map(|(_, m)| m.id())) {
                Some(AboutAnswer::GitHub) => ctx.open_url(egui::OpenUrl::new_tab(info.github)),
                Some(AboutAnswer::Close) => self.about = false,
                None => {}
            }
        }
        if self.offer && self.close_prompt.is_none() && !self.about {
            let answer = kit::confirm_dialog(
                ctx,
                t,
                "takeshot-prtsc-offer",
                tr(Msg::PrtScBusy),
                tr(Msg::PrtScConflictHelp),
                &mut |_| {},
                &[
                    (
                        tr(Msg::OpenWindowsSettings),
                        BtnKind::Secondary,
                        OfferChoice::Settings,
                    ),
                    (
                        tr(Msg::PrtScPriority),
                        BtnKind::Primary,
                        OfferChoice::Priority,
                    ),
                ],
                OfferChoice::Later,
            );
            if let Some(choice) = answer {
                self.offer = false;
                shared.settings.prtsc_offer_shown = true;
                shared.settings_dirty = true;
                match choice {
                    OfferChoice::Priority => {
                        shared.settings.prtsc_priority = true;
                        resident.set_priority(true);
                    }
                    OfferChoice::Settings => open_windows_keyboard_settings(),
                    OfferChoice::Later => {}
                }
            }
        }
    }
}

/// Stores a fresh list of recent captures; the version (which rebuilds the Linux native menu, closing
/// it if open) moves only when the list changed. True when it changed.
fn store_recents(
    out: &Mutex<Vec<PathBuf>>,
    version: &std::sync::atomic::AtomicU64,
    list: Vec<PathBuf>,
) -> bool {
    let Ok(mut r) = out.lock() else {
        return false;
    };
    if *r == list {
        return false;
    }
    *r = list;
    version.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    true
}

/// Windows' keyboard settings, where the Snipping Tool's PrtSc option lives.
fn open_windows_keyboard_settings() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg("ms-settings:easeofaccess-keyboard")
            .spawn();
    }
}

/// The short notice, drawn in whichever window is the main one.
fn draw_toast(ctx: &egui::Context, shared: &mut Shared) {
    if let Some((msg, at)) = &shared.toast {
        if at.elapsed().as_secs_f32() < 2.6 {
            widgets::toast(ctx, msg);
            ctx.request_repaint();
        } else {
            shared.toast = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_native_menu_is_rebuilt_only_when_the_recents_change() {
        // Rebuilding the Linux menu closes it under the pointer: same list, same version.
        let (out, version) = (Mutex::new(Vec::new()), std::sync::atomic::AtomicU64::new(0));
        let v = || version.load(std::sync::atomic::Ordering::Relaxed);
        store_recents(&out, &version, vec![PathBuf::from("a.png")]);
        assert_eq!(v(), 1);
        store_recents(&out, &version, vec![PathBuf::from("a.png")]);
        assert_eq!(v(), 1, "same list");
        store_recents(
            &out,
            &version,
            vec![PathBuf::from("b.png"), PathBuf::from("a.png")],
        );
        assert_eq!(v(), 2);
        assert_eq!(out.lock().unwrap().len(), 2);
    }
}
