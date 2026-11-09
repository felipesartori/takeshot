//! macOS: the traffic lights inside the theme's title bar (decision 13). The window keeps its
//! native frame with the content under a transparent title bar; an empty compact unified toolbar
//! (about 38 pt) keeps the lights near the middle of the theme's 44 px bar. Also whether the app
//! shows in the Dock.

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
        // The compact unified toolbar is the closest to the theme bar's 44 px (the full one is 52).
        window.setToolbarStyle(NSWindowToolbarStyle::UnifiedCompact);
    }
}

/// Shows this process in the Dock and the app switcher, or keeps it to the menu bar icon (an
/// accessory app, whose windows still open). Main thread only.
pub fn show_in_dock(show: bool) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy as Policy};
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let policy = if show {
        Policy::Regular
    } else {
        Policy::Accessory
    };
    NSApplication::sharedApplication(mtm).setActivationPolicy(policy);
}
