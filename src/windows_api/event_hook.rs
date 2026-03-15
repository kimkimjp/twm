use std::sync::Mutex;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};

/// EVENT_SYSTEM_MOVESIZEEND = 0x000B — fired when window move/resize completes
const EVENT_SYSTEM_MOVESIZEEND: u32 = 0x000B;

/// Events produced by the WinEvent hooks.
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// A new window was shown.
    Show(HWND),
    /// A window was destroyed.
    Destroy(HWND),
    /// The foreground window changed.
    FocusChange(HWND),
    /// A window finished being moved/resized.
    MoveEnd(HWND),
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
        (EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND),
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
    id_object: i32,
    id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    // OBJID_WINDOW = 0, CHILDID_SELF = 0
    // Only handle top-level window events, not child elements.
    // Without this filter, every button/control show/destroy fires an event,
    // causing massive churn and making the WM unresponsive.
    if id_object != 0 || id_child != 0 {
        return;
    }

    // Skip null/invalid window handles
    if hwnd.0 as isize == 0 {
        return;
    }

    let window_event = match event {
        e if e == EVENT_OBJECT_SHOW => WindowEvent::Show(hwnd),
        e if e == EVENT_OBJECT_DESTROY => WindowEvent::Destroy(hwnd),
        e if e == EVENT_SYSTEM_FOREGROUND => WindowEvent::FocusChange(hwnd),
        e if e == EVENT_SYSTEM_MOVESIZEEND => WindowEvent::MoveEnd(hwnd),
        _ => return,
    };

    // Use same poisoning recovery strategy as drain_events() (BUG-11)
    let mut queue = EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    queue.push(window_event);
}
