//! A capture pinned on top of other windows: drag to move, wheel to zoom, double-click to close.

use crate::screens::overlay::color_image;
use crate::screens::widgets::{self, ACCENT};
use crate::{Shared, Switch};
use egui::{Color32, Rect, Sense, Stroke, Ui, pos2, vec2};
use image::RgbaImage;

pub struct Pin {
    img: RgbaImage,
    tex: Option<egui::TextureHandle>,
    zoom: f32,
    base_size: egui::Vec2,
}

impl Pin {
    pub fn new(img: RgbaImage, base_size: egui::Vec2) -> Self {
        Self {
            img,
            tex: None,
            zoom: 1.0,
            base_size,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, frame: &eframe::Frame, shared: &mut Shared) -> Switch {
        let ctx = ui.ctx().clone();
        let tex = self
            .tex
            .get_or_insert_with(|| {
                ctx.load_texture("pin", color_image(&self.img), egui::TextureOptions::LINEAR)
            })
            .id();
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.scope_id().with("pin"), Sense::click_and_drag());
        ui.painter().image(
            tex,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let border = if resp.hovered() {
            ACCENT
        } else {
            Color32::from_black_alpha(120)
        };
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        if resp.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        let mut switch = Switch::None;
        if resp.double_clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            switch = Switch::Close;
        }
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if resp.hovered() && scroll.abs() > 0.5 {
            self.zoom = (self.zoom * if scroll > 0.0 { 1.06 } else { 1.0 / 1.06 }).clamp(0.15, 4.0);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.base_size * self.zoom));
        }
        resp.context_menu(|ui| {
            if ui.button("Copiar").clicked() {
                let _ = crate::system::actions::copy_image(&self.img);
                ui.close();
            }
            if ui.button("Salvar…").clicked() {
                if let Some(p) =
                    crate::system::actions::ask_save_path(&shared.settings.save_dir, Some(frame))
                {
                    let _ = crate::system::actions::save_png(&self.img, &p);
                }
                ui.close();
            }
            if ui.button("Abrir no editor").clicked() {
                if let Some(p) = crate::system::actions::temp_png(&self.img) {
                    let _ = crate::system::actions::spawn_self(&["--edit", &p.to_string_lossy()]);
                }
                ui.close();
            }
            if ui.button("Tamanho original").clicked() {
                self.zoom = 1.0;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.base_size));
                ui.close();
            }
            ui.separator();
            if ui.button("Fechar").clicked() {
                switch = Switch::Close;
            }
        });
        let _ = (vec2(0.0, 0.0), widgets::MUTED);
        switch
    }
}
