use std::sync::Mutex;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};

/// Events produced by the WinEvent hooks.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// A new window was shown.
    Show(HWND),
    /// A window was destroyed.
    Destroy(HWND),
    /// The foreground window changed.
    FocusChange(HWND),
}

// HWND is not Send by default, but we only pass them as opaque handles
// between the hook callback and the main thread on the same process.
unsafe impl Send for WindowEvent {}

/// Global event queue populated by the WinEvent callback.
static EVENTS: Mutex<Vec<WindowEvent>> = Mutex::new(Vec::new());

/// Drains all pending events from the queue and returns them.
pub fn drain_events() -> Vec<WindowEvent> {
    let mut queue = EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    queue.drain(..).collect()
}

/// Installs WinEvent hooks for window show, destroy, and foreground changes.
///
/// The returned handles must be kept alive for as long as the hooks should
/// remain active. Dropping them does NOT automatically unhook; the hooks
/// persist until the thread's message loop exits.
pub fn setup_event_hooks() -> Vec<HWINEVENTHOOK> {
    let mut hooks = Vec::new();

    let events = [
        (EVENT_OBJECT_SHOW, EVENT_OBJECT_SHOW),
        (EVENT_OBJECT_DESTROY, EVENT_OBJECT_DESTROY),
        (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
    ];

    for (min, max) in events {
        let hook = unsafe {
            SetWinEventHook(
                min,
                max,
                None,
                Some(win_event_proc),
                0, // All processes
                0, // All threads
                WINEVENT_OUTOFCONTEXT,
            )
        };

        if hook.is_invalid() {
            log::warn!(
                "SetWinEventHook failed for event range 0x{:X}..0x{:X}",
                min,
                max
            );
        } else {
            hooks.push(hook);
        }
    }

    hooks
}

/// WinEvent callback. Converts raw events into `WindowEvent` values and
/// pushes them onto the global queue.
unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    let window_event = match event {
        e if e == EVENT_OBJECT_SHOW => WindowEvent::Show(hwnd),
        e if e == EVENT_OBJECT_DESTROY => WindowEvent::Destroy(hwnd),
        e if e == EVENT_SYSTEM_FOREGROUND => WindowEvent::FocusChange(hwnd),
        _ => return,
    };

    if let Ok(mut queue) = EVENTS.lock() {
        queue.push(window_event);
    }
}
