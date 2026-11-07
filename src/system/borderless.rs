//! Borderless popups without the frame Windows adds: no 1 px top line, no square drop shadow.
//!
//! egui-winit always asks winit for the "undecorated shadow" on a borderless window, and winit
//! gets it by moving the client area 1 px down, which DWM paints as a line across the top and
//! frames with a rectangular shadow. A rounded, transparent popup (the tray menu) shows both as
//! pale square corners. egui cannot turn it off, so the window is subclassed instead.

/// Makes every window of this process titled `title` all client area, which also drops the DWM
/// shadow. Call it from the thread that owns the windows, after they exist; a window already done
/// carries a mark and is left alone, so calling it every frame costs a lookup.
#[cfg(windows)]
pub fn flatten_borderless(title: &str) {
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
            WM_NCCALCSIZE if wparam != 0 => 0,
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
