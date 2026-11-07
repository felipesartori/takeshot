//! macOS: the traffic lights inside the theme's title bar (decision 13). The window keeps its
//! native frame with the content under a transparent title bar; an empty unified-compact toolbar
//! makes that bar as tall as the theme's and centres the lights in it.

// The TakeUI shell calls it from plan 5, Task 4.
#![allow(dead_code)]

/// Gives the window of this app titled `title` the transparent bar and the toolbar that centres
/// the traffic lights. Main thread only; a window already done keeps its toolbar.
pub fn center_traffic_lights(title: &str) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSToolbar, NSWindowTitleVisibility, NSWindowToolbarStyle};
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        if window.title().to_string() != title || window.toolbar().is_some() {
            continue;
        }
        window.setTitlebarAppearsTransparent(true);
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        let toolbar = NSToolbar::new(mtm);
        window.setToolbar(Some(&toolbar));
        window.setToolbarStyle(NSWindowToolbarStyle::UnifiedCompact);
    }
}
