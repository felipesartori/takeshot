//! Bits the screens share: tool shortcuts, hex colours and the watermark logo cache.

use crate::engine::annotator::Annotator;
use crate::engine::model::*;
use crate::system::settings::{DrawOpts, Watermark};
use egui::Ui;

/// Pick a tool by its key (P, L, A, R, O, M, T, B, N, V).
pub fn tool_shortcuts(ui: &mut Ui, an: &mut Annotator, opts: &mut DrawOpts, allow_select: bool) {
    let mods = ui.input(|i| i.modifiers);
    if mods.ctrl || mods.command || mods.alt {
        return;
    }
    for t in [Tool::Select].into_iter().chain(Tool::DRAWING) {
        if t == Tool::Select && !allow_select {
            continue;
        }
        if ui.input(|i| i.key_pressed(t.key())) {
            an.set_tool(t, opts);
        }
    }
}

pub fn hex(c: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub struct LogoCache {
    pub path: Option<String>,
    pub image: Option<image::RgbaImage>,
    pub thumb: Option<egui::TextureHandle>,
}

impl LogoCache {
    pub fn new() -> Self {
        Self {
            path: None,
            image: None,
            thumb: None,
        }
    }
    pub fn sync(&mut self, ctx: &egui::Context, wm: &Watermark) {
        if self.path != wm.logo_path {
            self.path = wm.logo_path.clone();
            self.image = wm
                .logo_path
                .as_deref()
                .and_then(crate::engine::render::load_logo);
            self.thumb = self.image.as_ref().map(|img| {
                let t = image::imageops::thumbnail(
                    img,
                    96,
                    (96.0 * img.height() as f32 / img.width().max(1) as f32).max(1.0) as u32,
                );
                ctx.load_texture(
                    "logo-thumb",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [t.width() as usize, t.height() as usize],
                        t.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        }
    }
}
