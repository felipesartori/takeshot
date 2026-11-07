//! Capture overlay: the frozen screen, area selection, drawing tools and quick actions.

use crate::engine::annotator::{Annotator, View};
use crate::engine::doc::Doc;
use crate::engine::model::*;
use crate::screens::editor::Editor;
use crate::screens::icons::Icon;
use crate::screens::widgets::{self, ACCENT, MUTED, TEXT};
use crate::system::capture::Shot;
use crate::{Shared, Switch};
use egui::{
    Align2, Color32, CursorIcon, FontId, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke, Ui,
    pos2, vec2,
};
use image::RgbaImage;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Handle {
    Move,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

enum Drag {
    None,
    NewSel(Pos2),
    Adjust {
        handle: Handle,
        start_ptr: Pos2,
        orig: Rect,
    },
}

pub struct Overlay {
    shot: Shot,
    baked: RgbaImage,
    tex: egui::TextureHandle,
    /// Selection in picture pixels.
    sel: Option<Rect>,
    drag: Drag,
    an: Annotator,
    toolbar_size: egui::Vec2,
    options_size: egui::Vec2,
    window_ready: u32,
}

impl Overlay {
    pub fn new(ctx: &egui::Context, shot: Shot, shared: &Shared) -> Self {
        let tex = load_tex(ctx, "shot", &shot.image, egui::TextureOptions::NEAREST);
        let mut sel = None;
        if shared.settings.keep_selection
            && let Some([x, y, w, h]) = shared.settings.last_selection
        {
            let r = Rect::from_min_size(pos2(x, y), vec2(w, h));
            let full = Rect::from_min_size(
                Pos2::ZERO,
                vec2(shot.image.width() as f32, shot.image.height() as f32),
            );
            if full.contains_rect(r) && w > 2.0 && h > 2.0 {
                sel = Some(r);
            }
        }
        Self {
            baked: shot.image.clone(),
            shot,
            tex,
            sel,
            drag: Drag::None,
            an: Annotator::new(Tool::Select),
            toolbar_size: vec2(560.0, 44.0),
            options_size: vec2(420.0, 44.0),
            window_ready: 0,
        }
    }

    /// Sample state for screenshots: an area already chosen and a mark on it.
    pub fn demo(mut self) -> Self {
        self.sel = Some(Rect::from_min_size(pos2(304.0, 153.0), vec2(992.0, 540.0)));
        self.an.doc = Doc::from_anns(vec![Ann {
            shape: Shape::Ellipse(pos2(700.0, 230.0), pos2(1000.0, 330.0)),
            color: DEFAULT_COLOR,
            width: DEFAULT_WIDTH,
            filled: false,
        }]);
        self.an.tool = Tool::Arrow;
        self
    }

    fn img_size(&self) -> egui::Vec2 {
        vec2(
            self.shot.image.width() as f32,
            self.shot.image.height() as f32,
        )
    }

    fn crop(&self) -> Option<RgbaImage> {
        let s = self.sel?;
        let x = s.min.x.round().max(0.0) as u32;
        let y = s.min.y.round().max(0.0) as u32;
        let w = (s.width().round() as u32).min(self.baked.width().saturating_sub(x));
        let h = (s.height().round() as u32).min(self.baked.height().saturating_sub(y));
        (w > 0 && h > 0).then(|| image::imageops::crop_imm(&self.baked, x, y, w, h).to_image())
    }

    fn remember_selection(&self, shared: &mut Shared) {
        if let Some(s) = self.sel {
            shared.settings.last_selection = Some([s.min.x, s.min.y, s.width(), s.height()]);
            shared.settings.save();
        }
    }

    fn output(&self, shared: &mut Shared) -> Option<RgbaImage> {
        let crop = self.crop()?;
        let logo = shared.logo.image.clone();
        Some(crate::engine::render::finish(
            &crop,
            &shared.settings,
            logo.as_ref(),
            &shared.fonts,
            shared.settings.quick_style,
        ))
    }

    fn open_editor(&mut self, ctx: &egui::Context, shared: &mut Shared) -> Switch {
        self.an.commit_draft(&shared.settings.draw);
        let Some(sel) = self.sel else {
            return Switch::None;
        };
        let x = sel.min.x.round().max(0.0) as u32;
        let y = sel.min.y.round().max(0.0) as u32;
        let w = (sel.width().round() as u32)
            .min(self.shot.image.width().saturating_sub(x))
            .max(1);
        let h = (sel.height().round() as u32)
            .min(self.shot.image.height().saturating_sub(y))
            .max(1);
        let base = image::imageops::crop_imm(&self.shot.image, x, y, w, h).to_image();
        let mut anns = self.an.doc.visible_anns(&[]);
        for a in &mut anns {
            a.translate(-vec2(x as f32, y as f32));
        }
        self.remember_selection(shared);
        Switch::ToEditor(Box::new(Editor::new(ctx, base, anns, shared)))
    }

    pub fn ui(&mut self, ui: &mut Ui, frame: &eframe::Frame, shared: &mut Shared) -> Switch {
        let ctx = ui.ctx().clone();
        if self.window_ready < 3 {
            self.window_ready += 1;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        let screen = ui.max_rect();
        let k = screen.width() / self.img_size().x;
        let view = View {
            origin: screen.min,
            k,
        };
        let opts_snapshot = shared.settings.draw.clone();

        // ---------------- baked picture
        if let Some(anns) = self.an.bake() {
            self.baked =
                crate::engine::render::render_annotations(&self.shot.image, &anns, &shared.fonts);
            self.tex
                .set(color_image(&self.baked), egui::TextureOptions::NEAREST);
        }

        // ---------------- canvas interaction (first, so the floating bars sit on top)
        let canvas = ui.interact(
            screen,
            ui.scope_id().with("canvas"),
            Sense::click_and_drag(),
        );
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let typing = self.an.handle_typing(ui, &shared.settings.draw);

        let drawing_tool = self.sel.is_some() && self.an.tool != Tool::Select;
        if drawing_tool {
            self.an
                .handle_pointer(ui, &canvas, &view, &shared.settings.draw);
            ctx.set_cursor_icon(if self.an.tool == Tool::Text {
                CursorIcon::Text
            } else {
                CursorIcon::Crosshair
            });
            // mouse wheel changes the stroke width (or text size)
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.5 {
                let d = &mut shared.settings.draw;
                if self.an.tool == Tool::Text {
                    d.font_size = (d.font_size + scroll.signum() * 2.0).clamp(8.0, 240.0);
                } else {
                    d.width = (d.width + scroll.signum()).clamp(1.0, 40.0);
                }
            }
        } else {
            self.selection_input(ui, &canvas, &view, pointer);
        }

        // ---------------- painting
        let painter = ui.painter_at(screen);
        painter.image(
            self.tex.id(),
            screen,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let dim = Color32::from_black_alpha((DIM_ALPHA * 255.0) as u8);
        match self.sel {
            None => {
                painter.rect_filled(screen, 0.0, dim);
            }
            Some(s) => {
                let r = Rect::from_min_max(view.to_screen(s.min), view.to_screen(s.max));
                for part in [
                    Rect::from_min_max(screen.min, pos2(screen.max.x, r.min.y)),
                    Rect::from_min_max(pos2(screen.min.x, r.max.y), screen.max),
                    Rect::from_min_max(pos2(screen.min.x, r.min.y), pos2(r.min.x, r.max.y)),
                    Rect::from_min_max(pos2(r.max.x, r.min.y), pos2(screen.max.x, r.max.y)),
                ] {
                    painter.rect_filled(part, 0.0, dim);
                }
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0, Color32::from_black_alpha(120)),
                    egui::StrokeKind::Outside,
                );
                painter.rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Middle,
                );
                if self.an.tool == Tool::Select {
                    for (_, hp) in handles(r) {
                        let hr = Rect::from_center_size(hp, vec2(8.0, 8.0));
                        painter.rect_filled(hr, 2.0, Color32::WHITE);
                        painter.rect_stroke(
                            hr,
                            2.0,
                            Stroke::new(1.0, Color32::from_black_alpha(140)),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
                size_label(&painter, r, s, screen);
            }
        }
        self.an.paint_live(
            &painter,
            &view,
            &shared.fonts,
            &shared.settings.draw,
            ui.input(|i| i.time),
        );

        // magnifier while choosing the area
        let choosing = self.sel.is_none() || matches!(self.drag, Drag::NewSel(_));
        if choosing {
            if let Some(p) = pointer {
                self.magnifier(&painter, &view, p, screen);
                let (cx, cy) = (p.x, p.y);
                let guide = Stroke::new(1.0, Color32::from_white_alpha(70));
                painter.line_segment([pos2(screen.min.x, cy), pos2(screen.max.x, cy)], guide);
                painter.line_segment([pos2(cx, screen.min.y), pos2(cx, screen.max.y)], guide);
            }
            if self.sel.is_none() {
                hint_bar(&ctx, screen);
            }
        }

        // ---------------- floating bars
        let mut switch = Switch::None;
        if let (Some(s), false) = (
            self.sel,
            matches!(self.drag, Drag::NewSel(_) | Drag::Adjust { .. }) || self.an.busy(),
        ) {
            let r = Rect::from_min_max(view.to_screen(s.min), view.to_screen(s.max));
            switch = self.bars(&ctx, frame, r, screen, shared);
        }

        // ---------------- keyboard
        if !typing && let Some(sw) = self.keys(ui, &ctx, frame, shared, pointer, &view) {
            switch = sw;
        }

        if shared.settings.draw.color != opts_snapshot.color
            || shared.settings.draw.width != opts_snapshot.width
        {
            shared.settings_dirty = true;
        }
        ctx.request_repaint();
        switch
    }

    fn selection_input(
        &mut self,
        ui: &Ui,
        canvas: &egui::Response,
        view: &View,
        pointer: Option<Pos2>,
    ) {
        let ctx = ui.ctx();
        let mods = ui.input(|i| i.modifiers);
        let bounds = Rect::from_min_size(Pos2::ZERO, self.img_size());
        let hit = |sel: Option<Rect>, p: Pos2| -> Option<Handle> {
            let s = sel?;
            let r = Rect::from_min_max(view.to_screen(s.min), view.to_screen(s.max));
            for (h, hp) in handles(r) {
                if hp.distance(p) <= 9.0 {
                    return Some(h);
                }
            }
            r.contains(p).then_some(Handle::Move)
        };
        if let Some(p) = pointer {
            let hit_now = hit(self.sel, p);
            let icon = match (&self.drag, hit_now.as_ref()) {
                (Drag::NewSel(_), _) => CursorIcon::Crosshair,
                (Drag::Adjust { handle, .. }, _) | (_, Some(handle)) => match *handle {
                    Handle::Move => CursorIcon::Move,
                    Handle::N | Handle::S => CursorIcon::ResizeVertical,
                    Handle::E | Handle::W => CursorIcon::ResizeHorizontal,
                    Handle::NE | Handle::SW => CursorIcon::ResizeNeSw,
                    Handle::NW | Handle::SE => CursorIcon::ResizeNwSe,
                },
                _ => CursorIcon::Crosshair,
            };
            ctx.set_cursor_icon(icon);
        }
        if canvas.drag_started() {
            let origin = ui
                .input(|i| i.pointer.press_origin())
                .or(pointer)
                .unwrap_or_default();
            self.drag = match hit(self.sel, origin) {
                Some(h) => Drag::Adjust {
                    handle: h,
                    start_ptr: view.to_img(origin),
                    orig: self.sel.unwrap(),
                },
                None => Drag::NewSel(view.to_img(origin).clamp(bounds.min, bounds.max)),
            };
        }
        if canvas.dragged()
            && let Some(p) = pointer
        {
            let cur = view.to_img(p).clamp(bounds.min, bounds.max);
            match &self.drag {
                Drag::NewSel(start) => {
                    let (a, b) = constrain(Tool::Select, *start, cur, mods.shift, mods.alt);
                    self.sel = Some(Rect::from_two_pos(a, b).intersect(bounds));
                }
                Drag::Adjust {
                    handle,
                    start_ptr,
                    orig,
                } => {
                    let d = view.to_img(p) - *start_ptr;
                    let mut r = *orig;
                    match handle {
                        Handle::Move => {
                            r = r.translate(d);
                            let off = vec2(
                                (bounds.min.x - r.min.x).max(0.0)
                                    + (bounds.max.x - r.max.x).min(0.0),
                                (bounds.min.y - r.min.y).max(0.0)
                                    + (bounds.max.y - r.max.y).min(0.0),
                            );
                            r = r.translate(off);
                        }
                        h => {
                            if matches!(h, Handle::N | Handle::NE | Handle::NW) {
                                r.min.y += d.y;
                            }
                            if matches!(h, Handle::S | Handle::SE | Handle::SW) {
                                r.max.y += d.y;
                            }
                            if matches!(h, Handle::W | Handle::NW | Handle::SW) {
                                r.min.x += d.x;
                            }
                            if matches!(h, Handle::E | Handle::NE | Handle::SE) {
                                r.max.x += d.x;
                            }
                            r = Rect::from_two_pos(r.min, r.max).intersect(bounds);
                        }
                    }
                    self.sel = Some(r);
                }
                Drag::None => {}
            }
        }
        if canvas.drag_stopped() {
            if let Some(s) = self.sel
                && (s.width() < 3.0 || s.height() < 3.0)
            {
                self.sel = None;
            }
            self.drag = Drag::None;
        }
        // a click without a selection grabs the whole screen (like a window-less fallback)
        if canvas.double_clicked() && self.sel.is_none() {
            self.sel = Some(bounds);
        }
    }

    fn magnifier(&self, painter: &egui::Painter, view: &View, p: Pos2, screen: Rect) {
        let img = view.to_img(p);
        let (iw, ih) = (
            self.shot.image.width() as i32,
            self.shot.image.height() as i32,
        );
        let (px, py) = (img.x.floor() as i32, img.y.floor() as i32);
        if px < 0 || py < 0 || px >= iw || py >= ih {
            return;
        }
        let cells = 15.0;
        let size = 135.0;
        let mut at = p + vec2(22.0, 22.0);
        if at.x + size > screen.max.x {
            at.x = p.x - 22.0 - size;
        }
        if at.y + size + 44.0 > screen.max.y {
            at.y = p.y - 22.0 - size - 44.0;
        }
        let r = Rect::from_min_size(at, vec2(size, size));
        let half = (cells - 1.0) / 2.0;
        let uv = Rect::from_min_max(
            pos2(
                (px as f32 - half) / iw as f32,
                (py as f32 - half) / ih as f32,
            ),
            pos2(
                (px as f32 + half + 1.0) / iw as f32,
                (py as f32 + half + 1.0) / ih as f32,
            ),
        );
        painter.rect_filled(r.expand(3.0), 10.0, Color32::from_rgb(22, 22, 25));
        painter.image(self.tex.id(), r, uv, Color32::WHITE);
        let cell = size / cells;
        let c = Rect::from_min_size(r.min + vec2(half * cell, half * cell), vec2(cell, cell));
        painter.rect_stroke(c, 0.0, Stroke::new(1.5, ACCENT), egui::StrokeKind::Outside);
        painter.rect_stroke(
            r.expand(3.0),
            10.0,
            Stroke::new(1.0, Color32::from_rgb(60, 60, 68)),
            egui::StrokeKind::Inside,
        );
        let px_col = self.shot.image.get_pixel(px as u32, py as u32).0;
        let info = Rect::from_min_size(pos2(r.min.x - 3.0, r.max.y + 8.0), vec2(size + 6.0, 38.0));
        painter.rect_filled(info, 8.0, Color32::from_rgb(22, 22, 25));
        painter.rect_filled(
            Rect::from_min_size(info.min + vec2(9.0, 11.0), vec2(16.0, 16.0)),
            4.0,
            c32(px_col),
        );
        painter.text(
            info.min + vec2(32.0, 9.0),
            Align2::LEFT_TOP,
            widgets::hex(px_col),
            FontId::monospace(12.0),
            TEXT,
        );
        painter.text(
            info.min + vec2(32.0, 24.0),
            Align2::LEFT_TOP,
            format!("{px}, {py}  ·  C copia"),
            FontId::proportional(10.5),
            MUTED,
        );
    }

    fn bars(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        r: Rect,
        screen: Rect,
        shared: &mut Shared,
    ) -> Switch {
        let mut switch = Switch::None;
        let gap = 10.0;
        let tb = self.toolbar_size;
        let show_opts = self.an.tool != Tool::Select;
        let ob = if show_opts {
            self.options_size
        } else {
            vec2(0.0, 0.0)
        };
        let stack_h = tb.y + if show_opts { ob.y + 6.0 } else { 0.0 };
        let below = r.max.y + gap + stack_h <= screen.max.y - 4.0;
        let above = r.min.y - gap - stack_h >= screen.min.y + 4.0;
        let x = (r.max.x - tb.x).clamp(
            screen.min.x + 8.0,
            (screen.max.x - tb.x - 8.0).max(screen.min.x + 8.0),
        );
        let (tb_y, ob_y) = if below {
            (r.max.y + gap, r.max.y + gap + tb.y + 6.0)
        } else if above {
            (r.min.y - gap - tb.y, r.min.y - gap - tb.y - 6.0 - ob.y)
        } else {
            (r.max.y - gap - tb.y, r.max.y - gap - tb.y - 6.0 - ob.y)
        };

        let resp = egui::Area::new(egui::Id::unique("tools"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(x, tb_y))
            .show(ctx, |ui| {
                widgets::floating_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        widgets::tool_buttons(ui, &mut self.an, &mut shared.settings.draw, true);
                        widgets::separator(ui);
                        if widgets::icon_button(ui, Icon::Undo, false, "Desfazer (Ctrl+Z)")
                            .clicked()
                        {
                            self.an.undo();
                        }
                        if widgets::icon_button(ui, Icon::Redo, false, "Refazer (Ctrl+Y)").clicked()
                        {
                            self.an.redo();
                        }
                        widgets::separator(ui);
                        if widgets::icon_button(ui, Icon::Pin, false, "Fixar na tela (Ctrl+T)")
                            .clicked()
                        {
                            switch = self.act_pin(shared);
                        }
                        if widgets::icon_button(ui, Icon::Save, false, "Salvar PNG (Ctrl+S)")
                            .clicked()
                        {
                            switch = self.act_save(frame, shared);
                        }
                        if widgets::icon_button(ui, Icon::Copy, false, "Copiar (Ctrl+C)").clicked()
                        {
                            switch = self.act_copy(shared);
                        }
                        let edit = egui::Button::new(
                            RichText::new("Editar").color(Color32::WHITE).strong(),
                        )
                        .fill(ACCENT)
                        .corner_radius(8)
                        .min_size(vec2(0.0, 32.0));
                        if ui
                            .add(edit)
                            .on_hover_text(
                                "Abrir no editor: fundos, moldura e marca d'água (Enter)",
                            )
                            .clicked()
                        {
                            switch = self.open_editor(ctx, shared);
                        }
                        ui.add_space(2.0);
                        if widgets::icon_button(ui, Icon::Close, false, "Fechar (Esc)").clicked() {
                            switch = Switch::Close;
                        }
                    });
                });
            });
        self.toolbar_size = resp.response.rect.size();

        if show_opts {
            let ox = (r.max.x - self.options_size.x).clamp(
                screen.min.x + 8.0,
                (screen.max.x - self.options_size.x - 8.0).max(screen.min.x + 8.0),
            );
            let tool = self.an.tool;
            let resp = egui::Area::new(egui::Id::unique("tool_opts"))
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(ox, ob_y))
                .show(ctx, |ui| {
                    widgets::floating_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            widgets::options_row(ui, tool, &mut shared.settings.draw, &shared.fonts)
                        });
                    });
                });
            self.options_size = resp.response.rect.size();
        }
        switch
    }

    fn keys(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        shared: &mut Shared,
        pointer: Option<Pos2>,
        view: &View,
    ) -> Option<Switch> {
        let cmd = Modifiers::COMMAND;
        if ui.input_mut(|i| i.consume_key(cmd, Key::Z)) {
            self.an.undo();
        }
        if ui.input_mut(|i| {
            i.consume_key(cmd, Key::Y) || i.consume_key(cmd | Modifiers::SHIFT, Key::Z)
        }) {
            self.an.redo();
        }
        if ui.input_mut(|i| i.consume_key(cmd, Key::A)) {
            self.sel = Some(Rect::from_min_size(Pos2::ZERO, self.img_size()));
        }
        if self.sel.is_some() {
            if ui.input_mut(|i| i.consume_key(cmd, Key::C)) {
                return Some(self.act_copy(shared));
            }
            if ui.input_mut(|i| i.consume_key(cmd, Key::S)) {
                return Some(self.act_save(frame, shared));
            }
            if ui.input_mut(|i| i.consume_key(cmd, Key::T)) {
                return Some(self.act_pin(shared));
            }
            if ui.input_mut(|i| {
                i.consume_key(cmd, Key::E) || i.consume_key(Modifiers::NONE, Key::Enter)
            }) {
                return Some(self.open_editor(ctx, shared));
            }
        }
        if ui.input_mut(|i| i.consume_key(cmd, Key::X)) {
            return Some(Switch::Close);
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.an.tool != Tool::Select && self.sel.is_some() {
                self.an.set_tool(Tool::Select, &mut shared.settings.draw);
            } else {
                return Some(Switch::Close);
            }
        }
        if self.sel.is_none() && ui.input(|i| i.key_pressed(Key::C) && !i.modifiers.any()) {
            if let Some(p) = pointer {
                let ip = view.to_img(p);
                if ip.x >= 0.0
                    && ip.y >= 0.0
                    && (ip.x as u32) < self.shot.image.width()
                    && (ip.y as u32) < self.shot.image.height()
                {
                    let c = self.shot.image.get_pixel(ip.x as u32, ip.y as u32).0;
                    crate::system::actions::copy_text(&widgets::hex(c));
                    shared.toast(format!("Cor {} copiada", widgets::hex(c)));
                }
            }
            return None;
        }
        if self.sel.is_some() {
            widgets::tool_shortcuts(ui, &mut self.an, &mut shared.settings.draw, true);
            // arrows move the selection, Shift+arrows resize it
            if self.an.tool == Tool::Select {
                let mut d = vec2(0.0, 0.0);
                let shift = ui.input(|i| i.modifiers.shift);
                for (k, v) in [
                    (Key::ArrowLeft, vec2(-1.0, 0.0)),
                    (Key::ArrowRight, vec2(1.0, 0.0)),
                    (Key::ArrowUp, vec2(0.0, -1.0)),
                    (Key::ArrowDown, vec2(0.0, 1.0)),
                ] {
                    if ui.input(|i| i.key_pressed(k)) {
                        d += v;
                    }
                }
                if d != vec2(0.0, 0.0) {
                    let bounds = Rect::from_min_size(Pos2::ZERO, self.img_size());
                    if let Some(s) = &mut self.sel {
                        if shift {
                            s.max += d;
                        } else {
                            *s = s.translate(d);
                        }
                        *s = Rect::from_two_pos(s.min, s.max).intersect(bounds);
                    }
                }
            }
        }
        None
    }

    fn act_copy(&mut self, shared: &mut Shared) -> Switch {
        self.an.commit_draft(&shared.settings.draw);
        self.flush(shared);
        if let Some(img) = self.output(shared) {
            match crate::system::actions::copy_image(&img) {
                Ok(()) => {
                    self.remember_selection(shared);
                    return if shared.settings.show_bubbles {
                        Switch::ToNotice(Box::new(crate::screens::notice::Notice::copied()))
                    } else {
                        Switch::Close
                    };
                }
                Err(e) => shared.toast(format!("Não consegui copiar: {e}")),
            }
        }
        Switch::None
    }

    fn act_save(&mut self, frame: &eframe::Frame, shared: &mut Shared) -> Switch {
        self.an.commit_draft(&shared.settings.draw);
        self.flush(shared);
        let Some(img) = self.output(shared) else {
            return Switch::None;
        };
        let path = if shared.settings.auto_save {
            Some(crate::system::actions::auto_path(&shared.settings.save_dir))
        } else {
            crate::system::actions::ask_save_path(&shared.settings.save_dir, Some(frame))
        };
        let Some(path) = path else {
            return Switch::None;
        };
        match crate::system::actions::save_png(&img, &path) {
            Ok(()) => {
                if let Some(dir) = path.parent() {
                    shared.settings.save_dir = dir.to_string_lossy().into_owned();
                }
                self.remember_selection(shared);
                if shared.settings.show_bubbles {
                    Switch::ToNotice(Box::new(crate::screens::notice::Notice::saved(path)))
                } else {
                    Switch::Close
                }
            }
            Err(e) => {
                shared.toast(format!("Não consegui salvar: {e}"));
                Switch::None
            }
        }
    }

    fn act_pin(&mut self, shared: &mut Shared) -> Switch {
        self.an.commit_draft(&shared.settings.draw);
        self.flush(shared);
        let Some(img) = self.crop() else {
            return Switch::None;
        };
        if let Some(p) = crate::system::actions::temp_png(&img) {
            let _ = crate::system::actions::spawn_self(&["--pin", &p.to_string_lossy()]);
            self.remember_selection(shared);
            return Switch::Close;
        }
        Switch::None
    }

    /// Bake pending changes right away (actions can't wait for the next frame).
    fn flush(&mut self, shared: &Shared) {
        if let Some(anns) = self.an.bake() {
            self.baked =
                crate::engine::render::render_annotations(&self.shot.image, &anns, &shared.fonts);
        }
    }
}

fn handles(r: Rect) -> [(Handle, Pos2); 8] {
    let c = r.center();
    [
        (Handle::NW, r.left_top()),
        (Handle::N, pos2(c.x, r.top())),
        (Handle::NE, r.right_top()),
        (Handle::E, pos2(r.right(), c.y)),
        (Handle::SE, r.right_bottom()),
        (Handle::S, pos2(c.x, r.bottom())),
        (Handle::SW, r.left_bottom()),
        (Handle::W, pos2(r.left(), c.y)),
    ]
}

fn size_label(painter: &egui::Painter, r: Rect, s: Rect, screen: Rect) {
    let text = format!("{} × {}", s.width().round(), s.height().round());
    let font = FontId::new(12.0, egui::FontFamily::Name("bold".into()));
    let galley = painter.layout_no_wrap(
        text,
        font,
        Color32::from_rgb(SIZE_LABEL_TEXT[0], SIZE_LABEL_TEXT[1], SIZE_LABEL_TEXT[2]),
    );
    let size = galley.size() + vec2(14.0, 8.0);
    let mut at = pos2(r.min.x, r.min.y - size.y - 6.0);
    if at.y < screen.min.y + 2.0 {
        at = pos2(r.min.x + 6.0, r.min.y + 6.0);
    }
    let bg = Rect::from_min_size(at, size);
    let f = SIZE_LABEL_FILL;
    painter.rect_filled(
        bg,
        6.0,
        Color32::from_rgba_unmultiplied(f[0], f[1], f[2], f[3]),
    );
    painter.galley(at + vec2(7.0, 4.0), galley, Color32::WHITE);
}

fn hint_bar(ctx: &egui::Context, _screen: Rect) {
    egui::Area::new(egui::Id::unique("hint"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .anchor(Align2::CENTER_TOP, vec2(0.0, 22.0))
        .show(ctx, |ui| {
            widgets::floating_frame()
                .inner_margin(egui::Margin::symmetric(14, 9))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let k = |ui: &mut Ui, key: &str, what: &str| {
                            ui.label(RichText::new(key).color(Color32::WHITE).strong().size(12.5));
                            ui.label(RichText::new(what).color(MUTED).size(12.5));
                            ui.add_space(6.0);
                        };
                        k(ui, "Arraste", "selecionar");
                        k(ui, "Shift", "quadrado");
                        k(ui, "Ctrl+A", "tela inteira");
                        k(ui, "C", "copiar cor");
                        k(ui, "Esc", "sair");
                    });
                });
        });
}

pub fn color_image(img: &RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    )
}

pub fn load_tex(
    ctx: &egui::Context,
    name: &str,
    img: &RgbaImage,
    opts: egui::TextureOptions,
) -> egui::TextureHandle {
    ctx.load_texture(name, color_image(img), opts)
}
