//! Small notice shown after copying or saving.
//! Clicking it opens the folder of a saved file; it closes on its own.

use crate::widgets::{ACCENT, BORDER, MUTED, TEXT};
use crate::{Shared, Switch};
use egui::{Color32, RichText, Sense, Stroke, Ui, vec2};
use std::path::PathBuf;

pub const NOTICE_SIZE: egui::Vec2 = egui::vec2(380.0, 72.0);
const NOTICE_SECONDS: f64 = 3.5;

pub struct Notice {
    title: String,
    path: Option<PathBuf>,
    started: Option<f64>,
}

impl Notice {
    pub fn new(title: String, path: Option<PathBuf>) -> Self {
        Self {
            title,
            path,
            started: None,
        }
    }

    pub fn copied() -> Self {
        Self::new(
            "Sua captura foi copiada para a área de transferência".into(),
            None,
        )
    }

    pub fn saved(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::new(format!("Captura salva como {name}"), Some(path))
    }

    pub fn ui(&mut self, ui: &mut Ui, _frame: &eframe::Frame, _shared: &mut Shared) -> Switch {
        let now = ui.input(|i| i.time);
        let started = *self.started.get_or_insert(now);
        let rect = ui.max_rect();
        let resp = ui.interact(rect, ui.scope_id().with("notice"), Sense::click());
        let p = ui.painter();
        p.rect_filled(rect, 0.0, Color32::from_rgb(22, 22, 25));
        p.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, if resp.hovered() { ACCENT } else { BORDER }),
            egui::StrokeKind::Inside,
        );
        p.rect_filled(
            egui::Rect::from_min_size(rect.min, vec2(4.0, rect.height())),
            0.0,
            ACCENT,
        );
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(rect.shrink2(vec2(18.0, 14.0))),
            |ui| {
                ui.label(RichText::new(&self.title).color(TEXT).strong());
                let sub = if self.path.is_some() {
                    "Clique aqui para abrir a pasta."
                } else {
                    "Cole com Ctrl+V onde quiser."
                };
                ui.label(RichText::new(sub).color(MUTED).size(12.0));
            },
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
                crate::actions::open_folder(dir);
            }
            return Switch::Close;
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
        if now - started > NOTICE_SECONDS && !resp.hovered() {
            return Switch::Close;
        }
        Switch::None
    }
}
