use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
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

/// Returns the work area of the primary monitor (excludes the taskbar).
pub fn get_work_area() -> RECT {
    let mut rect = RECT::default();

    unsafe {
        let result = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );

        if let Err(e) = result {
            log::warn!("SystemParametersInfoW(SPI_GETWORKAREA) failed: {}", e);
            // Return a sensible default (full HD) on failure
            rect = RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            };
        }
    }

    rect
}
