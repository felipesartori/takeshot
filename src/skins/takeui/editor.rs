//! TakeUI's editor: the title bar (file and size, undo and redo, pin, save as, save, copy, window
//! buttons), the tool bar (tools, colours, widths, zoom), the canvas, the right panel and the
//! status bar, as in "Editor · escuro" of the design. Behaviour is in `screens::editor`.

use crate::Shared;
use crate::engine::model::{PALETTE, Tool, WIDTHS};
use crate::screens::editor::{Editor, EditorAction, Zoom, tool_hint, tool_name};
use crate::screens::home::MainAction;
use crate::ui::chrome::{self, BAR_HEIGHT};
use crate::ui::i18n::{Msg, t as tr, tf};
use crate::ui::kit::{self, BtnKind, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::Tokens;
use egui::{Align, Color32, Frame, Layout, Rect, Stroke, Ui, UiBuilder, pos2, vec2};

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
        .show(ui, |ui| panel(ui, t, shared, &mut acts));
    egui::CentralPanel::no_frame().show(ui, |ui| acts.extend(ed.canvas(ui, t, shared)));
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

fn tool_icon(tool: Tool) -> Option<Ico> {
    Some(match tool {
        Tool::Select => Ico::Select,
        Tool::Pen => Ico::Pen,
        Tool::Line => Ico::Line,
        Tool::Arrow => Ico::Arrow,
        Tool::Rect => Ico::Rect,
        Tool::Ellipse => Ico::Ellipse,
        Tool::Marker => Ico::Marker,
        Tool::Text => Ico::Text,
        Tool::Blur => Ico::Blur,
        Tool::Counter => return None,
    })
}

fn divider(ui: &mut Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(1.0, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, t.border);
}

fn tool_bar(ui: &mut Ui, t: &Tokens, ed: &Editor, shared: &Shared, acts: &mut Vec<EditorAction>) {
    let r = ui.max_rect();
    ui.painter()
        .hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, t.border));
    let opts = &shared.settings.draw;
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
        for c in PALETTE {
            let on = opts.color[..3] == c[..3];
            let col = Color32::from_rgb(c[0], c[1], c[2]);
            let tip = format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
            if kit::color_dot(ui, t, col, on, &tip).clicked() {
                acts.push(EditorAction::SetColor(c));
            }
        }
        let add = kit::color_add(ui, t, tr(Msg::CustomColor));
        egui::Popup::from_toggle_button_response(&add)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                let c = opts.color;
                let mut col = Color32::from_rgb(c[0], c[1], c[2]);
                if egui::color_picker::color_picker_color32(
                    ui,
                    &mut col,
                    egui::color_picker::Alpha::Opaque,
                ) {
                    acts.push(EditorAction::SetColor(col.to_array()));
                }
            });
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
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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

/// The right panel; Task 3 adds the Object tab. For now the style controls.
fn panel(ui: &mut Ui, t: &Tokens, shared: &mut Shared, acts: &mut Vec<EditorAction>) {
    let r = ui.max_rect();
    ui.painter()
        .vline(r.left() + 0.5, r.y_range(), Stroke::new(1.0, t.border));
    egui::ScrollArea::vertical()
        .id_salt("takeui-editor-panel")
        .auto_shrink([false, false])
        .scroll_bar_rect(r.shrink2(vec2(0.0, 8.0)))
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
                    let mut main = Vec::new();
                    if super::style::controls(ui, t, shared, &mut main) {
                        acts.push(EditorAction::StyleChanged);
                    }
                    for a in main {
                        if a == MainAction::ChooseLogo {
                            acts.push(EditorAction::ChooseLogo);
                        }
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
    use crate::skins::takeui::testing::{click, painted_at, run_editor};
    use crate::ui::i18n::every;
    use egui::{Event, Key, Modifiers};

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
}
