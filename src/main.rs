#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Takeshot: a screenshot tool with an editor for backgrounds, frames and watermarks.
//!
//! Modes (each runs in its own window/process):
//!   (none)                 main window, listens for the capture keys
//!   --capture [--full] [--delay S] [--wait] [--sample]   area capture overlay
//!   --edit [file]          editor for a picture (a sample when no file)
//!   --pin file             picture pinned on top of other windows
//!   --screenshot out.png   draw ~120 frames, save the window as PNG, exit

mod engine;
mod screens;
mod system;
mod ui;

use engine::{model, render};
use screens::{editor, home, icons, notice, overlay, pin, widgets};
use system::{actions, capture, fonts, settings};

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Shared {
    pub settings: settings::Settings,
    pub settings_dirty: bool,
    pub fonts: Arc<fonts::FontLib>,
    pub logo: widgets::LogoCache,
    toast: Option<(String, std::time::Instant)>,
}

impl Shared {
    pub fn toast(&mut self, msg: String) {
        self.toast = Some((msg, std::time::Instant::now()));
    }
}

pub enum Switch {
    None,
    ToEditor(Box<editor::Editor>),
    ToNotice(Box<notice::Notice>),
    Close,
}

enum Mode {
    Home(Box<home::Home>),
    Overlay(Box<overlay::Overlay>),
    Editor(Box<editor::Editor>),
    Pin(Box<pin::Pin>),
    Notice(Box<notice::Notice>),
}

enum Launch {
    Home,
    Capture { full: bool },
    Edit(Option<PathBuf>),
    Pin(PathBuf),
}

struct Args {
    launch: Launch,
    screenshot: Option<PathBuf>,
    delay: f32,
    sample: bool,
}

fn parse_args() -> Args {
    let mut a = Args {
        launch: Launch::Home,
        screenshot: None,
        delay: 0.0,
        sample: false,
    };
    let v: Vec<String> = std::env::args().skip(1).collect();
    let mut full = false;
    let mut capture = false;
    let mut i = 0;
    while i < v.len() {
        match v[i].as_str() {
            "--screenshot" => {
                a.screenshot = v.get(i + 1).map(PathBuf::from);
                i += 1;
            }
            "--capture" => capture = true,
            "--full" => full = true,
            "--sample" => a.sample = true,
            "--wait" => a.delay = a.delay.max(0.3),
            "--delay" => {
                a.delay = v.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3.0);
                i += 1;
            }
            "--edit" => {
                let file = v
                    .get(i + 1)
                    .filter(|s| !s.starts_with("--"))
                    .map(PathBuf::from);
                if file.is_some() {
                    i += 1;
                }
                a.launch = Launch::Edit(file);
            }
            "--pin" => {
                if let Some(f) = v.get(i + 1) {
                    a.launch = Launch::Pin(PathBuf::from(f));
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if capture {
        a.launch = Launch::Capture { full };
    }
    a
}

struct App {
    mode: Mode,
    shared: Shared,
    screenshot: Option<PathBuf>,
    frames: u32,
    shot_saved: Arc<AtomicBool>,
    shot_requested: bool,
    editor_window: (egui::Pos2, egui::Vec2),
    /// Monitor of the capture, in points (for placing the notice).
    monitor: (egui::Pos2, egui::Vec2),
}

impl App {
    fn switch_to_editor(&mut self, ctx: &egui::Context, e: Box<editor::Editor>) {
        use egui::ViewportCommand as C;
        let (pos, size) = self.editor_window;
        if !cfg!(target_os = "macos") {
            ctx.send_viewport_cmd(C::Fullscreen(false));
        }
        ctx.send_viewport_cmd(C::WindowLevel(egui::WindowLevel::Normal));
        ctx.send_viewport_cmd(C::Decorations(true));
        ctx.send_viewport_cmd(C::Resizable(true));
        ctx.send_viewport_cmd(C::InnerSize(size));
        ctx.send_viewport_cmd(C::OuterPosition(pos));
        ctx.send_viewport_cmd(C::Title("Takeshot · editor".into()));
        self.mode = Mode::Editor(e);
    }

    fn switch_to_notice(&mut self, ctx: &egui::Context, n: Box<notice::Notice>) {
        use egui::ViewportCommand as C;
        let (pos, size) = self.monitor;
        if !cfg!(target_os = "macos") {
            ctx.send_viewport_cmd(C::Fullscreen(false));
        }
        ctx.send_viewport_cmd(C::Decorations(false));
        ctx.send_viewport_cmd(C::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        ctx.send_viewport_cmd(C::InnerSize(notice::NOTICE_SIZE));
        ctx.send_viewport_cmd(C::OuterPosition(notice_pos(pos, size)));
        self.mode = Mode::Notice(n);
    }
}

fn notice_pos(pos: egui::Pos2, size: egui::Vec2) -> egui::Pos2 {
    pos + size - notice::NOTICE_SIZE - egui::vec2(24.0, 64.0)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let sw = match &mut self.mode {
            Mode::Home(h) => h.ui(ui, frame, &mut self.shared),
            Mode::Overlay(o) => o.ui(ui, frame, &mut self.shared),
            Mode::Editor(e) => e.ui(ui, frame, &mut self.shared),
            Mode::Pin(p) => p.ui(ui, frame, &mut self.shared),
            Mode::Notice(n) => n.ui(ui, frame, &mut self.shared),
        };
        match sw {
            Switch::ToEditor(e) => self.switch_to_editor(&ctx, e),
            Switch::ToNotice(n) => {
                self.shared.settings.save();
                self.switch_to_notice(&ctx, n);
            }
            Switch::Close => {
                self.shared.settings.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Switch::None => {}
        }

        if let Some((msg, at)) = &self.shared.toast {
            if at.elapsed().as_secs_f32() < 2.6 {
                widgets::toast(&ctx, msg);
                ctx.request_repaint();
            } else {
                self.shared.toast = None;
            }
        }
        if self.shared.settings_dirty && !ctx.input(|i| i.pointer.any_down()) {
            self.shared.settings_dirty = false;
            if self.screenshot.is_none() {
                self.shared.settings.save();
            }
        }

        // self-check: draw ~120 frames, then save the window picture
        if let Some(path) = &self.screenshot {
            self.frames += 1;
            ctx.request_repaint();
            if self.frames >= 120 && !self.shot_requested {
                self.shot_requested = true;
                let path = path.clone();
                let done = self.shot_saved.clone();
                ctx.request_screenshot(move |img| {
                    let [w, h] = img.size;
                    let mut bytes = Vec::with_capacity(w * h * 4);
                    for p in &img.pixels {
                        bytes.extend_from_slice(&p.to_srgba_unmultiplied());
                    }
                    if let Some(rgba) = image::RgbaImage::from_raw(w as u32, h as u32, bytes)
                        && let Err(e) = actions::save_png(&rgba, &path)
                    {
                        eprintln!("screenshot failed: {e}");
                    }
                    done.store(true, Ordering::SeqCst);
                });
            }
            if self.shot_saved.load(Ordering::SeqCst) || self.frames > 600 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

fn main() -> eframe::Result {
    let args = parse_args();
    let settings = settings::Settings::load();

    // Grab the screen before any window of ours exists.
    let mut shot = None;
    let mut capture_error = None;
    if let Launch::Capture { .. } = args.launch {
        if args.delay > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f32(args.delay));
        }
        if args.sample || args.screenshot.is_some() {
            shot = Some(capture::sample_shot());
        } else {
            match capture::grab() {
                Ok(s) => shot = Some(s),
                Err(e) => capture_error = Some(e),
            }
        }
    }

    let mut vp = egui::ViewportBuilder::default()
        .with_icon(Arc::new(icons::app_icon()))
        .with_app_id("takeshot");
    let mut editor_window = (egui::pos2(80.0, 60.0), egui::vec2(1280.0, 820.0));
    let mut monitor = (egui::pos2(0.0, 0.0), egui::vec2(1600.0, 900.0));
    let mut instant_notice = None;
    let mut pin_img = None;
    match (&args.launch, &shot) {
        (Launch::Capture { full }, Some(s)) => {
            let k = s.scale;
            let pos = egui::pos2(s.x as f32 / k, s.y as f32 / k);
            let size = egui::vec2(s.image.width() as f32 / k, s.image.height() as f32 / k);
            let ew = egui::vec2(1280.0f32.min(size.x - 40.0), 820.0f32.min(size.y - 60.0));
            editor_window = (pos + (size - ew) / 2.0, ew);
            monitor = (pos, size);
            if *full && !settings.open_editor_after_full && args.screenshot.is_none() {
                // Shift+PrtSc saves the full screen at once.
                let fonts = fonts::FontLib::load();
                let logo = settings
                    .watermark
                    .logo_path
                    .as_deref()
                    .and_then(render::load_logo);
                let img = render::finish(
                    &s.image,
                    &settings,
                    logo.as_ref(),
                    &fonts,
                    settings.quick_style,
                );
                let path = actions::auto_path(&settings.save_dir);
                if let Err(e) = actions::save_png(&img, &path) {
                    eprintln!("save failed: {e}");
                    return Ok(());
                }
                if !settings.show_bubbles {
                    return Ok(());
                }
                vp = vp
                    .with_title("Takeshot")
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_resizable(false)
                    .with_inner_size(notice::NOTICE_SIZE)
                    .with_position(notice_pos(pos, size));
                instant_notice = Some(notice::Notice::saved(path));
            } else if *full && settings.open_editor_after_full {
                vp = vp
                    .with_title("Takeshot · editor")
                    .with_inner_size(ew)
                    .with_position(editor_window.0);
            } else if args.screenshot.is_some() {
                vp = vp
                    .with_title("Takeshot · captura")
                    .with_inner_size([1280.0, 720.0])
                    .with_decorations(false);
            } else {
                vp = vp
                    .with_title("Takeshot · captura")
                    .with_decorations(false)
                    .with_position(pos)
                    .with_inner_size(size)
                    .with_always_on_top()
                    .with_resizable(false);
                if !cfg!(target_os = "macos") {
                    vp = vp.with_fullscreen(true);
                }
            }
        }
        (Launch::Edit(_), _) => {
            vp = vp
                .with_title("Takeshot · editor")
                .with_inner_size([1280.0, 820.0])
                .with_min_inner_size([900.0, 560.0]);
        }
        (Launch::Pin(p), _) => {
            if let Ok(img) = image::open(p) {
                let img = img.to_rgba8();
                let sz = egui::vec2(img.width() as f32, img.height() as f32);
                let k = (900.0 / sz.x).min(700.0 / sz.y).min(1.0);
                vp = vp
                    .with_title("Takeshot · fixado")
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_inner_size(sz * k)
                    .with_resizable(false);
                pin_img = Some((img, sz * k));
                let _ = std::fs::remove_file(p);
            }
        }
        _ => {
            vp = vp
                .with_title("Takeshot")
                .with_inner_size([1160.0, 740.0])
                .with_min_inner_size([900.0, 600.0])
                .with_drag_and_drop(true);
        }
    }

    let options = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };
    let screenshot = args.screenshot.clone();
    let demo = args.sample;
    let res = eframe::run_native(
        "Takeshot",
        options,
        Box::new(move |cc| {
            let ctx = &cc.egui_ctx;
            let fonts = Arc::new(fonts::FontLib::load());
            fonts.install(ctx);
            widgets::apply_theme(ctx);
            let mut shared = Shared {
                settings,
                settings_dirty: false,
                fonts,
                logo: widgets::LogoCache::new(),
                toast: None,
            };
            shared.logo.sync(ctx, &shared.settings.watermark);
            let mode = match args.launch {
                Launch::Capture { .. } if instant_notice.is_some() => {
                    Mode::Notice(Box::new(instant_notice.take().unwrap()))
                }
                Launch::Capture { full } => match shot {
                    Some(s) if full && shared.settings.open_editor_after_full => Mode::Editor(
                        Box::new(editor::Editor::new(ctx, s.image, Vec::new(), &shared)),
                    ),
                    Some(s) => {
                        let o = overlay::Overlay::new(ctx, s, &shared);
                        Mode::Overlay(Box::new(if demo { o.demo() } else { o }))
                    }
                    None => {
                        shared.toast(format!(
                            "Não consegui capturar a tela: {}",
                            capture_error.unwrap_or_default()
                        ));
                        Mode::Home(Box::new(home::Home::new(ctx, &shared, false)))
                    }
                },
                Launch::Edit(file) => {
                    let img = file
                        .as_ref()
                        .and_then(|p| image::open(p).ok())
                        .map(|i| i.to_rgba8());
                    let (img, demo) = match img {
                        Some(i) => (i, false),
                        None => (capture::sample_window(1200, 740), true),
                    };
                    if let Some(p) = &file
                        && p.starts_with(std::env::temp_dir())
                    {
                        let _ = std::fs::remove_file(p);
                    }
                    let anns = if demo { demo_annotations() } else { Vec::new() };
                    Mode::Editor(Box::new(editor::Editor::new(ctx, img, anns, &shared)))
                }
                Launch::Pin(_) => match pin_img {
                    Some((img, size)) => Mode::Pin(Box::new(pin::Pin::new(img, size))),
                    None => Mode::Home(Box::new(home::Home::new(ctx, &shared, false))),
                },
                Launch::Home => Mode::Home(Box::new(home::Home::new(
                    ctx,
                    &shared,
                    screenshot.is_none(),
                ))),
            };
            Ok(Box::new(App {
                mode,
                shared,
                screenshot,
                frames: 0,
                shot_saved: Arc::new(AtomicBool::new(false)),
                shot_requested: false,
                editor_window,
                monitor,
            }))
        }),
    );
    actions::wait_clipboard_keepers();
    res
}

/// A few marks on the sample picture so the demo editor shows the tools.
fn demo_annotations() -> Vec<model::Ann> {
    use egui::pos2;
    use model::{Ann, Shape};
    vec![
        Ann {
            shape: Shape::Ellipse(pos2(560.0, 150.0), pos2(830.0, 300.0)),
            color: [255, 0, 0, 255],
            width: 4.0,
            filled: false,
        },
        Ann {
            shape: Shape::Arrow(pos2(980.0, 470.0), pos2(820.0, 360.0)),
            color: [255, 0, 0, 255],
            width: 4.0,
            filled: false,
        },
        Ann {
            shape: Shape::Counter {
                center: pos2(1000.0, 490.0),
                n: 1,
            },
            color: [255, 0, 0, 255],
            width: 4.0,
            filled: true,
        },
    ]
}
