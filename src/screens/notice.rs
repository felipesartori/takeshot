//! Small notice shown after copying or saving.
//! Clicking it opens the folder of a saved file; it closes on its own.

use crate::screens::widgets::{ACCENT, BORDER, MUTED, TEXT};
use crate::{Shared, Switch};
use egui::{Color32, RichText, Sense, Stroke, Ui, vec2};
use std::path::PathBuf;

pub const NOTICE_SIZE: egui::Vec2 = egui::vec2(380.0, 72.0);
/// The gap to the right edge and to the taskbar (the edge of the work area).
pub const NOTICE_MARGIN: f32 = 16.0;
const NOTICE_SECONDS: f64 = 3.5;

pub struct Notice {
    title: String,
    path: Option<PathBuf>,
    started: Option<f64>,
}

/// Where the notice opens: the bottom-right corner of the work area (`area`, in points).
pub fn notice_pos(area: egui::Rect) -> egui::Pos2 {
    area.max - NOTICE_SIZE - vec2(NOTICE_MARGIN, NOTICE_MARGIN)
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
        // The accent bar on the left is the only highlight; hovering shows the hand cursor.
        p.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, BORDER),
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
                crate::system::actions::open_folder(dir);
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Rect, pos2};

    #[test]
    fn the_notice_sits_in_the_corner_of_the_work_area() {
        // A 40 px taskbar at the bottom: the notice keeps off it by the same gap as off the right edge.
        let area = Rect::from_min_max(pos2(0.0, 0.0), pos2(1920.0, 1040.0));
        let r = Rect::from_min_size(notice_pos(area), NOTICE_SIZE);
        assert!(area.contains_rect(r), "{r:?}");
        assert_eq!(area.max.x - r.max.x, NOTICE_MARGIN);
        assert_eq!(area.max.y - r.max.y, NOTICE_MARGIN);
        // A second monitor to the left, taskbar on top: still its own corner.
        let area = Rect::from_min_max(pos2(-1280.0, 48.0), pos2(0.0, 1024.0));
        let r = Rect::from_min_size(notice_pos(area), NOTICE_SIZE);
        assert!(area.contains_rect(r), "{r:?}");
    }
}
