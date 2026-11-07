//! Borderless popups without the frame Windows adds: no 1 px top line, no square drop shadow.
//!
//! egui-winit always asks winit for the "undecorated shadow" on a borderless window, and winit
//! gets it by moving the client area 1 px down, which DWM paints as a line across the top and
//! frames with a rectangular shadow. A rounded, transparent popup (the tray menu) shows both as
//! pale square corners. egui cannot turn it off, so the window is subclassed instead.
//!
//! The main window and the editor (the theme's own title bar, decision 13) are subclassed the same
//! way and then get the shadow back from DWM through a 1 px frame inside the client area, which
//! the app paints over; a maximized one is kept to the monitor's work area.

/// Makes every window of this process titled `title` all client area, which also drops the DWM
/// shadow. Call it from the thread that owns the windows, after they exist; a window already done
/// carries a mark and is left alone, so calling it every frame costs a lookup.
#[cfg(windows)]
pub fn flatten_borderless(title: &str) {
    for_each_new_window(title, |_| {});
}

/// Like `flatten_borderless`, for a window with the theme's title bar: DWM draws its shadow again
/// (through a 1 px frame the app paints over) and Windows 11 rounds its corners; Windows 10 just
/// ignores the corner request.
#[cfg(windows)]
#[allow(dead_code)] // The TakeUI shell calls it from plan 5, Task 4.
pub fn frameless_with_shadow(title: &str) {
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmExtendFrameIntoClientArea,
        DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;
    for_each_new_window(title, |hwnd| {
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 1,
            cyBottomHeight: 0,
        };
        let round = DWMWCP_ROUND;
        // SAFETY: a live window of this process; the attribute value outlives the call.
        unsafe {
            DwmExtendFrameIntoClientArea(hwnd, &margins);
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &round as *const _ as *const core::ffi::c_void,
                std::mem::size_of_val(&round) as u32,
            );
        }
    });
}

#[cfg(not(windows))]
#[allow(dead_code)] // The TakeUI shell calls it from plan 5, Task 4.
pub fn frameless_with_shadow(_title: &str) {}

/// Subclasses each window of this process titled `title` that is not done yet, then runs `extra`
/// on it. Call from the thread that owns the windows, after they exist.
#[cfg(windows)]
fn for_each_new_window(title: &str, mut extra: impl FnMut(windows_sys::Win32::Foundation::HWND)) {
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::Shell::SetWindowSubclass;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, GetPropW, GetWindowThreadProcessId, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetPropW, SetWindowPos,
    };
    let wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    let mark = mark();
    // SAFETY: Win32 lookups on top-level windows; the subclass and the mark are only added on
    // windows of this process, from the caller's (owning) thread, and go when the window goes.
    unsafe {
        let me = GetCurrentProcessId();
        let mut after = std::ptr::null_mut();
        loop {
            let hwnd = FindWindowExW(std::ptr::null_mut(), after, std::ptr::null(), wide.as_ptr());
            if hwnd.is_null() {
                break;
            }
            after = hwnd;
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != me || !GetPropW(hwnd, mark.as_ptr()).is_null() {
                continue;
            }
            SetPropW(hwnd, mark.as_ptr(), 1 as _);
            SetWindowSubclass(hwnd, Some(no_frame), SUBCLASS_ID, 0);
            extra(hwnd);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

/// The client area `WM_NCCALCSIZE` gives a frameless window: all of it, or, maximized, the
/// monitor's work area (Windows makes a maximized window larger than the monitor by its frame).
#[cfg(windows)]
pub fn client_rect(
    window: windows_sys::Win32::Foundation::RECT,
    maximized: bool,
    work: windows_sys::Win32::Foundation::RECT,
) -> windows_sys::Win32::Foundation::RECT {
    if maximized { work } else { window }
}

#[cfg(not(windows))]
pub fn flatten_borderless(_title: &str) {}

// GetWindowSubclass would tell whether a window is done, but comctl32 exports it by ordinal only
// (411), which a by-name import cannot load; a window property marks it instead.
#[cfg(windows)]
const SUBCLASS_ID: usize = 0x7a6b_5f01;

#[cfg(windows)]
fn mark() -> Vec<u16> {
    "takeshot.borderless"
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

/// The whole window is client area: no frame for DWM to paint or shadow.
#[cfg(windows)]
unsafe extern "system" fn no_frame(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
    id: usize,
    _data: usize,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{RemovePropW, WM_NCCALCSIZE, WM_NCDESTROY};
    // SAFETY: called by Windows with this window's own message; the rest goes on down the chain.
    unsafe {
        match msg {
            WM_NCCALCSIZE if wparam != 0 => {
                use windows_sys::Win32::Graphics::Gdi::{
                    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
                };
                use windows_sys::Win32::UI::WindowsAndMessaging::{IsZoomed, NCCALCSIZE_PARAMS};
                let params = &mut *(lparam as *mut NCCALCSIZE_PARAMS);
                let maximized = IsZoomed(hwnd) != 0;
                let mut info: MONITORINFO = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
                let work = if maximized
                    && GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info)
                        != 0
                {
                    info.rcWork
                } else {
                    params.rgrc[0]
                };
                params.rgrc[0] = client_rect(params.rgrc[0], maximized, work);
                0
            }
            WM_NCDESTROY => {
                RemovePropW(hwnd, mark().as_ptr());
                RemoveWindowSubclass(hwnd, Some(no_frame), id);
                DefSubclassProc(hwnd, msg, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn a_maximized_frameless_window_fits_the_work_area() {
        use super::client_rect;
        use windows_sys::Win32::Foundation::RECT;
        let r = |left, top, right, bottom| RECT {
            left,
            top,
            right,
            bottom,
        };
        // Windows makes a maximized window larger than the monitor by its frame.
        let window = r(-8, -8, 1928, 1088);
        let work = r(0, 0, 1920, 1040);
        let eq = |a: RECT, b: RECT| {
            (a.left, a.top, a.right, a.bottom) == (b.left, b.top, b.right, b.bottom)
        };
        assert!(
            eq(client_rect(window, true, work), work),
            "maximized: the work area"
        );
        assert!(
            eq(client_rect(window, false, work), window),
            "otherwise: the whole window"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_flattened_window_is_all_client_area() {
        use super::flatten_borderless;
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetClientRect, GetWindowRect, WS_OVERLAPPEDWINDOW,
        };
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let title = "takeshot test flatten";
        let size = |hwnd| {
            let (mut w, mut c) = (RECT::default(), RECT::default());
            // SAFETY: a live window of this thread and plain out-pointers.
            unsafe {
                GetWindowRect(hwnd, &mut w);
                GetClientRect(hwnd, &mut c);
            }
            ((w.right - w.left, w.bottom - w.top), (c.right, c.bottom))
        };
        // SAFETY: a plain STATIC window owned by this test thread, destroyed at the end.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(title).as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                300,
                200,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null());
        let (window, client) = size(hwnd);
        assert_ne!(window, client, "a caption and borders to begin with");
        flatten_borderless(title);
        let (window, client) = size(hwnd);
        assert_eq!(window, client, "the frame is gone");
        flatten_borderless(title);
        assert_eq!(size(hwnd), (window, client), "twice is the same as once");
        // SAFETY: the window created above.
        unsafe { DestroyWindow(hwnd) };
    }
}
