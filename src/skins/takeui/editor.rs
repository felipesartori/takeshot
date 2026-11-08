//! TakeUI's editor: the title bar (file and size, undo and redo, pin, save as, save, copy, window
//! buttons), the tool bar (tools, colours, widths, zoom), the canvas, the right panel and the
//! status bar, as in "Editor · escuro" of the design. Behaviour is in `screens::editor`.

use super::tool_icon;
use crate::Shared;
use crate::engine::doc::ObjId;
use crate::engine::model::{BlurMode, Tool, WIDTHS};
use crate::screens::editor::{
    Editor, EditorAction, PanelTab, Zoom, object_label, tool_hint, tool_name,
};
use crate::screens::home::MainAction;
use crate::system::fonts::FontLib;
use crate::system::settings::DrawOpts;
use crate::ui::chrome::{self, BAR_HEIGHT};
use crate::ui::i18n::{Msg, t as tr, tf};
use crate::ui::kit::{self, BtnKind, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::Tokens;
use egui::{Align, Align2, Color32, Frame, Layout, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

/// Heights of the tool bar and the status bar, and the right panel's width.
pub const TOOL_BAR: f32 = 48.0;
pub const STATUS_BAR: f32 = 30.0;
pub const PANEL: f32 = 320.0;

pub fn editor(ui: &mut Ui, t: &Tokens, ed: &mut Editor, shared: &mut Shared) -> Vec<EditorAction> {
    let mut acts = Vec::new();
    egui::Panel::top("takeui-editor-title")
        .exact_size(BAR_HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(t.shell))
        .show(ui, |ui| title_bar(ui, t, ed, &mut acts));
    egui::Panel::top("takeui-editor-tools")
        .exact_size(TOOL_BAR)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(t.surf))
        .show(ui, |ui| tool_bar(ui, t, ed, shared, &mut acts));
    egui::Panel::bottom("takeui-editor-status")
        .exact_size(STATUS_BAR)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(t.surf))
        .show(ui, |ui| status_bar(ui, t, ed));
    egui::Panel::right("takeui-editor-panel")
        .exact_size(PANEL)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(t.surf))
        .show(ui, |ui| panel(ui, t, ed, shared, &mut acts));
    egui::CentralPanel::no_frame().show(ui, |ui| acts.extend(ed.canvas(ui, t, shared)));
    object_bar(ui, t, ed, shared, &mut acts);
    chrome::resize_edges(ui);
    acts
}

/// "Ctrl+S" on Windows and Linux, "Cmd+S" on macOS.
fn cmd(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("Cmd+{key}")
    } else {
        format!("Ctrl+{key}")
    }
}

fn title_bar(ui: &mut Ui, t: &Tokens, ed: &Editor, acts: &mut Vec<EditorAction>) {
    chrome::drag_area(ui, ui.max_rect());
    let row = ui.max_rect();
    // The actions show their labels and shortcuts when the name keeps at least 160 px beside
    // them; otherwise Pin and Save as become icons and the shortcuts go. The full width is the
    // one measured the last time it was drawn.
    let full_id = egui::Id::unique("takeui-editor-actions");
    let full_w = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(full_id))
        .unwrap_or(620.0);
    let lead = 16.0 + chrome::leading_inset() + 22.0 + 16.0;
    let history = 6.0 + 32.0 + 2.0 + 32.0 + 12.0;
    let window = if chrome::draws_window_buttons() {
        3.0 * 44.0 + 2.0 * 4.0 + 24.0
    } else {
        12.0
    };
    let compact = row.width() < lead + 160.0 + history + full_w + window;
    let right = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(row)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if chrome::draws_window_buttons() {
                    if let Some(c) = chrome::window_buttons(ui, t) {
                        acts.push(EditorAction::Window(c));
                    }
                    ui.add_space(10.0);
                    let (line, _) = ui.allocate_exact_size(vec2(1.0, 20.0), egui::Sense::hover());
                    ui.painter().rect_filled(line, 0.0, t.border);
                    ui.add_space(6.0);
                } else {
                    ui.add_space(12.0);
                }
                ui.spacing_mut().item_spacing.x = 6.0;
                let w = ui
                    .scope(|ui| actions(ui, t, compact, acts))
                    .response
                    .rect
                    .width();
                if !compact {
                    ui.ctx().data_mut(|d| d.insert_temp(full_id, w));
                }
            },
        )
        .response
        .rect;
    let left = Rect::from_min_max(
        row.min,
        pos2((right.left() - 12.0).max(row.left()), row.max.y),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(left)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_space(16.0 + chrome::leading_inset());
            let mark = ed_mark(ui, t);
            ui.add(egui::Image::new((mark, vec2(22.0, 22.0))));
            ui.add_space(10.0);
            let [w, h] = ed.size();
            let room = (ui.available_width() - history).max(0.0);
            name_and_size(ui, t, ed.name(), &format!("· {w} × {h}"), room);
            ui.add_space(12.0);
            ui.spacing_mut().item_spacing.x = 2.0;
            if kit::icon_button(
                ui,
                t,
                Ico::Undo,
                false,
                &format!("{} ({})", tr(Msg::Undo), cmd("Z")),
            )
            .clicked()
            {
                acts.push(EditorAction::Undo);
            }
            if kit::icon_button(
                ui,
                t,
                Ico::Redo,
                false,
                &format!("{} ({})", tr(Msg::Redo), cmd("Y")),
            )
            .clicked()
            {
                acts.push(EditorAction::Redo);
            }
        },
    );
}

/// The theme's mark, cached per theme and mode in egui's memory.
fn ed_mark(ui: &Ui, t: &Tokens) -> egui::TextureId {
    let coral = t.theme == crate::ui::tokens::ThemeId::Coral;
    let id = egui::Id::unique(("takeui-editor-mark", coral, t.dark));
    if let Some(tex) = ui.ctx().data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return tex.id();
    }
    let tex = crate::ui::brand::mark_texture(ui.ctx(), t.theme, t.dark);
    let tid = tex.id();
    ui.ctx().data_mut(|d| d.insert_temp(id, tex));
    tid
}

/// The file name (13 medium) and the size (13, muted), cut with "…" to `room`.
fn name_and_size(ui: &mut Ui, t: &Tokens, name: &str, size: &str, room: f32) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        name,
        0.0,
        egui::TextFormat::simple(font(13.0, Weight::Medium), t.text),
    );
    job.append(
        size,
        6.0,
        egui::TextFormat::simple(font(13.0, Weight::Regular), t.faint),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(room.max(1.0));
    let g = ui.painter().layout_job(job);
    let (r, _) = ui.allocate_exact_size(vec2(g.size().x, BAR_HEIGHT), egui::Sense::hover());
    ui.painter()
        .galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, t.text);
}

/// Copy (primary), Save, Save as and Pin, from the right; compact drops labels and shortcuts.
fn actions(ui: &mut Ui, t: &Tokens, compact: bool, acts: &mut Vec<EditorAction>) {
    let copy_keys = cmd("C");
    let save_keys = cmd("S");
    let pin_keys = cmd("T");
    let keys = |k: &str| (!compact).then(|| k.to_string());
    if kit::button(
        ui,
        t,
        BtnKind::Primary,
        Some(Ico::Copy),
        tr(Msg::Copy),
        keys(&copy_keys).as_deref(),
    )
    .clicked()
    {
        acts.push(EditorAction::Copy);
    }
    if kit::button(
        ui,
        t,
        BtnKind::Secondary,
        Some(Ico::Download),
        tr(Msg::Save),
        keys(&save_keys).as_deref(),
    )
    .clicked()
    {
        acts.push(EditorAction::Save);
    }
    if compact {
        if kit::icon_button(ui, t, Ico::FileDown, false, tr(Msg::SaveAs)).clicked() {
            acts.push(EditorAction::SaveAs);
        }
        let tip = format!("{} ({pin_keys})", tr(Msg::PinOnScreen));
        if kit::icon_button(ui, t, Ico::Pin, false, &tip).clicked() {
            acts.push(EditorAction::Pin);
        }
    } else {
        if kit::button(
            ui,
            t,
            BtnKind::Secondary,
            Some(Ico::FileDown),
            tr(Msg::SaveAs),
            None,
        )
        .clicked()
        {
            acts.push(EditorAction::SaveAs);
        }
        if kit::button(
            ui,
            t,
            BtnKind::Secondary,
            Some(Ico::Pin),
            tr(Msg::PinOnScreen),
            Some(&pin_keys),
        )
        .clicked()
        {
            acts.push(EditorAction::Pin);
        }
    }
}

/// The palette as editor actions.
fn palette(ui: &mut Ui, t: &Tokens, color: [u8; 4], acts: &mut Vec<EditorAction>) {
    if let Some(c) = super::palette(ui, t, color) {
        acts.push(EditorAction::SetColor(c));
    }
}

fn divider(ui: &mut Ui, t: &Tokens) {
    super::divider(ui, t, 22.0);
}

fn tool_bar(ui: &mut Ui, t: &Tokens, ed: &Editor, shared: &Shared, acts: &mut Vec<EditorAction>) {
    let r = ui.max_rect();
    ui.painter()
        .hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, t.border));
    let opts = &shared.settings.draw;
    // The width every part needs, measured the last time all of them were drawn; narrower, the
    // width dots go (the panel's Width slider stays).
    let full_id = egui::Id::unique("takeui-editor-tool-bar");
    let full_w = ui.ctx().data(|d| d.get_temp::<f32>(full_id));
    let compact = full_w.is_some_and(|w| r.width() < w);
    ui.horizontal_centered(|ui| {
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.x = 2.0;
        for tool in [Tool::Select].into_iter().chain(Tool::DRAWING) {
            let tip = format!("{} ({})", tr(tool_name(tool)), tool.key_name());
            if kit::tool_button(ui, t, tool_icon(tool), ed.an.tool == tool, &tip, false).clicked() {
                acts.push(EditorAction::SetTool(tool));
            }
        }
        ui.add_space(8.0);
        divider(ui, t);
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        palette(ui, t, opts.color, acts);
        if !compact {
            ui.add_space(4.0);
            divider(ui, t);
            ui.add_space(4.0);
            ui.spacing_mut().item_spacing.x = 2.0;
            for w in WIDTHS {
                let tip = format!("{w} px");
                if kit::width_dot(ui, t, w, (opts.width - w).abs() < 0.01, &tip).clicked() {
                    acts.push(EditorAction::SetWidth(w));
                }
            }
        }
        let left_end = ui.cursor().left();
        let right = ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(12.0);
            ui.spacing_mut().item_spacing.x = 4.0;
            if kit::icon_button(ui, t, Ico::Maximize, ed.zoom().is_none(), tr(Msg::ZoomFit))
                .clicked()
            {
                acts.push(EditorAction::Zoom(Zoom::Fit));
            }
            let pct = (ed.shown_scale() * 100.0).round() as u32;
            kit::combo(
                ui,
                t,
                "takeui-editor-zoom",
                &format!("{pct} %"),
                56.0,
                30.0,
                |ui| {
                    if ui
                        .selectable_label(ed.zoom().is_none(), tr(Msg::ZoomFit))
                        .clicked()
                    {
                        acts.push(EditorAction::Zoom(Zoom::Fit));
                    }
                    for p in [25, 50, 75, 100, 150, 200, 300, 400] {
                        if ui.selectable_label(pct == p, format!("{p} %")).clicked() {
                            acts.push(EditorAction::Zoom(Zoom::Percent(p)));
                        }
                    }
                },
            );
        });
        if !compact {
            // The parts, 16 px apart at the least.
            let need = (left_end - r.left()) + 16.0 + (r.right() - right.response.rect.left());
            ui.ctx().data_mut(|d| d.insert_temp(full_id, need));
        }
    });
}

fn status_bar(ui: &mut Ui, t: &Tokens, ed: &Editor) {
    let r = ui.max_rect();
    ui.painter()
        .hline(r.x_range(), r.top() + 0.5, Stroke::new(1.0, t.border));
    let doc = &ed.an.doc;
    let hint = match doc.selected() {
        [one] => match doc.kind(*one) {
            Some((kind, _)) => tf(Msg::HintObject, &[tr(tool_name(kind))]),
            None => tr(tool_hint(ed.an.tool)).to_string(),
        },
        _ => tr(tool_hint(ed.an.tool)).to_string(),
    };
    let n = doc.objs().len();
    let count = if n == 1 {
        tr(Msg::ObjectsOne).to_string()
    } else {
        tf(Msg::ObjectsMany, &[&n.to_string()])
    };
    let [w, h] = ed.size();
    ui.horizontal_centered(|ui| {
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(hint)
                .font(font(12.0, Weight::Regular))
                .color(t.muted),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.label(
                egui::RichText::new(format!("{w} × {h}"))
                    .font(font(12.0, Weight::Regular))
                    .color(t.faint),
            );
            ui.label(
                egui::RichText::new(count)
                    .font(font(12.0, Weight::Regular))
                    .color(t.faint),
            );
        });
    });
}

/// The bar over the selected object (its colour, Duplicate, Bring to front, Send to back, Delete),
/// as in "Editor · escuro": above it, or below when there is no room, inside the canvas; hidden
/// while something is drawn or dragged, or while the selection is out of view.
fn object_bar(ui: &mut Ui, t: &Tokens, ed: &Editor, shared: &Shared, acts: &mut Vec<EditorAction>) {
    let Some(sel) = ed
        .selection_on_screen(shared)
        .filter(|s| s.intersects(ed.canvas_area()))
    else {
        return;
    };
    // 5 x 28 px buttons 2 apart, 4 px inside.
    let size = vec2(4.0 + 5.0 * 28.0 + 4.0 * 2.0 + 4.0, 36.0);
    // The handles stand up to 6 px outside the object.
    let at =
        crate::engine::placement::object_bar(sel.expand(6.0), size, ed.canvas_area().shrink(8.0));
    let color = shared.settings.draw.color;
    egui::Area::new(egui::Id::unique("takeui-object-bar"))
        .order(egui::Order::Foreground)
        .fixed_pos(at.min)
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(t.surf)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(10)
                .inner_margin(4)
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: Color32::from_black_alpha(64),
                })
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    ui.horizontal(|ui| {
                        let dot = Color32::from_rgb(color[0], color[1], color[2]);
                        let pick = kit::bar_button(ui, t, kit::BarItem::Color(dot), tr(Msg::Color));
                        egui::Popup::from_toggle_button_response(&pick)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .show(|ui| {
                                // The picker sits in this popup: egui keeps one popup open, so
                                // a "+" here could not open another.
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    if let Some(c) = super::palette_dots(ui, t, color) {
                                        acts.push(EditorAction::SetColor(c));
                                    }
                                });
                                let mut col = Color32::from_rgb(color[0], color[1], color[2]);
                                if egui::color_picker::color_picker_color32(
                                    ui,
                                    &mut col,
                                    egui::color_picker::Alpha::Opaque,
                                ) {
                                    acts.push(EditorAction::SetColor(col.to_array()));
                                }
                            });
                        let duplicate = format!("{} ({})", tr(Msg::Duplicate), cmd("D"));
                        for (icon, tip, act) in [
                            (Ico::CopyPlus, duplicate.as_str(), EditorAction::Duplicate),
                            (
                                Ico::BringFront,
                                tr(Msg::BringToFront),
                                EditorAction::ToFront,
                            ),
                            (Ico::SendBack, tr(Msg::SendToBack), EditorAction::ToBack),
                            (
                                Ico::Trash,
                                &format!("{} (Del)", tr(Msg::Delete)),
                                EditorAction::Delete,
                            ),
                        ] {
                            if kit::bar_button(ui, t, kit::BarItem::Icon(icon), tip).clicked() {
                                acts.push(act);
                            }
                        }
                    });
                });
        });
}

/// The right panel, as in "Editor · escuro": the Object and Style tabs 12 px in, then the open
/// tab's page, 20 px in, which scrolls.
fn panel(ui: &mut Ui, t: &Tokens, ed: &Editor, shared: &mut Shared, acts: &mut Vec<EditorAction>) {
    let r = ui.max_rect();
    ui.painter()
        .vline(r.left() + 0.5, r.y_range(), Stroke::new(1.0, t.border));
    let tabs = Rect::from_min_size(r.min + vec2(12.0, 12.0), vec2(r.width() - 24.0, 29.0));
    let mut tab = ed.tab();
    ui.scope_builder(UiBuilder::new().max_rect(tabs), |ui| {
        let items = [
            (PanelTab::Object, tr(Msg::ObjectTab)),
            (PanelTab::Style, tr(Msg::Style)),
        ];
        if kit::tab_bar(ui, t, &mut tab, &items).changed() {
            acts.push(EditorAction::Tab(tab));
        }
    });
    let body = Rect::from_min_max(pos2(r.left(), tabs.bottom()), r.max);
    ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical()
            .id_salt(("takeui-editor-panel", tab == PanelTab::Style))
            .auto_shrink([false, false])
            .scroll_bar_rect(body.shrink2(vec2(0.0, 8.0)))
            .show(ui, |ui| {
                Frame::new()
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 0,
                        top: 20,
                        bottom: 20,
                    })
                    .show(ui, |ui| {
                        ui.set_width((r.width() - 40.0).max(0.0));
                        match tab {
                            PanelTab::Object => object_page(ui, t, ed, shared, acts),
                            PanelTab::Style => style_page(ui, t, shared, acts),
                        }
                    });
            });
    });
}

fn style_page(ui: &mut Ui, t: &Tokens, shared: &mut Shared, acts: &mut Vec<EditorAction>) {
    let mut main = Vec::new();
    if super::style::controls(ui, t, shared, &mut main) {
        acts.push(EditorAction::StyleChanged);
    }
    for a in main {
        if a == MainAction::ChooseLogo {
            acts.push(EditorAction::ChooseLogo);
        }
    }
}

/// The Object tab: what is selected (or the tool in hand), its options, Duplicate and Delete,
/// then the list of objects. Groups sit 20 px apart.
fn object_page(
    ui: &mut Ui,
    t: &Tokens,
    ed: &Editor,
    shared: &Shared,
    acts: &mut Vec<EditorAction>,
) {
    ui.spacing_mut().item_spacing = vec2(6.0, 20.0);
    header(ui, t, ed);
    if let Some(tool) = ed.an.bar_tool() {
        tool_options(ui, t, tool, &shared.settings.draw, &shared.fonts, acts);
    }
    if !ed.an.doc.selected().is_empty() {
        // One row, as in the design; with their key badges the two run past the 280 px column,
        // so the keys go in the tooltips.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if kit::button(
                ui,
                t,
                BtnKind::Secondary,
                Some(Ico::CopyPlus),
                tr(Msg::Duplicate),
                None,
            )
            .on_hover_text(format!("{} ({})", tr(Msg::Duplicate), cmd("D")))
            .clicked()
            {
                acts.push(EditorAction::Duplicate);
            }
            if kit::button(
                ui,
                t,
                BtnKind::Secondary,
                Some(Ico::Trash),
                tr(Msg::Delete),
                None,
            )
            .on_hover_text(format!("{} (Del)", tr(Msg::Delete)))
            .clicked()
            {
                acts.push(EditorAction::Delete);
            }
        });
    }
    let (rule, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rule, 0.0, t.line);
    object_list(ui, t, ed, acts);
}

/// The icon (18 px), the name (15 bold) and, for one object, where it stands ("2 of 3").
fn header(ui: &mut Ui, t: &Tokens, ed: &Editor) {
    let doc = &ed.an.doc;
    let (kind, title, count) = match doc.selected() {
        [] => (
            Some(ed.an.tool),
            tr(tool_name(ed.an.tool)).to_string(),
            None,
        ),
        [one] => match doc.kind(*one) {
            Some((kind, text)) => (
                Some(kind),
                object_label(kind, text.as_deref()),
                doc.position_of(*one)
                    .map(|(i, n)| tf(Msg::PositionOf, &[&i.to_string(), &n.to_string()])),
            ),
            None => (None, String::new(), None),
        },
        many => (
            Some(Tool::Select),
            tf(Msg::SelectedMany, &[&many.len().to_string()]),
            None,
        ),
    };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    let p = ui.painter();
    let icon_at = pos2(rect.left() + 9.0, rect.center().y);
    match kind.map(tool_icon) {
        Some(Some(i)) => {
            p.text(
                icon_at,
                Align2::CENTER_CENTER,
                crate::ui::icon::glyph(i),
                crate::ui::icon::font(18.0),
                t.text,
            );
        }
        Some(None) => kit::step_glyph(p, icon_at, 17.0, t.text),
        None => {}
    }
    let mut right = rect.right();
    if let Some(c) = count {
        let g = p.layout_no_wrap(c, font(12.0, Weight::Regular), t.faint);
        right -= g.size().x + 12.0;
        p.galley(
            pos2(
                rect.right() - g.size().x,
                rect.center().y - g.size().y / 2.0,
            ),
            g,
            t.faint,
        );
    }
    let mut job = egui::text::LayoutJob::simple_singleline(title, font(15.0, Weight::Bold), t.text);
    job.wrap = egui::text::TextWrapping::truncate_at_width((right - rect.left() - 28.0).max(1.0));
    let g = p.layout_job(job);
    p.galley(
        pos2(rect.left() + 28.0, rect.center().y - g.size().y / 2.0),
        g,
        t.text,
    );
}

/// Frames a slider's change (`r`, now at `v`) so its whole drag undoes in one step: grabbing and
/// letting go wrap the changes in between.
fn gesture(
    r: egui::Response,
    v: f32,
    acts: &mut Vec<EditorAction>,
    set: impl Fn(f32) -> EditorAction,
) {
    if r.drag_started() {
        acts.push(EditorAction::Gesture(true));
    }
    if r.changed() {
        acts.push(set(v));
    }
    if r.drag_stopped() {
        acts.push(EditorAction::Gesture(false));
    }
}

/// A 13 px label on the left and a switch on the right; returns the new state when flipped.
fn switch_row(ui: &mut Ui, t: &Tokens, label: &str, on: bool) -> Option<bool> {
    let mut v = on;
    let changed = ui
        .horizontal(|ui| {
            ui.label(
                egui::RichText::new(label)
                    .font(font(13.0, Weight::Regular))
                    .color(t.text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                kit::switch(ui, t, &mut v).changed()
            })
            .inner
        })
        .inner;
    changed.then_some(v)
}

/// The options of `tool`, for what is selected or what will be drawn next.
fn tool_options(
    ui: &mut Ui,
    t: &Tokens,
    tool: Tool,
    opts: &DrawOpts,
    fonts: &FontLib,
    acts: &mut Vec<EditorAction>,
) {
    if tool != Tool::Blur {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(
                egui::RichText::new(tr(Msg::Color))
                    .font(font(13.0, Weight::Regular))
                    .color(t.text),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                palette(ui, t, opts.color, acts);
            });
        });
    }
    match tool {
        Tool::Text => {
            let mut v = opts.font_size;
            let r = kit::slider_row(ui, t, tr(Msg::Size), &mut v, 8.0..=120.0, "px");
            gesture(r, v, acts, |v| EditorAction::SetFontSize(v.round()));
        }
        Tool::Blur => {}
        _ => {
            let label = if tool == Tool::Counter {
                Msg::Size
            } else {
                Msg::Width
            };
            let mut v = opts.width;
            let r = kit::slider_row(ui, t, tr(label), &mut v, 1.0..=40.0, "px");
            gesture(r, v, acts, |v| EditorAction::SetWidth(v.round().max(1.0)));
        }
    }
    if tool != Tool::Blur {
        let mut v = (opts.color[3] as f32 * 100.0 / 255.0).round();
        let r = kit::slider_row(ui, t, tr(Msg::Opacity), &mut v, 0.0..=100.0, "%");
        gesture(r, v, acts, |v| {
            EditorAction::SetOpacity((v * 255.0 / 100.0).round() as u8)
        });
    }
    match tool {
        Tool::Rect | Tool::Ellipse => {
            if let Some(on) = switch_row(ui, t, tr(Msg::Fill), opts.filled) {
                acts.push(EditorAction::SetFill(on));
            }
        }
        Tool::Text => {
            let names: Vec<(usize, &str)> = fonts
                .list
                .iter()
                .enumerate()
                .map(|(i, f)| (i, f.name.as_str()))
                .collect();
            let mut f = opts.font;
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.label(
                    egui::RichText::new(tr(Msg::Font))
                        .font(font(13.0, Weight::Regular))
                        .color(t.text),
                );
                let w = ui.available_width();
                if kit::select(ui, t, "takeui-editor-font", &mut f, &names, w).changed() {
                    acts.push(EditorAction::SetFont(f));
                }
            });
            if let Some(on) = switch_row(ui, t, tr(Msg::TextBackground), opts.text_box) {
                acts.push(EditorAction::SetTextBox(on));
            }
        }
        Tool::Blur => {
            let mut m = opts.blur_mode;
            let items = [
                (BlurMode::Blur, tr(Msg::ToolBlur)),
                (BlurMode::Pixelate, tr(Msg::Pixelate)),
            ];
            if kit::segmented(ui, t, &mut m, &items).changed() {
                acts.push(EditorAction::SetBlurMode(m));
            }
        }
        _ => {}
    }
}

/// "Objects": a row per object, front first; a click picks it (Shift adds it), the eye hides or
/// shows it, the right button brings it to the front or sends it to the back. When the selection
/// changes, its row scrolls into view.
fn object_list(ui: &mut Ui, t: &Tokens, ed: &Editor, acts: &mut Vec<EditorAction>) {
    let doc = &ed.an.doc;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        ui.label(
            egui::RichText::new(tr(Msg::ObjectList))
                .font(font(13.0, Weight::Bold))
                .color(t.text),
        );
        if doc.objs().is_empty() {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tr(Msg::NoObjects))
                        .font(font(12.0, Weight::Regular))
                        .color(t.faint),
                )
                .wrap(),
            );
            return;
        }
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let seen_id = egui::Id::unique("takeui-editor-list-selection");
            let seen = ui.ctx().data(|d| d.get_temp::<Vec<ObjId>>(seen_id));
            let moved = seen.as_deref() != Some(doc.selected());
            // The new selection counts as seen once its row is in view; until then it asks to
            // scroll there (a request in a frame egui discards would otherwise be lost).
            let mut in_view = doc.selected().is_empty();
            for o in doc.objs().iter().rev() {
                let Some((kind, text)) = doc.kind(o.id) else {
                    continue;
                };
                let selected = doc.selected().contains(&o.id);
                let label = object_label(kind, text.as_deref());
                let row = kit::object_row(ui, t, tool_icon(kind), &label, selected, o.hidden);
                if row.eye.clicked() {
                    acts.push(EditorAction::Hide(o.id, !o.hidden));
                } else if row.row.clicked() {
                    let add = ui.input(|i| i.modifiers.shift);
                    acts.push(EditorAction::Select(o.id, add));
                }
                if moved && doc.selected().first() == Some(&o.id) {
                    in_view = ui.clip_rect().contains_rect(row.row.rect);
                    if !in_view {
                        // A jump, not an animation: motion is only the 80 ms colour ease.
                        ui.scroll_to_rect_animation(
                            row.row.rect,
                            None,
                            egui::style::ScrollAnimation::none(),
                        );
                    }
                }
                let tip = if o.hidden {
                    Msg::ShowObject
                } else {
                    Msg::HideObject
                };
                row.eye.on_hover_text(tr(tip));
                // A hidden object cannot be picked, so it has nothing to reorder.
                if !o.hidden {
                    row.row.context_menu(|ui| {
                        for (msg, act) in [
                            (Msg::BringToFront, EditorAction::ToFront),
                            (Msg::SendToBack, EditorAction::ToBack),
                        ] {
                            if ui.button(tr(msg)).clicked() {
                                acts.push(EditorAction::Select(o.id, false));
                                acts.push(act);
                                ui.close();
                            }
                        }
                    });
                }
            }
            if moved && in_view {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(seen_id, doc.selected().to_vec()));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::annotator::TextDraft;
    use crate::engine::doc::Doc;
    use crate::engine::model::{Ann, Shape};
    use crate::screens::editor::PanelTab;
    use crate::skins::takeui::testing::{click, painted_at, run_editor};
    use crate::ui::i18n::every;
    use egui::{Event, Key, Modifiers, Pos2};

    fn editor() -> (Editor, Shared) {
        let shared = Shared::for_tests();
        let base = image::RgbaImage::from_pixel(1200, 740, image::Rgba([240, 240, 240, 255]));
        let ann = |shape| Ann {
            shape,
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: false,
        };
        let doc = Doc::from_anns(vec![
            ann(Shape::Rect(pos2(100.0, 100.0), pos2(300.0, 200.0))),
            ann(Shape::Arrow(pos2(400.0, 400.0), pos2(600.0, 300.0))),
            ann(Shape::Ellipse(pos2(700.0, 100.0), pos2(900.0, 250.0))),
        ]);
        let ed = Editor::new(&egui::Context::default(), base, doc, &shared);
        (ed, shared)
    }

    #[test]
    fn the_editor_bars_fit_at_the_minimum_size() {
        let (mut ed, mut shared) = editor();
        for size in [(900.0, 600.0), (1280.0, 820.0)] {
            let (_, painted, _) = run_editor(&mut ed, &mut shared, size, true, vec![vec![]]);
            let bars: Vec<_> = painted
                .iter()
                .filter(|(_, r)| r.center().y < BAR_HEIGHT + TOOL_BAR)
                .collect();
            assert!(bars.len() > 8, "{bars:?}");
            // The file name keeps room to read, even beside every action.
            let name = bars
                .iter()
                .find(|(s, r)| s.starts_with("Takeshot ") && r.center().y < BAR_HEIGHT)
                .map(|(_, r)| *r)
                .expect("the file name");
            assert!(name.width() >= 120.0, "{name:?} at {size:?}: {bars:?}");
            // The tools, colours and widths stay left of the zoom box, with a gap.
            let t = shared.tokens(None);
            let (_, _, blocks) = run_editor(&mut ed, &mut shared, size, true, vec![]);
            let in_tools = |r: &Rect| r.top() >= BAR_HEIGHT && r.bottom() <= BAR_HEIGHT + TOOL_BAR;
            let zoom_text = painted
                .iter()
                .find(|(s, r)| s.ends_with(" %") && in_tools(r))
                .map(|(_, r)| *r)
                .expect("the zoom");
            let zoom_box = blocks
                .iter()
                .filter(|(r, c)| *c == t.input && r.contains(zoom_text.center()))
                .map(|(r, _)| *r)
                .min_by(|a, b| a.area().total_cmp(&b.area()))
                .expect("the zoom box");
            for (r, _) in blocks.iter().filter(|(r, _)| {
                in_tools(r) && r.width() <= 34.0 && r.center().x < zoom_box.center().x
            }) {
                assert!(
                    r.right() <= zoom_box.left() - 4.0,
                    "{r:?} runs into the zoom box {zoom_box:?} at {size:?}"
                );
            }
            for (i, (a, ra)) in bars.iter().enumerate() {
                assert!(
                    ra.right() <= size.0 + 0.5,
                    "{a:?} {ra:?} outside at {size:?}"
                );
                for (b, rb) in &bars[i + 1..] {
                    assert!(
                        !ra.shrink(0.5).intersects(rb.shrink(0.5)),
                        "{a:?} {ra:?} overlaps {b:?} {rb:?} at {size:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_tool_click_sets_the_tool() {
        let (mut ed, mut shared) = editor();
        let (_, _, blocks) = run_editor(&mut ed, &mut shared, (1280.0, 820.0), true, vec![]);
        // The tools are 34 px squares in the tool bar, Select first: Rect is the fifth.
        let mut tools: Vec<_> = blocks
            .iter()
            .map(|(r, _)| *r)
            .filter(|r| r.size() == vec2(34.0, 34.0) && r.top() > BAR_HEIGHT)
            .collect();
        tools.sort_by(|a, b| a.left().total_cmp(&b.left()));
        tools.dedup_by(|a, b| (a.left() - b.left()).abs() < 1.0);
        assert_eq!(tools.len(), 10, "{tools:?}");
        let (acts, _, _) = run_editor(
            &mut ed,
            &mut shared,
            (1280.0, 820.0),
            true,
            click(tools[4].center()),
        );
        assert!(
            acts.contains(&EditorAction::SetTool(Tool::Rect)),
            "{acts:?}"
        );
    }

    #[test]
    fn letters_do_not_switch_tools_while_typing() {
        let (mut ed, mut shared) = editor();
        ed.an.tool = Tool::Text;
        ed.an.draft = Some(TextDraft {
            pos: pos2(50.0, 50.0),
            text: "Pico".into(),
            started: 0.0,
            caret: None,
        });
        let key = |pressed| Event::Key {
            key: Key::R,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let (acts, _, _) = run_editor(
            &mut ed,
            &mut shared,
            (1280.0, 820.0),
            true,
            vec![vec![key(true), Event::Text("r".into()), key(false)]],
        );
        assert!(
            !acts.iter().any(|a| matches!(a, EditorAction::SetTool(_))),
            "{acts:?}"
        );
        assert_eq!(ed.an.draft.as_ref().map(|d| d.text.as_str()), Some("Picor"));
    }

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// The editor with `text` being typed, after `frames` of input.
    fn type_into(text: &str, frames: Vec<Vec<Event>>) -> (Editor, Shared) {
        let (mut ed, mut shared) = editor();
        ed.an.tool = Tool::Text;
        ed.an.draft = Some(TextDraft {
            pos: pos2(500.0, 500.0),
            text: text.into(),
            started: 0.0,
            caret: None,
        });
        run_editor(&mut ed, &mut shared, SIZE, true, frames);
        (ed, shared)
    }

    fn draft(ed: &Editor) -> Option<&str> {
        ed.an.draft.as_ref().map(|d| d.text.as_str())
    }

    #[test]
    fn the_hint_has_the_text_size() {
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        shared.settings.draw.font_size = 60.0;
        ed.an.tool = Tool::Text;
        ed.an.draft = Some(TextDraft {
            pos: pos2(500.0, 500.0),
            text: String::new(),
            started: 0.0,
            caret: None,
        });
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![vec![]]);
        let view = ed.view().expect("the canvas view");
        let size = 60.0 * view.k;
        let hint = painted
            .iter()
            .find(|(s, _)| s == tr(Msg::TypeHere))
            .map(|(_, r)| *r)
            .expect("the hint");
        assert!(hint.height() >= size, "{hint:?} for a {size} px text");
        let at = view.to_screen(pos2(500.0, 500.0));
        assert!(
            (hint.left() - at.x).abs() < 2.0 && (hint.top() - at.y).abs() < 2.0,
            "{hint:?} at {at:?}"
        );
    }

    #[test]
    fn an_edited_text_opens_with_the_caret_where_it_was_clicked() {
        let (mut ed, mut shared) = editor();
        ed.an.tool = Tool::Text;
        ed.an.draft = Some(TextDraft {
            pos: pos2(500.0, 500.0),
            text: "Pico".into(),
            started: 1.0,
            caret: Some(2),
        });
        run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![vec![], vec![Event::Text("X".into())]],
        );
        assert_eq!(draft(&ed), Some("PiXco"));
    }

    #[test]
    fn typing_edits_at_the_caret() {
        let left = || key(Key::ArrowLeft, Modifiers::NONE);
        let (ed, _) = type_into(
            "Pico",
            vec![
                vec![left(), left()],
                vec![Event::Text("X".into())],
                vec![key(Key::Home, Modifiers::NONE)],
                vec![Event::Text("A".into())],
            ],
        );
        assert_eq!(draft(&ed), Some("APiXco"));
    }

    #[test]
    fn a_selection_is_typed_over() {
        let shift_left = || key(Key::ArrowLeft, Modifiers::SHIFT);
        let (ed, _) = type_into(
            "Pico em outubro",
            vec![
                (0..7).map(|_| shift_left()).collect(),
                vec![Event::Text("maio".into())],
            ],
        );
        assert_eq!(draft(&ed), Some("Pico em maio"));
    }

    #[test]
    fn enter_keeps_the_text_and_shift_enter_breaks_the_line() {
        let (ed, _) = type_into(
            "Pico",
            vec![
                vec![key(Key::Enter, Modifiers::SHIFT)],
                vec![Event::Text("x".into())],
                vec![key(Key::Enter, Modifiers::NONE)],
            ],
        );
        assert_eq!(draft(&ed), None);
        let last = ed.an.doc.objs().last().map(|o| o.ann.shape.clone());
        assert!(
            matches!(&last, Some(Shape::Text { text, .. }) if text == "Pico
x"),
            "{last:?}"
        );
    }

    #[test]
    fn esc_ends_the_text_and_keeps_it() {
        let (ed, _) = type_into("Pico", vec![vec![key(Key::Escape, Modifiers::NONE)]]);
        assert_eq!(draft(&ed), None);
        assert_eq!(ed.an.doc.objs().len(), 4, "kept as an object");
        let (ed, _) = type_into("", vec![vec![key(Key::Escape, Modifiers::NONE)]]);
        assert_eq!(draft(&ed), None);
        assert_eq!(ed.an.doc.objs().len(), 3, "an empty text goes");
    }

    #[test]
    fn space_drag_pans_the_zoomed_picture() {
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        ed.apply(
            EditorAction::Zoom(Zoom::Percent(200)),
            &ctx,
            None,
            &mut shared,
        );
        let space = |pressed| Event::Key {
            key: Key::Space,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let at = pos2(480.0, 450.0);
        let button = |p, pressed| Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let to = at + vec2(-120.0, -60.0);
        let before = ed.an.doc.objs().len();
        run_editor(
            &mut ed,
            &mut shared,
            (1280.0, 820.0),
            true,
            vec![
                vec![Event::PointerMoved(at)],
                vec![space(true)],
                vec![button(at, true)],
                vec![Event::PointerMoved(at + vec2(-60.0, -30.0))],
                vec![Event::PointerMoved(to)],
                vec![button(to, false)],
                vec![space(false)],
            ],
        );
        assert!(ed.pan().length() > 50.0, "pan {:?}", ed.pan());
        assert_eq!(
            ed.an.doc.objs().len(),
            before,
            "nothing drawn while panning"
        );
    }

    #[test]
    fn ctrl_wheel_zooms_at_the_pointer() {
        // Three notches, each one frame of zoom and a frame without.
        let (mut ed, mut shared) = editor();
        // The styled picture is the largest mesh (it may run past the canvas once zoomed).
        let picture = |blocks: &[crate::skins::takeui::testing::Block]| {
            blocks
                .iter()
                .filter(|(_, c)| *c == crate::skins::takeui::testing::MESH)
                .map(|(r, _)| *r)
                .max_by(|a, b| a.area().total_cmp(&b.area()))
                .expect("the picture")
        };
        let (_, _, blocks) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let before = picture(&blocks);
        let p = pos2(580.0, 505.0);
        let zoom = || vec![Event::Zoom(2.0)];
        let (_, _, blocks) = run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![
                vec![Event::PointerMoved(p)],
                zoom(),
                vec![],
                zoom(),
                vec![],
                zoom(),
                vec![],
            ],
        );
        let after = picture(&blocks);
        assert!(
            after.width() > before.width() * 1.5,
            "{before:?} → {after:?}"
        );
        // The point of the picture under the pointer stays under it.
        let at = |r: Rect| (p - r.min) / r.size();
        assert!(
            (at(before) - at(after)).length() < 0.01,
            "{:?} → {:?}",
            at(before),
            at(after)
        );
    }

    #[test]
    fn space_pans_with_the_hand() {
        let (mut ed, mut shared) = editor();
        let space = |pressed| Event::Key {
            key: Key::Space,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let at = pos2(480.0, 450.0);
        let (_, p) = crate::skins::takeui::testing::drive_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![vec![Event::PointerMoved(at)], vec![space(true)], vec![]],
        );
        assert_eq!(p.cursor, egui::CursorIcon::Grab, "Space over the picture");
        let (_, p) = crate::skins::takeui::testing::drive_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![
                vec![Event::PointerMoved(at)],
                vec![space(true)],
                vec![press(at, true)],
                vec![Event::PointerMoved(at + vec2(-40.0, -20.0))],
                vec![Event::PointerMoved(at + vec2(-80.0, -40.0))],
            ],
        );
        assert_eq!(p.cursor, egui::CursorIcon::Grabbing, "while it drags");
    }

    /// The centre of `icon` painted over the canvas (the panel has its own Duplicate and Delete).
    fn on_canvas(painted: &[(String, Rect)], icon: Ico) -> Option<Pos2> {
        let g = crate::ui::icon::glyph(icon).to_string();
        painted
            .iter()
            .find(|(s, r)| *s == g && r.right() < SIZE.0 - PANEL && r.top() > BAR_HEIGHT + TOOL_BAR)
            .map(|(_, r)| r.center())
    }

    #[test]
    fn the_object_bar_acts_on_the_selection() {
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(EditorAction::Select(rect, false), &ctx, None, &mut shared);
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let dup = on_canvas(&painted, Ico::CopyPlus).expect("Duplicate on the object bar");
        run_editor(&mut ed, &mut shared, SIZE, true, click(dup));
        assert_eq!(ed.an.doc.objs().len(), 4, "a copy");
        assert_eq!(ed.an.doc.selected().len(), 1);
        assert_ne!(ed.an.doc.selected()[0], rect, "the copy is selected");

        ed.apply(EditorAction::Select(rect, false), &ctx, None, &mut shared);
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let front = on_canvas(&painted, Ico::BringFront).expect("Bring to front");
        run_editor(&mut ed, &mut shared, SIZE, true, click(front));
        assert_eq!(ed.an.doc.objs().last().map(|o| o.id), Some(rect));
        assert_eq!(ed.an.doc.selected(), &[rect], "the selection stays");

        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let back = on_canvas(&painted, Ico::SendBack).expect("Send to back");
        run_editor(&mut ed, &mut shared, SIZE, true, click(back));
        assert_eq!(ed.an.doc.objs().first().map(|o| o.id), Some(rect));

        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let del = on_canvas(&painted, Ico::Trash).expect("Delete");
        run_editor(&mut ed, &mut shared, SIZE, true, click(del));
        assert_eq!(ed.an.doc.objs().len(), 3);
        assert!(ed.an.doc.objs().iter().all(|o| o.id != rect));
    }

    #[test]
    fn no_object_bar_for_a_selection_out_of_view() {
        // At 400 % the view shows the picture's middle; the rectangle (100,100)-(300,200) is off it.
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(EditorAction::Select(rect, false), &ctx, None, &mut shared);
        ed.apply(
            EditorAction::Zoom(Zoom::Percent(400)),
            &ctx,
            None,
            &mut shared,
        );
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        assert!(on_canvas(&painted, Ico::CopyPlus).is_none());
    }

    /// Two objects of their own colour and opacity, both selected.
    fn two_selected() -> (Editor, Shared, [crate::engine::doc::ObjId; 2]) {
        let shared = Shared::for_tests();
        let base = image::RgbaImage::from_pixel(1200, 740, image::Rgba([240, 240, 240, 255]));
        let ann = |shape, color| Ann {
            shape,
            color,
            width: 3.0,
            filled: false,
        };
        let doc = Doc::from_anns(vec![
            ann(
                Shape::Rect(pos2(100.0, 100.0), pos2(300.0, 200.0)),
                [255, 0, 0, 255],
            ),
            ann(
                Shape::Rect(pos2(400.0, 100.0), pos2(600.0, 200.0)),
                [0, 0, 255, 128],
            ),
        ]);
        let mut ed = Editor::new(&egui::Context::default(), base, doc, &shared);
        let mut shared = shared;
        let ids = [ed.an.doc.objs()[0].id, ed.an.doc.objs()[1].id];
        let ctx = egui::Context::default();
        ed.apply(EditorAction::Select(ids[0], false), &ctx, None, &mut shared);
        ed.apply(EditorAction::Select(ids[1], true), &ctx, None, &mut shared);
        (ed, shared, ids)
    }

    #[test]
    fn opacity_and_colour_keep_what_each_object_does_not_change() {
        let ctx = egui::Context::default();
        let colours = |ed: &Editor| -> Vec<[u8; 4]> {
            ed.an.doc.objs().iter().map(|o| o.ann.color).collect()
        };
        let (mut ed, mut shared, _) = two_selected();
        ed.apply(EditorAction::SetOpacity(191), &ctx, None, &mut shared);
        assert_eq!(colours(&ed), vec![[255, 0, 0, 191], [0, 0, 255, 191]]);
        let (mut ed, mut shared, _) = two_selected();
        ed.apply(
            EditorAction::SetColor([0, 200, 0, 255]),
            &ctx,
            None,
            &mut shared,
        );
        assert_eq!(colours(&ed), vec![[0, 200, 0, 255], [0, 200, 0, 128]]);
    }

    #[test]
    fn a_gesture_ends_when_the_pointer_is_up() {
        // A slider that stops being drawn mid-drag never reports its release: the next frame
        // with no button down closes the gesture, so later changes are steps of their own.
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(EditorAction::Select(rect, false), &ctx, None, &mut shared);
        ed.apply(EditorAction::Gesture(true), &ctx, None, &mut shared);
        ed.apply(EditorAction::SetOpacity(100), &ctx, None, &mut shared);
        run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        ed.apply(EditorAction::SetOpacity(50), &ctx, None, &mut shared);
        assert!(ed.an.doc.undo());
        assert_eq!(ed.an.doc.objs()[0].ann.color[3], 100);
    }

    #[test]
    fn a_slider_release_ends_its_step() {
        // After a drag on the opacity slider, a colour change is its own undo step.
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(EditorAction::Select(rect, false), &ctx, None, &mut shared);
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let label = panel_text(&painted, tr(Msg::Opacity));
        let at = |x: f32| pos2(label.left() + 96.0 + x, label.center().y);
        let (_, p) = crate::skins::takeui::testing::drive_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![
                vec![Event::PointerMoved(at(60.0))],
                vec![press(at(60.0), true)],
                vec![Event::PointerMoved(at(40.0))],
                vec![Event::PointerMoved(at(20.0))],
                vec![press(at(20.0), false)],
            ],
        );
        let _ = p;
        let alpha = ed.an.doc.objs()[0].ann.color[3];
        assert!(alpha < 255, "the drag changed the opacity");
        ed.apply(
            EditorAction::SetColor([0, 200, 0, 255]),
            &ctx,
            None,
            &mut shared,
        );
        assert!(ed.an.doc.undo());
        assert_eq!(ed.an.doc.objs()[0].ann.color, [255, 0, 0, alpha]);
    }

    #[test]
    fn undo_and_redo_show_the_style_they_restore() {
        let (mut ed, mut shared) = editor();
        let ctx = egui::Context::default();
        let arrow = ed.an.doc.objs()[1].id;
        ed.apply(EditorAction::Select(arrow, false), &ctx, None, &mut shared);
        ed.apply(EditorAction::SetOpacity(128), &ctx, None, &mut shared);
        ed.apply(EditorAction::Undo, &ctx, None, &mut shared);
        assert_eq!(ed.an.doc.objs()[1].ann.color[3], 255);
        assert_eq!(
            shared.settings.draw.color[3], 255,
            "the panel shows the restored opacity"
        );
        ed.apply(EditorAction::Redo, &ctx, None, &mut shared);
        assert_eq!(shared.settings.draw.color[3], 128);
        // Ctrl+Z on the canvas too.
        let ctrl_z = Event::Key {
            key: Key::Z,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        run_editor(&mut ed, &mut shared, SIZE, true, vec![vec![ctrl_z]]);
        assert_eq!(ed.an.doc.objs()[1].ann.color[3], 255);
        assert_eq!(shared.settings.draw.color[3], 255);
    }

    #[test]
    fn duplicate_and_delete_share_a_row() {
        let _lang = crate::ui::i18n::lang_lock();
        let before = crate::ui::i18n::lang();
        for l in [crate::ui::i18n::Lang::Pt, crate::ui::i18n::Lang::En] {
            crate::ui::i18n::set_lang(l);
            let (mut ed, mut shared) = editor();
            let rect = ed.an.doc.objs()[0].id;
            ed.apply(
                EditorAction::Select(rect, false),
                &egui::Context::default(),
                None,
                &mut shared,
            );
            let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
            let dup = panel_text(&painted, tr(Msg::Duplicate));
            let del = panel_text(&painted, tr(Msg::Delete));
            crate::ui::i18n::set_lang(before);
            assert!(
                (dup.center().y - del.center().y).abs() < 1.0,
                "{l:?}: {dup:?} and {del:?}"
            );
        }
        crate::ui::i18n::set_lang(before);
    }

    #[test]
    fn the_object_bar_colour_offers_the_picker() {
        // The bar's colour opens the palette with the colour picker in it (a "+" there opened
        // nothing: egui keeps one popup open at a time).
        let (mut ed, mut shared) = editor();
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(
            EditorAction::Select(rect, false),
            &egui::Context::default(),
            None,
            &mut shared,
        );
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let dup = on_canvas(&painted, Ico::CopyPlus).expect("the object bar");
        let colour = dup - vec2(30.0, 0.0);
        let (_, painted, blocks) = run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            [click(colour), vec![vec![]]].concat(),
        );
        // egui's picker paints its colour square as a light, textured square of 100 px or so.
        let picker = blocks.iter().any(|(r, c)| {
            r.right() < SIZE.0 - PANEL
                && (80.0..300.0).contains(&r.width())
                && (r.width() - r.height()).abs() < 1.0
                && c.r() > 200
                && c.g() > 200
        });
        assert!(picker, "a colour picker in the popup");
        assert!(
            on_canvas(&painted, Ico::Plus).is_none(),
            "no \"+\" that opens nothing"
        );
    }

    #[test]
    fn the_wheel_changes_the_width_one_step_a_notch() {
        let (mut ed, mut shared) = editor();
        ed.an.tool = Tool::Rect;
        shared.settings.draw.width = 3.0;
        let at = pos2(480.0, 450.0);
        let wheel = Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0.0, 1.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        };
        run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![
                vec![Event::PointerMoved(at)],
                vec![wheel],
                vec![],
                vec![],
                vec![],
            ],
        );
        assert_eq!(shared.settings.draw.width, 4.0);
    }

    #[test]
    fn the_object_bar_hides_while_dragging() {
        let (mut ed, mut shared) = editor();
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(
            EditorAction::Select(rect, false),
            &egui::Context::default(),
            None,
            &mut shared,
        );
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        assert!(on_canvas(&painted, Ico::CopyPlus).is_some());
        // Grab the rectangle's top edge (picture y 100, between x 100 and 300) and move it.
        let view = ed.view().expect("the canvas view");
        let at = view.to_screen(pos2(200.0, 100.0));
        let (_, painted, _) = run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![
                vec![Event::PointerMoved(at)],
                vec![press(at, true)],
                vec![Event::PointerMoved(at + vec2(30.0, 20.0))],
                vec![Event::PointerMoved(at + vec2(60.0, 40.0))],
            ],
        );
        assert!(ed.an.dragging.is_some(), "the object is being dragged");
        assert!(
            on_canvas(&painted, Ico::CopyPlus).is_none(),
            "hidden mid-drag"
        );
    }

    #[test]
    fn the_status_bar_counts_objects_and_shows_the_size() {
        // Built and painted in the same language: hold it.
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        let (_, painted, _) = run_editor(&mut ed, &mut shared, (1280.0, 820.0), true, vec![]);
        let three = [tf(Msg::ObjectsMany, &["3"])];
        let names: Vec<&str> = three.iter().map(String::as_str).collect();
        assert!(painted_at(&painted, &names, |r| r.top() > 760.0).is_some());
        let size = painted
            .iter()
            .find(|(s, r)| s == "1200 × 740" && r.top() > 760.0)
            .map(|(_, r)| *r)
            .expect("the size");
        assert!(
            size.right() <= 1280.0 - 16.0 + 0.5,
            "16 px from the edge: {size:?}"
        );
        let _ = every(Msg::ObjectsOne);
    }

    const SIZE: (f32, f32) = (1280.0, 820.0);

    fn in_panel(r: Rect) -> bool {
        r.left() >= SIZE.0 - PANEL
    }

    /// The lowest painted text equal to `text` in the panel (the list sits below the header).
    fn panel_text(painted: &[(String, Rect)], text: &str) -> Rect {
        painted
            .iter()
            .filter(|(s, r)| s == text && in_panel(*r))
            .map(|(_, r)| *r)
            .max_by(|a, b| a.top().total_cmp(&b.top()))
            .unwrap_or_else(|| panic!("{text:?} in the panel: {painted:?}"))
    }

    fn press(p: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn a_row_click_selects_the_object() {
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let row = panel_text(&painted, tr(Msg::ToolRect));
        run_editor(&mut ed, &mut shared, SIZE, true, click(row.center()));
        let rect = ed.an.doc.objs()[0].id;
        assert_eq!(ed.an.doc.selected(), &[rect]);
        assert_eq!(ed.an.tool, Tool::Select);
    }

    #[test]
    fn the_eye_hides_and_shows() {
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        let eye = |painted: &[(String, Rect)], icon: Ico| {
            let row = panel_text(painted, tr(Msg::ToolRect));
            let g = crate::ui::icon::glyph(icon).to_string();
            painted
                .iter()
                .find(|(s, r)| {
                    *s == g && in_panel(*r) && (r.center().y - row.center().y).abs() < 4.0
                })
                .map(|(_, r)| r.center())
                .unwrap_or_else(|| panic!("{icon:?} beside the row"))
        };
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            click(eye(&painted, Ico::Eye)),
        );
        assert!(ed.an.doc.objs()[0].hidden);
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            click(eye(&painted, Ico::EyeOff)),
        );
        assert!(!ed.an.doc.objs()[0].hidden);
    }

    #[test]
    fn opacity_sets_the_alpha_of_the_selection() {
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        // The tool's colour differs from the object's: the change must keep the object's red.
        shared.settings.draw.color = [0, 0, 255, 255];
        let rect = ed.an.doc.objs()[0].id;
        ed.apply(
            EditorAction::Select(rect, false),
            &egui::Context::default(),
            None,
            &mut shared,
        );
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let label = panel_text(&painted, tr(Msg::Opacity));
        let value = painted
            .iter()
            .find(|(s, r)| {
                s == "100 %" && in_panel(*r) && (r.center().y - label.center().y).abs() < 4.0
            })
            .map(|(_, r)| *r)
            .expect("the value box");
        // kit::slider_row: the track starts 96 px after the label and ends 12 px before the
        // 58 px value box, whose text ends 8 px inside it.
        let (lo, hi) = (label.left() + 96.0, value.right() + 8.0 - 58.0 - 12.0);
        let at = |f: f32| pos2(lo + f * (hi - lo), label.center().y);
        run_editor(
            &mut ed,
            &mut shared,
            SIZE,
            true,
            vec![
                vec![Event::PointerMoved(at(0.5))],
                vec![press(at(0.5), true)],
                vec![Event::PointerMoved(at(0.6))],
                vec![Event::PointerMoved(at(0.75))],
                vec![press(at(0.75), false)],
            ],
        );
        assert_eq!(ed.an.doc.objs()[0].ann.color, [255, 0, 0, 191]);
        assert!(ed.an.doc.undo());
        assert_eq!(ed.an.doc.objs()[0].ann.color, [255, 0, 0, 255]);
        assert!(!ed.an.doc.undo(), "the whole drag is one undo step");
    }

    #[test]
    fn the_style_tab_changes_the_saved_style() {
        let _lang = crate::ui::i18n::lang_lock();
        let (mut ed, mut shared) = editor();
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let tab = panel_text(&painted, tr(Msg::Style));
        run_editor(&mut ed, &mut shared, SIZE, true, click(tab.center()));
        assert_eq!(ed.tab(), PanelTab::Style);
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![]);
        let none = panel_text(&painted, tr(Msg::Transparent));
        let (acts, _, _) = run_editor(&mut ed, &mut shared, SIZE, true, click(none.center()));
        assert_eq!(
            shared.settings.style.background,
            crate::system::settings::Background::None
        );
        assert!(acts.contains(&EditorAction::StyleChanged), "{acts:?}");
    }

    #[test]
    fn the_object_page_fits_the_panel_in_both_languages() {
        // For each kind of object picked, nothing runs past the content's 20 px right padding
        // (it holds the scroll bar) and no text runs into another, in either language.
        let _lang = crate::ui::i18n::lang_lock();
        let before = crate::ui::i18n::lang();
        let ann = |shape| Ann {
            shape,
            color: [255, 0, 0, 255],
            width: 3.0,
            filled: false,
        };
        let anns = vec![
            ann(Shape::Rect(pos2(100.0, 100.0), pos2(300.0, 200.0))),
            ann(Shape::Text {
                pos: pos2(400.0, 100.0),
                text: "Peak in October, the best month of the year by far".into(),
                font: 0,
                size: 24.0,
                boxed: true,
            }),
            ann(Shape::Blur(
                pos2(500.0, 300.0),
                pos2(700.0, 400.0),
                crate::engine::model::BlurMode::Blur,
            )),
            ann(Shape::Counter {
                center: pos2(800.0, 500.0),
                n: 1,
            }),
        ];
        for l in [crate::ui::i18n::Lang::Pt, crate::ui::i18n::Lang::En] {
            crate::ui::i18n::set_lang(l);
            for size in [(900.0, 600.0), SIZE] {
                for pick in 0..anns.len() {
                    let mut shared = Shared::for_tests();
                    let base =
                        image::RgbaImage::from_pixel(1200, 740, image::Rgba([240, 240, 240, 255]));
                    let mut ed = Editor::new(
                        &egui::Context::default(),
                        base,
                        Doc::from_anns(anns.clone()),
                        &shared,
                    );
                    let id = ed.an.doc.objs()[pick].id;
                    ed.apply(
                        EditorAction::Select(id, false),
                        &egui::Context::default(),
                        None,
                        &mut shared,
                    );
                    let (_, painted, _) = run_editor(&mut ed, &mut shared, size, true, vec![]);
                    let left = size.0 - PANEL;
                    let texts: Vec<_> = painted
                        .iter()
                        .filter(|(_, r)| {
                            r.left() >= left
                                && r.top() >= BAR_HEIGHT + TOOL_BAR
                                && r.bottom() <= size.1 - STATUS_BAR
                        })
                        .collect();
                    assert!(texts.len() > 8, "{l:?} {size:?} {pick}: {texts:?}");
                    for (i, (text, r)) in texts.iter().enumerate() {
                        assert!(
                            r.right() <= size.0 - 20.0 + 0.5,
                            "{l:?} {size:?} {pick}: {text:?} {r:?} past the padding"
                        );
                        for (other, o) in &texts[i + 1..] {
                            assert!(
                                !r.shrink(0.5).intersects(o.shrink(0.5)),
                                "{l:?} {size:?} {pick}: {text:?} {r:?} runs into {other:?} {o:?}"
                            );
                        }
                    }
                }
            }
        }
        crate::ui::i18n::set_lang(before);
    }

    #[test]
    fn a_long_list_scrolls_and_keeps_the_selection_in_view() {
        let _lang = crate::ui::i18n::lang_lock();
        let mut shared = Shared::for_tests();
        let base = image::RgbaImage::from_pixel(1200, 740, image::Rgba([240, 240, 240, 255]));
        let anns = (1..=100u32)
            .map(|n| Ann {
                shape: Shape::Counter {
                    center: pos2(30.0 + (n % 20) as f32 * 55.0, 40.0 + (n / 20) as f32 * 70.0),
                    n,
                },
                color: [255, 0, 0, 255],
                width: 3.0,
                filled: false,
            })
            .collect();
        let mut ed = Editor::new(
            &egui::Context::default(),
            base,
            Doc::from_anns(anns),
            &shared,
        );
        // Step 1 is the back-most object: the list's last row.
        let first = ed.an.doc.objs()[0].id;
        ed.apply(
            EditorAction::Select(first, false),
            &egui::Context::default(),
            None,
            &mut shared,
        );
        let (_, painted, _) = run_editor(&mut ed, &mut shared, SIZE, true, vec![vec![], vec![]]);
        let step = |n: u32| format!("{} {n}", tr(Msg::ToolStep));
        let row = panel_text(&painted, &step(1));
        let view = (BAR_HEIGHT + TOOL_BAR, SIZE.1 - STATUS_BAR);
        assert!(
            row.top() >= view.0 && row.bottom() <= view.1,
            "the selected row is in view: {row:?}"
        );
        let front = panel_text(&painted, &step(100));
        assert!(front.bottom() < view.0, "the list scrolled: {front:?}");
    }
}
