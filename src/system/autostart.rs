//! Start with the system: the OS launches `takeshot --minimized` at login (Run key, LaunchAgent, XDG autostart).

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

/// The login entry for this executable; nothing changes on the system until `set_autostart`.
pub fn autostart() -> Result<AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut b = AutoLaunchBuilder::new();
    b.set_app_name("Takeshot")
        .set_app_path(&exe.to_string_lossy())
        .set_args(&["--minimized"]);
    #[cfg(windows)]
    b.set_windows_enable_mode(auto_launch::WindowsEnableMode::CurrentUser);
    #[cfg(target_os = "macos")]
    b.set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent);
    #[cfg(target_os = "linux")]
    b.set_linux_launch_mode(auto_launch::LinuxLaunchMode::XdgAutostart);
    b.build().map_err(|e| e.to_string())
}

/// Adds or removes the login entry.
#[allow(dead_code)] // Settings > General calls it from plan 4, Task 6.
pub fn set_autostart(on: bool) -> Result<(), String> {
    let a = autostart()?;
    if on { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
}

/// Whether the login entry exists, read from the system each time (the switch shows the truth).
#[allow(dead_code)] // Settings > General calls it from plan 4, Task 6.
pub fn autostart_enabled() -> bool {
    autostart().is_ok_and(|a| a.is_enabled().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_starts_minimized_as_takeshot() {
        let a = autostart().expect("autostart entry");
        assert_eq!(a.get_app_name(), "Takeshot");
        assert_eq!(a.get_args(), ["--minimized".to_string()]);
    }
}
