//! Pointer policy: which cursor shows what a click will do, from the tool, handles, objects and capture area.

use crate::engine::doc::Handle;
use crate::engine::model::Tool;
use egui::CursorIcon;

/// Where the pointer is relative to the capture area; the editor has none.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Area {
    NotStarted,
    Outside,
    Inside,
    Handle(Handle),
    NotApplicable,
}

#[derive(Clone, Copy, Debug)]
pub struct CursorCtx {
    pub tool: Tool,
    /// Over the picture being annotated: the editor canvas, or the capture's chosen area.
    pub over_image: bool,
    pub over_chrome: bool,
    /// Handle of a selected object under the pointer, or being dragged.
    pub handle: Option<Handle>,
    pub over_object: bool,
    /// Over a selected object: any tool moves it by its body.
    pub over_selection: bool,
    pub typing: bool,
    pub dragging: bool,
    pub space_pan: bool,
    pub area: Area,
}

fn resize_icon(h: Handle, dragging: bool) -> CursorIcon {
    match h {
        Handle::N | Handle::S => CursorIcon::ResizeVertical,
        Handle::E | Handle::W => CursorIcon::ResizeHorizontal,
        Handle::NW | Handle::SE => CursorIcon::ResizeNwSe,
        Handle::NE | Handle::SW => CursorIcon::ResizeNeSw,
        Handle::Start | Handle::End if dragging => CursorIcon::Grabbing,
        Handle::Start | Handle::End => CursorIcon::Grab,
    }
}

/// The pointer for this frame, in priority order: typing, space pan, bars and panels, object handle,
/// area not started, the selection with any tool or another object with Select, drawing tool over
/// the image, area handle, inside the area.
pub fn pick(c: &CursorCtx) -> CursorIcon {
    if c.typing {
        return CursorIcon::Text;
    }
    if c.space_pan {
        return if c.dragging {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        };
    }
    if c.over_chrome {
        return CursorIcon::Default;
    }
    if let Some(h) = c.handle {
        return resize_icon(h, c.dragging);
    }
    if c.area == Area::NotStarted {
        return CursorIcon::Crosshair;
    }
    if c.over_selection || (c.over_object && c.tool == Tool::Select) {
        return CursorIcon::Move;
    }
    // A drawing tool draws inside the capture area, or over the picture outside the capture;
    // outside a chosen area it shows the arrow (spec 5.8).
    let drawable = c.area == Area::Inside || (c.over_image && c.area != Area::Outside);
    if drawable && c.tool != Tool::Select {
        return CursorIcon::Crosshair;
    }
    match c.area {
        Area::Handle(h) => resize_icon(h, c.dragging),
        Area::Inside => CursorIcon::Move,
        _ => CursorIcon::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_table() {
        use egui::CursorIcon as C;
        let base = CursorCtx {
            tool: Tool::Select,
            over_image: true,
            over_chrome: false,
            handle: None,
            over_object: false,
            over_selection: false,
            typing: false,
            dragging: false,
            space_pan: false,
            area: Area::NotApplicable,
        };
        let cases = [
            // Any tool moves the selection by its body (the shape just drawn stays selected).
            (
                CursorCtx {
                    over_object: true,
                    over_selection: true,
                    tool: Tool::Pen,
                    ..base
                },
                C::Move,
            ),
            (
                CursorCtx {
                    over_object: true,
                    over_selection: true,
                    tool: Tool::Text,
                    area: Area::Inside,
                    ..base
                },
                C::Move,
            ),
            (
                CursorCtx {
                    over_chrome: true,
                    tool: Tool::Arrow,
                    ..base
                },
                C::Default,
            ),
            (
                CursorCtx {
                    tool: Tool::Arrow,
                    ..base
                },
                C::Crosshair,
            ),
            (
                CursorCtx {
                    tool: Tool::Arrow,
                    over_image: false,
                    ..base
                },
                C::Default,
            ),
            (
                CursorCtx {
                    over_object: true,
                    ..base
                },
                C::Move,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::SE),
                    ..base
                },
                C::ResizeNwSe,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::NE),
                    tool: Tool::Rect,
                    ..base
                },
                C::ResizeNeSw,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::N),
                    ..base
                },
                C::ResizeVertical,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::W),
                    ..base
                },
                C::ResizeHorizontal,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::End),
                    ..base
                },
                C::Grab,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::End),
                    dragging: true,
                    ..base
                },
                C::Grabbing,
            ),
            (
                CursorCtx {
                    typing: true,
                    ..base
                },
                C::Text,
            ),
            (
                CursorCtx {
                    space_pan: true,
                    ..base
                },
                C::Grab,
            ),
            (
                CursorCtx {
                    space_pan: true,
                    dragging: true,
                    tool: Tool::Pen,
                    ..base
                },
                C::Grabbing,
            ),
            (
                CursorCtx {
                    area: Area::NotStarted,
                    ..base
                },
                C::Crosshair,
            ),
            (
                CursorCtx {
                    area: Area::Inside,
                    ..base
                },
                C::Move,
            ),
            (
                CursorCtx {
                    area: Area::Outside,
                    ..base
                },
                C::Default,
            ),
            (
                CursorCtx {
                    area: Area::Handle(Handle::E),
                    ..base
                },
                C::ResizeHorizontal,
            ),
            (
                CursorCtx {
                    area: Area::Inside,
                    over_object: true,
                    ..base
                },
                C::Move,
            ),
            (
                CursorCtx {
                    area: Area::Inside,
                    tool: Tool::Pen,
                    ..base
                },
                C::Crosshair,
            ),
            // Beyond the brief: the remaining directions and the priority edges.
            (
                CursorCtx {
                    handle: Some(Handle::NW),
                    ..base
                },
                C::ResizeNwSe,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::SW),
                    ..base
                },
                C::ResizeNeSw,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::S),
                    ..base
                },
                C::ResizeVertical,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::E),
                    ..base
                },
                C::ResizeHorizontal,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::Start),
                    ..base
                },
                C::Grab,
            ),
            (
                CursorCtx {
                    over_object: true,
                    tool: Tool::Pen,
                    ..base
                },
                C::Crosshair,
            ),
            (
                CursorCtx {
                    area: Area::Handle(Handle::NW),
                    tool: Tool::Pen,
                    over_image: false,
                    ..base
                },
                C::ResizeNwSe,
            ),
            (
                CursorCtx {
                    area: Area::Outside,
                    tool: Tool::Pen,
                    over_image: false,
                    ..base
                },
                C::Default,
            ),
            (
                CursorCtx {
                    typing: true,
                    over_chrome: true,
                    ..base
                },
                C::Text,
            ),
            (
                CursorCtx {
                    over_chrome: true,
                    handle: Some(Handle::SE),
                    ..base
                },
                C::Default,
            ),
            (
                CursorCtx {
                    over_image: false,
                    ..base
                },
                C::Default,
            ),
            // Spec 5.8: inside the area a drawing tool draws; outside it, once chosen, the arrow.
            (
                CursorCtx {
                    area: Area::Inside,
                    tool: Tool::Pen,
                    over_image: false,
                    ..base
                },
                C::Crosshair,
            ),
            (
                CursorCtx {
                    area: Area::Outside,
                    tool: Tool::Pen,
                    ..base
                },
                C::Default,
            ),
            // Priority edges.
            (
                CursorCtx {
                    space_pan: true,
                    over_chrome: true,
                    ..base
                },
                C::Grab,
            ),
            (
                CursorCtx {
                    typing: true,
                    space_pan: true,
                    ..base
                },
                C::Text,
            ),
            (
                CursorCtx {
                    handle: Some(Handle::S),
                    area: Area::NotStarted,
                    ..base
                },
                C::ResizeVertical,
            ),
            (
                CursorCtx {
                    area: Area::NotStarted,
                    over_object: true,
                    ..base
                },
                C::Crosshair,
            ),
            (
                CursorCtx {
                    tool: Tool::Text,
                    ..base
                },
                C::Crosshair,
            ),
        ];
        for (i, (ctx, want)) in cases.iter().enumerate() {
            assert_eq!(pick(ctx), *want, "caso {i}: {ctx:?}");
        }
    }
}
