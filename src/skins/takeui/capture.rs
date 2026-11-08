//! TakeUI's capture: the main bar under the area (tools, undo, redo, pin, save, copy, Edit,
//! close), the options bar under it for a drawing tool or a selected object, and the hint before
//! an area is chosen, as in "Captura (PrtSc) · escuro". Behaviour is in `screens::overlay`.

use super::{divider, tool_icon};
use crate::Shared;
use crate::engine::model::{BlurMode, Tool, WIDTHS};
use crate::engine::placement::capture_bars;
use crate::screens::editor::{object_label, tool_name};
use crate::screens::overlay::{CaptureAction, Overlay};
use crate::ui::i18n::{Msg, t as tr};
use crate::ui::kit::{self, BtnKind, Ico};
use crate::ui::style::{Weight, font};
use crate::ui::tokens::Tokens;
use egui::{Color32, Frame, Rect, Ui, vec2};

/// The bars' look: the surface with a 1 px border, 12 px corners and a soft shadow.
fn bar_frame(t: &Tokens) -> Frame {
    Frame::new()
        .fill(t.surf)
        .stroke(egui::Stroke::new(1.0, t.border))
        .corner_radius(12)
        .shadow(egui::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(102),
        })
}

/// The bars once an area is chosen, the hint before. Returns what the user asked for.
pub fn capture(
    ui: &mut Ui,
    t: &Tokens,
    ov: &Overlay,
    shared: &Shared,
    screen: Rect,
) -> Vec<CaptureAction> {
    let mut acts = Vec::new();
    match ov.area_on(screen) {
        None => hint(ui.ctx(), t),
        Some(area) if ov.bars_shown() => bars(ui.ctx(), t, ov, shared, area, screen, &mut acts),
        Some(_) => {}
    }
    acts
}

/// The keys before an area is chosen, centred 22 px under the top of the screen.
fn hint(ctx: &egui::Context, t: &Tokens) {
    egui::Area::new(egui::Id::unique("takeui-capture-hint"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .anchor(egui::Align2::CENTER_TOP, vec2(0.0, 22.0))
        .show(ctx, |ui| {
            bar_frame(t)
                .inner_margin(egui::Margin::symmetric(14, 9))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for (key, what) in [
                            (tr(Msg::CapDrag), Msg::CapSelect),
                            ("Shift", Msg::CapSquare),
                            ("Ctrl+A", Msg::CapWholeScreen),
                            ("C", Msg::CapCopyColor),
                            ("Esc", Msg::CapLeave),
                        ] {
                            ui.label(
                                egui::RichText::new(key)
                                    .font(font(12.5, Weight::Medium))
                                    .color(t.text),
                            );
                            ui.label(
                                egui::RichText::new(tr(what))
                                    .font(font(12.5, Weight::Regular))
                                    .color(t.muted),
                            );
                            ui.add_space(6.0);
                        }
                    });
                });
        });
}

/// The main bar and, with a drawing tool or a selected object, the options bar, placed by
/// `capture_bars` from their sizes in the last frame.
fn bars(
    ctx: &egui::Context,
    t: &Tokens,
    ov: &Overlay,
    shared: &Shared,
    area: Rect,
    screen: Rect,
    acts: &mut Vec<CaptureAction>,
) {
    let main_id = egui::Id::unique("takeui-capture-main");
    let opts_id = egui::Id::unique("takeui-capture-options");
    let size = |id: egui::Id, default: egui::Vec2| {
        ctx.data(|d| d.get_temp::<egui::Vec2>(id))
            .unwrap_or(default)
    };
    let bar_tool = ov.an.bar_tool();
    let (main_at, opts_at) = capture_bars(
        area,
        size(main_id, vec2(674.0, 42.0)),
        bar_tool.map(|_| size(opts_id, vec2(552.0, 40.0))),
        screen,
    );
    let shown = egui::Area::new(main_id)
        .order(egui::Order::Foreground)
        .fixed_pos(main_at.min)
        .show(ctx, |ui| {
            bar_frame(t)
                .inner_margin(4)
                .show(ui, |ui| main_bar(ui, t, ov, acts))
        });
    ctx.data_mut(|d| d.insert_temp(main_id, shown.response.rect.size()));
    if let (Some(tool), Some(at)) = (bar_tool, opts_at) {
        let shown = egui::Area::new(opts_id)
            .order(egui::Order::Foreground)
            .fixed_pos(at.min)
            .show(ctx, |ui| {
                bar_frame(t)
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| options_bar(ui, t, ov, shared, tool, acts))
            });
        ctx.data_mut(|d| d.insert_temp(opts_id, shown.response.rect.size()));
    }
}

/// Tools, undo and redo, pin, save, copy, Edit (primary) and close; 34 px buttons 2 apart.
fn main_bar(ui: &mut Ui, t: &Tokens, ov: &Overlay, acts: &mut Vec<CaptureAction>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for tool in [Tool::Select].into_iter().chain(Tool::DRAWING) {
            let tip = format!("{} ({})", tr(tool_name(tool)), tool.key_name());
            if kit::tool_button(ui, t, tool_icon(tool), ov.an.tool == tool, &tip, false).clicked() {
                acts.push(CaptureAction::SetTool(tool));
            }
        }
        ui.add_space(2.0);
        divider(ui, t, 22.0);
        ui.add_space(4.0);
        let keys = |name: Msg, k: &str| format!("{} ({k})", tr(name));
        for (icon, tip, act) in [
            (Ico::Undo, keys(Msg::Undo, "Ctrl+Z"), CaptureAction::Undo),
            (Ico::Redo, keys(Msg::Redo, "Ctrl+Y"), CaptureAction::Redo),
            (
                Ico::Pin,
                keys(Msg::PinOnScreen, "Ctrl+T"),
                CaptureAction::Pin,
            ),
            (
                Ico::Download,
                keys(Msg::Save, "Ctrl+S"),
                CaptureAction::Save,
            ),
            (Ico::Copy, keys(Msg::Copy, "Ctrl+C"), CaptureAction::Copy),
        ] {
            if kit::tool_button(ui, t, Some(icon), false, &tip, false).clicked() {
                acts.push(act);
            }
        }
        ui.add_space(2.0);
        if kit::button(
            ui,
            t,
            BtnKind::Primary,
            Some(Ico::Edit),
            tr(Msg::Edit),
            None,
        )
        .on_hover_text(format!("{} (Enter)", tr(Msg::EditTip)))
        .clicked()
        {
            acts.push(CaptureAction::Edit);
        }
        let close = format!("{} (Esc)", tr(Msg::CloseWord));
        if kit::tool_button(ui, t, Some(Ico::Close), false, &close, false).clicked() {
            acts.push(CaptureAction::Close);
        }
    });
}

/// The tool's or the selected object's name, then its options: colours, widths, Fill for
/// rectangles and ellipses, font and background for text, Blur or Pixelate for blur; with an
/// object selected, Duplicate and Delete.
fn options_bar(
    ui: &mut Ui,
    t: &Tokens,
    ov: &Overlay,
    shared: &Shared,
    tool: Tool,
    acts: &mut Vec<CaptureAction>,
) {
    let opts = &shared.settings.draw;
    let doc = &ov.an.doc;
    let name = match doc.selected() {
        [one] => doc
            .kind(*one)
            .map(|(k, text)| object_label(k, text.as_deref()))
            .unwrap_or_default(),
        _ => tr(tool_name(tool)).to_string(),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let mut job =
            egui::text::LayoutJob::simple_singleline(name, font(12.0, Weight::Medium), t.muted);
        job.wrap = egui::text::TextWrapping::truncate_at_width(160.0);
        ui.label(job);
        ui.add_space(4.0);
        if tool != Tool::Blur
            && let Some(c) = super::palette(ui, t, opts.color)
        {
            acts.push(CaptureAction::SetColor(c));
        }
        match tool {
            Tool::Text => {
                divider(ui, t, 20.0);
                let names: Vec<(usize, &str)> = shared
                    .fonts
                    .list
                    .iter()
                    .enumerate()
                    .map(|(i, f)| (i, f.name.as_str()))
                    .collect();
                let mut f = opts.font;
                if kit::select(ui, t, "takeui-capture-font", &mut f, &names, 150.0).changed() {
                    acts.push(CaptureAction::SetFont(f));
                }
                let size = opts.font_size.round();
                kit::combo(
                    ui,
                    t,
                    "takeui-capture-size",
                    &format!("{size} px"),
                    52.0,
                    28.0,
                    |ui| {
                        for s in [12.0, 16.0, 20.0, 24.0, 32.0, 48.0, 64.0, 96.0] {
                            if ui.selectable_label(size == s, format!("{s} px")).clicked() {
                                acts.push(CaptureAction::SetFontSize(s));
                            }
                        }
                    },
                );
                let mut boxed = opts.text_box;
                if kit::checkbox(ui, t, &mut boxed, tr(Msg::TextBackground)).clicked() {
                    acts.push(CaptureAction::SetTextBox(boxed));
                }
            }
            Tool::Blur => {
                let mut m = opts.blur_mode;
                let items = [
                    (BlurMode::Blur, tr(Msg::ToolBlur)),
                    (BlurMode::Pixelate, tr(Msg::Pixelate)),
                ];
                if kit::segmented(ui, t, &mut m, &items).changed() {
                    acts.push(CaptureAction::SetBlurMode(m));
                }
            }
            _ => {
                divider(ui, t, 20.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                for w in WIDTHS {
                    let tip = format!("{w} px");
                    if kit::width_dot(ui, t, w, (opts.width - w).abs() < 0.01, &tip).clicked() {
                        acts.push(CaptureAction::SetWidth(w));
                    }
                }
                ui.spacing_mut().item_spacing.x = 6.0;
            }
        }
        if matches!(tool, Tool::Rect | Tool::Ellipse) {
            divider(ui, t, 20.0);
            let mut fill = opts.filled;
            if kit::checkbox(ui, t, &mut fill, tr(Msg::Fill)).clicked() {
                acts.push(CaptureAction::SetFill(fill));
            }
        }
        if !doc.selected().is_empty() {
            divider(ui, t, 20.0);
            ui.spacing_mut().item_spacing.x = 2.0;
            for (icon, tip, act) in [
                (
                    Ico::CopyPlus,
                    format!("{} (Ctrl+D)", tr(Msg::Duplicate)),
                    CaptureAction::Duplicate,
                ),
                (
                    Ico::Trash,
                    format!("{} (Del)", tr(Msg::Delete)),
                    CaptureAction::Delete,
                ),
            ] {
                if kit::bar_button(ui, t, kit::BarItem::Icon(icon), &tip).clicked() {
                    acts.push(act);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::engine::doc::Doc;
    use crate::engine::model::{Ann, Shape, Tool};
    use crate::screens::editor::tool_name;
    use crate::screens::overlay::{CaptureAction, Overlay};
    use crate::skins::takeui::testing::{click, run_capture};
    use crate::ui::i18n::{Lang, Msg, lang, set_lang, t as tr};
    use egui::{Rect, pos2};

    /// The 1600 x 900 sample with the area (100,100)-(900,700) and a filled rectangle in it.
    fn capture(tool: Tool) -> (Overlay, crate::Shared) {
        let shared = crate::Shared::for_tests();
        let ov = Overlay::new(
            &egui::Context::default(),
            crate::system::capture::sample_shot(),
            &shared,
        )
        .with_area(Rect::from_min_max(pos2(100.0, 100.0), pos2(900.0, 700.0)))
        .with_doc(
            tool,
            Doc::from_anns(vec![Ann {
                shape: Shape::Rect(pos2(300.0, 300.0), pos2(400.0, 400.0)),
                color: [255, 0, 0, 255],
                width: 4.0,
                filled: true,
            }]),
        );
        (ov, shared)
    }

    fn tool_names() -> Vec<&'static str> {
        [Tool::Select]
            .into_iter()
            .chain(Tool::DRAWING)
            .map(|t| tr(tool_name(t)))
            .collect()
    }

    #[test]
    fn the_options_bar_shows_only_with_a_drawing_tool_or_a_selection() {
        let (mut ov, mut shared) = capture(Tool::Select);
        let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
        let names = tool_names();
        assert!(
            !p.texts.iter().any(|(s, _)| names.contains(&s.as_str())),
            "no options bar with Select and nothing selected: {:?}",
            p.texts
        );
        // A drawing tool: its name heads the options bar.
        let (mut ov, mut shared) = capture(Tool::Ellipse);
        let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
        assert!(p.texts.iter().any(|(s, _)| s == tr(Msg::ToolEllipse)));
        // A selected object: its name, and Duplicate on the options bar.
        let (mut ov, mut shared) = capture(Tool::Select);
        let id = ov.an.doc.objs()[0].id;
        ov.an.doc.select(Some(id), false);
        let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
        assert!(p.texts.iter().any(|(s, _)| s == tr(Msg::ToolRect)));
        let dup = crate::ui::icon::glyph(crate::ui::kit::Ico::CopyPlus).to_string();
        assert!(p.texts.iter().any(|(s, _)| *s == dup));
    }

    #[test]
    fn the_options_bar_offers_a_custom_colour_and_no_object_buttons_without_a_selection() {
        let (mut ov, mut shared) = capture(Tool::Pen);
        let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
        let glyph = |i| crate::ui::icon::glyph(i).to_string();
        assert!(
            p.texts
                .iter()
                .any(|(s, _)| *s == glyph(crate::ui::kit::Ico::Plus))
        );
        for i in [crate::ui::kit::Ico::CopyPlus, crate::ui::kit::Ico::Trash] {
            assert!(!p.texts.iter().any(|(s, _)| *s == glyph(i)), "{i:?}");
        }
    }

    #[test]
    fn edit_and_copy_buttons_act() {
        let (mut ov, mut shared) = capture(Tool::Select);
        let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
        let edit = p
            .texts
            .iter()
            .find(|(s, _)| s == tr(Msg::Edit))
            .map(|(_, r)| r.center())
            .expect("Edit");
        let (acts, _) = run_capture(&mut ov, &mut shared, click(edit));
        assert!(acts.contains(&CaptureAction::Edit), "{acts:?}");
        let copy = crate::ui::icon::glyph(crate::ui::kit::Ico::Copy).to_string();
        let at = p
            .texts
            .iter()
            .find(|(s, _)| *s == copy)
            .map(|(_, r)| r.center())
            .expect("Copy");
        let (acts, _) = run_capture(&mut ov, &mut shared, click(at));
        assert!(acts.contains(&CaptureAction::Copy), "{acts:?}");
    }

    #[test]
    fn capture_texts_follow_the_language() {
        let before = lang();
        for (l, edit, drag) in [(Lang::Pt, "Editar", "Arraste"), (Lang::En, "Edit", "Drag")] {
            set_lang(l);
            let (mut ov, mut shared) = capture(Tool::Select);
            let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
            assert!(
                p.texts.iter().any(|(s, _)| s == edit),
                "{l:?}: {:?}",
                p.texts
            );
            // Before an area is chosen, the hint.
            let mut ov = Overlay::new(
                &egui::Context::default(),
                crate::system::capture::sample_shot(),
                &shared,
            );
            let (_, p) = run_capture(&mut ov, &mut shared, vec![]);
            assert!(
                p.texts.iter().any(|(s, _)| s == drag),
                "{l:?}: {:?}",
                p.texts
            );
        }
        set_lang(before);
    }
}
