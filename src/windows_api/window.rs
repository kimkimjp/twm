use std::mem;
use std::sync::OnceLock;

use crate::config::parser::WindowRule;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowRect,
    GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, IsZoomed, PostMessageW, SetForegroundWindow, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, GWL_STYLE, GW_OWNER, SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_HIDE, SW_RESTORE, SW_SHOW, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

/// Enumerates all visible, manageable top-level windows.
pub fn get_visible_windows() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut Vec<HWND> as isize),
        );
    }

    windows
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<HWND>);

    if is_manageable(hwnd) {
        windows.push(hwnd);
    }

    BOOL(1) // Continue enumeration
}

/// Determines whether a window should be managed by the tiling WM.
///
/// Uses the same heuristic as the Windows taskbar to decide which windows
/// are "app windows" that should appear as tiles:
/// 1. Must be visible
/// 2. Must NOT have an owner window (dialogs, property sheets, etc. have owners)
/// 3. If it has WS_EX_APPWINDOW, always include it (explicit app window)
/// 4. Must NOT be WS_EX_TOOLWINDOW (floating toolbars, palettes)
/// 5. Must NOT be WS_EX_NOACTIVATE (non-interactive overlays)
/// 6. Must have a non-empty title
/// 7. Must NOT be cloaked (hidden by virtual desktop or UWP)
pub fn is_manageable(hwnd: HWND) -> bool {
    unsafe {
        // Must be visible
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        // Check extended styles
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        // WS_EX_APPWINDOW: explicitly marked as an app window — always manage
        let is_app_window = ex_style & WS_EX_APPWINDOW.0 != 0;

        // Check owner window: owned windows (dialogs, save-as, etc.) should NOT be tiled.
        // GetWindow(GW_OWNER) returns the owner. If non-null, this is an owned window.
        let owner = GetWindow(hwnd, GW_OWNER);
        let has_owner = match owner {
            Ok(h) => h.0 as isize != 0,
            Err(_) => false,
        };

        // Owned windows are excluded UNLESS they have WS_EX_APPWINDOW
        if has_owner && !is_app_window {
            return false;
        }

        // Tool windows are excluded UNLESS they have WS_EX_APPWINDOW
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 && !is_app_window {
            return false;
        }

        // Non-activatable windows (overlays, HUDs) are always excluded
        if ex_style & WS_EX_NOACTIVATE.0 != 0 {
            return false;
        }

        // Must have a non-empty title
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len == 0 {
            return false;
        }

        // Exclude cloaked windows (hidden by virtual desktops, UWP, etc.)
        if is_cloaked(hwnd) {
            return false;
        }

        // Exclude windows matching "ignore" window rules from config
        if matches_ignore_rule(hwnd) {
            return false;
        }

        true
    }
}

/// Checks if a window is cloaked (hidden by DWM, e.g. on another virtual desktop).
fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            mem::size_of::<u32>() as u32,
        )
    };

    result.is_ok() && cloaked != 0
}

/// Moves and resizes a window without changing Z-order or activation state.
///
/// Handles:
/// - Restoring maximized/minimized windows before repositioning
/// - Compensating for DWM invisible borders (Windows 10/11)
/// - Using SWP_FRAMECHANGED to force window frame update
pub fn set_window_pos(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        // Restore maximized windows — SetWindowPos doesn't work on maximized windows
        if IsZoomed(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        // Skip minimized windows — don't force-restore them
        if IsIconic(hwnd).as_bool() {
            return;
        }

        // Compensate for DWM invisible borders.
        // GetWindowRect returns bounds INCLUDING invisible borders.
        // DWMWA_EXTENDED_FRAME_BOUNDS returns the VISIBLE bounds.
        // The difference is the invisible border we need to account for.
        let (adj_x, adj_y, adj_w, adj_h) = get_border_compensation(hwnd, x, y, w, h);

        let flags: SET_WINDOW_POS_FLAGS = SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED;
        if let Err(e) = SetWindowPos(hwnd, None, adj_x, adj_y, adj_w, adj_h, flags) {
            log::warn!("SetWindowPos failed for {:?}: {}", hwnd, e);
        }
    }
}

/// Calculates border compensation to account for DWM invisible borders.
/// Returns adjusted (x, y, w, h) for SetWindowPos.
fn get_border_compensation(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> (i32, i32, i32, i32) {
    let mut window_rect = RECT::default();
    let mut frame_rect = RECT::default();

    unsafe {
        if GetWindowRect(hwnd, &mut window_rect).is_err() {
            return (x, y, w, h);
        }

        let result = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame_rect as *mut RECT as *mut _,
            mem::size_of::<RECT>() as u32,
        );

        if result.is_err() {
            return (x, y, w, h);
        }
    }

    // Calculate invisible border sizes
    let border_left = frame_rect.left - window_rect.left;
    let border_right = window_rect.right - frame_rect.right;
    let border_top = frame_rect.top - window_rect.top;
    let border_bottom = window_rect.bottom - frame_rect.bottom;

    // Expand the SetWindowPos rect to compensate for invisible borders
    (
        x - border_left,
        y - border_top,
        w + border_left + border_right,
        h + border_top + border_bottom,
    )
}

/// Gets the accurate window bounds using DWM extended frame bounds.
/// Falls back to GetWindowRect if DWM query fails.
pub fn get_window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();

    // Try DWM extended frame bounds first (excludes invisible borders)
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut _,
            mem::size_of::<RECT>() as u32,
        )
    };

    if result.is_ok() {
        return Some(rect);
    }

    // Fallback to GetWindowRect
    unsafe {
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            Some(rect)
        } else {
            log::warn!("GetWindowRect failed for {:?}", hwnd);
            None
        }
    }
}

/// Returns the window title text.
pub fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return String::new();
        }

        let mut buf = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

/// Returns the window class name.
pub fn get_window_class(hwnd: HWND) -> String {
    unsafe {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

/// Sends WM_CLOSE to request the window to close gracefully.
pub fn close_window(hwnd: HWND) {
    unsafe {
        use windows::Win32::Foundation::{WPARAM, LPARAM};
        if let Err(e) = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)) {
            log::warn!("PostMessageW(WM_CLOSE) failed for {:?}: {}", hwnd, e);
        }
    }
}

/// Brings the window to the foreground.
///
/// Uses AttachThreadInput trick to bypass Windows' SetForegroundWindow
/// restrictions. Without this, SetForegroundWindow silently fails when
/// our process doesn't currently own the foreground.
pub fn set_foreground(hwnd: HWND) {
    unsafe {
        let current_thread = GetCurrentThreadId();
        let foreground_hwnd = GetForegroundWindow();

        if foreground_hwnd.0 as isize != 0 {
            let foreground_thread = GetWindowThreadProcessId(foreground_hwnd, None);

            if foreground_thread != current_thread {
                // Attach to the foreground thread's input to gain permission
                let _ = AttachThreadInput(current_thread, foreground_thread, true);
                let _ = SetForegroundWindow(hwnd);
                let _ = AttachThreadInput(current_thread, foreground_thread, false);
                return;
            }
        }

        let _ = SetForegroundWindow(hwnd);
    }
}

/// Checks if the HWND still refers to a valid window.
pub fn is_window_valid(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd).as_bool() }
}

/// Shows or hides a window.
pub fn show_window(hwnd: HWND, show: bool) {
    unsafe {
        let cmd = if show { SW_SHOW } else { SW_HIDE };
        let _ = ShowWindow(hwnd, cmd);
    }
}

/// Checks if a window is minimized.
pub fn is_window_minimized(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

/// Global storage for window rules loaded from config.
static WINDOW_RULES: OnceLock<Vec<WindowRule>> = OnceLock::new();

/// Store window rules for use by is_manageable().
pub fn set_window_rules(rules: Vec<WindowRule>) {
    let _ = WINDOW_RULES.set(rules);
}

/// Check if a window matches any "ignore" window rule.
fn matches_ignore_rule(hwnd: HWND) -> bool {
    let rules = match WINDOW_RULES.get() {
        Some(r) => r,
        None => return false,
    };

    // Only evaluate rules that are "ignore" commands
    let ignore_rules: Vec<&WindowRule> = rules.iter().filter(|r| r.command == "ignore").collect();
    if ignore_rules.is_empty() {
        return false;
    }

    let class = get_window_class(hwnd);
    let title = get_window_title(hwnd);

    for rule in ignore_rules {
        let class_match = match &rule.class {
            Some(pattern) => class.contains(pattern.as_str()),
            None => true,
        };
        let title_match = match &rule.title {
            Some(pattern) => title.contains(pattern.as_str()),
            None => true,
        };
        if class_match && title_match {
            return true;
        }
    }

    false
}
