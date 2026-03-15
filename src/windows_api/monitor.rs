use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO,
    MONITOR_DEFAULTTONEAREST,
};

/// A simple rectangle representing position and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl From<RECT> for Rect {
    fn from(r: RECT) -> Self {
        Rect {
            x: r.left,
            y: r.top,
            w: r.right - r.left,
            h: r.bottom - r.top,
        }
    }
}

/// Information about a single monitor.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// HMONITOR internal value used to identify the monitor.
    pub id: isize,
    /// Work area excluding the taskbar.
    pub work_area: Rect,
}

/// Enumerates all monitors and returns their work areas, sorted left-to-right by x coordinate.
pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();

    unsafe {
        let result = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_monitor_callback),
            LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize),
        );

        if !result.as_bool() {
            log::warn!("EnumDisplayMonitors failed");
        }
    }

    // Sort monitors left-to-right by x coordinate
    monitors.sort_by_key(|m| m.work_area.x);

    monitors
}

unsafe extern "system" fn enum_monitor_callback(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);

    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    let result = GetMonitorInfoW(hmonitor, &mut mi);
    if result.as_bool() {
        monitors.push(MonitorInfo {
            id: hmonitor.0 as isize,
            work_area: Rect::from(mi.rcWork),
        });
    } else {
        log::warn!("GetMonitorInfoW failed for monitor {:?}", hmonitor.0 as isize);
    }

    BOOL(1) // Continue enumeration
}

/// Returns the HMONITOR id (as isize) for the monitor that contains the given window.
/// Falls back to the nearest monitor if the window is not on any monitor.
pub fn monitor_from_window(hwnd: HWND) -> isize {
    let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    hmonitor.0 as isize
}
