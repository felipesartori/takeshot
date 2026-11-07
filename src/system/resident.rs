//! The resident process: single instance, hotkeys, PrtSc hook and tray feed one event queue the app drains each frame.

use crate::system::hotkeys::{Hotkeys, Shot};
use crate::system::instance::{Listener, Page};
use crate::system::settings::{OnClose, Settings, TrayClick};
use crate::system::tray::{Tray, TrayEvent};
use std::sync::mpsc;

/// Something the resident must react to on the UI thread.
#[derive(Clone, PartialEq, Debug)]
pub enum ResidentEvent {
    Show(Page),
    Shot(Shot),
    Tray(TrayEvent),
}

/// What the X of the main window does right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum XAction {
    Ask,
    Hide,
    Quit,
}

pub fn on_x(setting: OnClose) -> XAction {
    match setting {
        OnClose::Ask => XAction::Ask,
        OnClose::Minimize => XAction::Hide,
        OnClose::Quit => XAction::Quit,
    }
}

/// A left click on the tray icon, as the setting wants it.
pub fn left_click(setting: TrayClick) -> ResidentEvent {
    match setting {
        TrayClick::OpenHome => ResidentEvent::Show(Page::Start),
        TrayClick::CaptureArea => ResidentEvent::Shot(Shot::Area),
    }
}

/// The UI-thread end of the event queue.
pub struct Queue {
    rx: mpsc::Receiver<ResidentEvent>,
}

/// The end the hotkey, hook, tray and instance threads hold: sending also wakes the app.
#[derive(Clone)]
pub struct Sender {
    tx: mpsc::Sender<ResidentEvent>,
    ctx: egui::Context,
}

impl Queue {
    pub fn new(ctx: &egui::Context) -> (Self, Sender) {
        let (tx, rx) = mpsc::channel();
        (
            Self { rx },
            Sender {
                tx,
                ctx: ctx.clone(),
            },
        )
    }

    pub fn drain(&self) -> Vec<ResidentEvent> {
        self.rx.try_iter().collect()
    }
}

impl Sender {
    /// Queues the event and asks egui for a frame; cheap enough for a keyboard hook callback.
    pub fn send(&self, e: ResidentEvent) {
        let _ = self.tx.send(e);
        self.ctx.request_repaint();
    }
}

/// What the resident owns: the hotkeys, the PrtSc hook (Windows), the tray icon and the queue
/// their threads feed. It lives on the main thread (the tray icon must).
pub struct Resident {
    queue: Queue,
    sender: Sender,
    pub hotkeys: Hotkeys,
    #[cfg(windows)]
    hook: Option<crate::system::prtsc_hook::PrtScHook>,
    pub tray: Option<Tray>,
}

impl Resident {
    /// Starts everything; `listener` answers second launches when this process won the name.
    pub fn start(ctx: &egui::Context, settings: &Settings, listener: Option<Listener>) -> Self {
        let (queue, sender) = Queue::new(ctx);
        if let Some(l) = listener {
            let s = sender.clone();
            l.spawn(move |page| s.send(ResidentEvent::Show(page)));
        }
        let s = sender.clone();
        let hotkeys = Hotkeys::start(settings, move |shot| s.send(ResidentEvent::Shot(shot)));
        let s = sender.clone();
        let tray = match Tray::start(settings.ui_theme, move |e| s.send(ResidentEvent::Tray(e))) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("takeshot: no tray icon: {e}");
                None
            }
        };
        let mut r = Self {
            queue,
            sender,
            hotkeys,
            #[cfg(windows)]
            hook: None,
            tray,
        };
        r.set_priority(settings.prtsc_priority);
        r
    }

    pub fn poll(&self) -> Vec<ResidentEvent> {
        self.queue.drain()
    }

    /// Turns the PrtSc priority hook on or off (Windows; elsewhere there is none).
    pub fn set_priority(&mut self, on: bool) {
        #[cfg(windows)]
        {
            if on && self.hook.is_none() {
                let s = self.sender.clone();
                self.hook = match crate::system::prtsc_hook::PrtScHook::install(move |shot| {
                    s.send(ResidentEvent::Shot(shot))
                }) {
                    Ok(h) => Some(h),
                    Err(e) => {
                        eprintln!("takeshot: PrtSc priority unavailable: {e}");
                        None
                    }
                };
            } else if !on {
                self.hook = None;
            }
        }
        #[cfg(not(windows))]
        let _ = (on, &self.sender);
    }

    pub fn priority_on(&self) -> bool {
        #[cfg(windows)]
        return self.hook.is_some();
        #[cfg(not(windows))]
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::settings::{OnClose, TrayClick};

    #[test]
    fn x_follows_the_remembered_choice() {
        assert_eq!(on_x(OnClose::Ask), XAction::Ask);
        assert_eq!(on_x(OnClose::Minimize), XAction::Hide);
        assert_eq!(on_x(OnClose::Quit), XAction::Quit);
    }

    #[test]
    fn left_click_on_the_icon_follows_the_setting() {
        assert_eq!(
            left_click(TrayClick::OpenHome),
            ResidentEvent::Show(Page::Start)
        );
        assert_eq!(
            left_click(TrayClick::CaptureArea),
            ResidentEvent::Shot(Shot::Area)
        );
    }

    #[test]
    fn events_reach_the_queue_and_wake_the_app() {
        let ctx = egui::Context::default();
        let (q, sender) = Queue::new(&ctx);
        sender.send(ResidentEvent::Show(Page::Settings));
        sender.send(ResidentEvent::Shot(Shot::Full));
        assert_eq!(
            q.drain(),
            vec![
                ResidentEvent::Show(Page::Settings),
                ResidentEvent::Shot(Shot::Full)
            ]
        );
        assert!(q.drain().is_empty());
    }
}
