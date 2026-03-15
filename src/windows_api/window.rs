use std::mem;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowLongW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, IsWindow, IsWindowVisible, PostMessageW, SetForegroundWindow, SetWindowPos,
    ShowWindow, GWL_EXSTYLE, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE,
    SW_SHOW, WM_CLOSE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
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
/// Filters out: invisible windows, tool windows, non-activatable windows,
/// windows with empty titles, and cloaked (virtual-desktop-hidden) windows.
pub fn is_manageable(hwnd: HWND) -> bool {
    unsafe {
        // Must be visible
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }

        // Check extended styles
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }

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
pub fn set_window_pos(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let flags: SET_WINDOW_POS_FLAGS = SWP_NOZORDER | SWP_NOACTIVATE;
        if let Err(e) = SetWindowPos(hwnd, None, x, y, w, h, flags) {
            log::warn!("SetWindowPos failed for {:?}: {}", hwnd, e);
        }
    }
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
        if let Err(e) = PostMessageW(hwnd, WM_CLOSE, None, None) {
            log::warn!("PostMessageW(WM_CLOSE) failed for {:?}: {}", hwnd, e);
        }
    }
}

/// Brings the window to the foreground.
pub fn set_foreground(hwnd: HWND) {
    unsafe {
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
