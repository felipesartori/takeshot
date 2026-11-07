//! PrtSc priority on Windows: a low-level keyboard hook that takes PrtSc before other apps.

// The resident installs it from plan 4, Task 6; until then only the tests use it.
#![allow(dead_code)]

use crate::system::hotkeys::Shot;

/// Windows' virtual-key code for Print Screen.
const VK_SNAPSHOT: u32 = 0x2C;

/// Which capture a key event starts: a fresh PrtSc press (not auto-repeat), Shift choosing the full screen.
pub fn hook_decision(vk: u32, key_down: bool, shift: bool, already_down: bool) -> Option<Shot> {
    if vk != VK_SNAPSHOT || !key_down || already_down {
        return None;
    }
    Some(if shift { Shot::Full } else { Shot::Area })
}

/// The hook hides every PrtSc event, press and release, from other apps (the Snipping Tool can act
/// on the release); everything else passes through.
pub fn swallows(vk: u32) -> bool {
    vk == VK_SNAPSHOT
}

#[cfg(windows)]
#[allow(unused_imports)] // Used by the resident from plan 4, Task 6.
pub use imp::PrtScHook;

#[cfg(windows)]
mod imp {
    use super::{hook_decision, swallows};
    use crate::system::hotkeys::Shot;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, MSG, PostThreadMessageW, SetWindowsHookExW,
        UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    type Sink = Arc<dyn Fn(Shot) + Send + Sync>;

    /// The hook procedure is a plain function, so it reaches the callback through a global.
    static SINK: Mutex<Option<Sink>> = Mutex::new(None);
    /// PrtSc is held down: its auto-repeat must not start more captures.
    static DOWN: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            // SAFETY: for WH_KEYBOARD_LL with code >= 0, lparam points to a KBDLLHOOKSTRUCT.
            let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
            if swallows(kb.vkCode) {
                let down = wparam == WM_KEYDOWN as WPARAM || wparam == WM_SYSKEYDOWN as WPARAM;
                // SAFETY: plain Win32 call without pointers.
                let shift = unsafe { GetAsyncKeyState(VK_SHIFT as i32) } < 0;
                let was_down = DOWN.swap(down, Ordering::Relaxed);
                if let Some(shot) = hook_decision(kb.vkCode, down, shift, was_down) {
                    let sink = SINK.lock().ok().and_then(|s| s.clone());
                    if let Some(sink) = sink {
                        sink(shot);
                    }
                }
                return 1;
            }
        }
        // SAFETY: forwards the event unchanged to the next hook.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }

    /// The installed hook; dropping it removes the hook and ends its thread.
    pub struct PrtScHook {
        thread_id: u32,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl PrtScHook {
        /// Installs the hook on its own thread with a message loop, as low-level hooks require.
        /// `on_shot` must return quickly: Windows drops hooks that stall input.
        pub fn install(on_shot: impl Fn(Shot) + Send + Sync + 'static) -> std::io::Result<Self> {
            *SINK
                .lock()
                .map_err(|_| std::io::Error::other("hook state poisoned"))? =
                Some(Arc::new(on_shot));
            let (tx, rx) = channel();
            let thread = std::thread::Builder::new()
                .name("takeshot-prtsc-hook".into())
                .spawn(move || {
                    // SAFETY: Win32 calls on this thread; the hook handle never leaves it.
                    unsafe {
                        let module = GetModuleHandleW(std::ptr::null());
                        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
                        if hook.is_null() {
                            let _ = tx.send(Err(std::io::Error::last_os_error()));
                            return;
                        }
                        let _ = tx.send(Ok(GetCurrentThreadId()));
                        let mut msg: MSG = std::mem::zeroed();
                        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
                        UnhookWindowsHookEx(hook);
                    }
                })?;
            let thread_id = rx
                .recv()
                .map_err(|_| std::io::Error::other("hook thread ended"))??;
            Ok(Self {
                thread_id,
                thread: Some(thread),
            })
        }
    }

    impl Drop for PrtScHook {
        fn drop(&mut self) {
            // SAFETY: posts WM_QUIT to the hook thread's own message queue.
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            if let Ok(mut s) = SINK.lock() {
                *s = None;
            }
            DOWN.store(false, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VK_A: u32 = 0x41;

    #[test]
    fn the_hook_fires_on_a_fresh_prtsc_press_only() {
        assert_eq!(
            hook_decision(VK_SNAPSHOT, true, false, false),
            Some(Shot::Area)
        );
        assert_eq!(
            hook_decision(VK_SNAPSHOT, true, true, false),
            Some(Shot::Full)
        );
        assert_eq!(
            hook_decision(VK_SNAPSHOT, false, false, true),
            None,
            "key up"
        );
        assert_eq!(
            hook_decision(VK_SNAPSHOT, true, false, true),
            None,
            "held down: auto-repeat"
        );
        assert_eq!(hook_decision(VK_A, true, false, false), None);
    }

    #[test]
    fn the_hook_swallows_every_prtsc_event_and_nothing_else() {
        assert!(swallows(VK_SNAPSHOT));
        assert!(!swallows(VK_A));
    }

    #[cfg(windows)]
    #[test]
    fn the_hook_installs_and_uninstalls() {
        let hook = PrtScHook::install(|_| {}).expect("install the keyboard hook");
        drop(hook);
    }
}
