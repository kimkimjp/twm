mod config;
mod layout;
#[cfg(target_os = "windows")]
mod windows_api;
mod wm;

#[cfg(target_os = "windows")]
fn main() {
    use config::parser::load_config;
    use wm::state::WmState;

    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };

    env_logger::init();
    log::info!("twm starting — auto-tiling mode");

    // Set DPI awareness BEFORE any window/monitor operations.
    unsafe {
        use windows::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Load configuration (gaps + window rules only)
    let config = load_config();
    log::info!("Config: gaps inner={}, outer={}", config.gaps.inner, config.gaps.outer);

    // Enumerate monitors
    let monitor_infos = windows_api::monitor::enumerate_monitors();
    log::info!("Found {} monitors", monitor_infos.len());
    for (i, mi) in monitor_infos.iter().enumerate() {
        log::info!("  Monitor {}: id={}, area={:?}", i, mi.id, mi.work_area);
    }

    let monitor_data: Vec<(isize, layout::bsp::Rect)> = monitor_infos
        .iter()
        .map(|mi| {
            (
                mi.id,
                layout::bsp::Rect {
                    x: mi.work_area.x,
                    y: mi.work_area.y,
                    w: mi.work_area.w,
                    h: mi.work_area.h,
                },
            )
        })
        .collect();

    // Initialize WM state
    let mut wm_state = WmState::new(monitor_data, config.gaps.inner, config.gaps.outer);

    // Set up event hooks
    let _hooks = windows_api::event_hook::setup_event_hooks();
    log::info!("Event hooks installed");

    // Add existing visible windows
    let visible_windows = windows_api::window::get_visible_windows();
    log::info!("Found {} visible windows", visible_windows.len());
    for hwnd in visible_windows {
        let mon_id = windows_api::monitor::monitor_from_window(hwnd);
        wm_state.add_window(hwnd, mon_id);
    }

    // Apply initial layout
    wm_state.apply_all_layouts();
    log::info!("Initial layout applied");

    // Message loop — just process window events, no hotkeys
    log::info!("Entering message loop");
    unsafe {
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 <= 0 {
                break;
            }

            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // Process window events
            let events = windows_api::event_hook::drain_events();
            for event in events {
                match event {
                    windows_api::event_hook::WindowEvent::Show(hwnd) => {
                        if windows_api::window::is_manageable(hwnd)
                            && !wm_state.has_window(hwnd)
                        {
                            let mon_id = windows_api::monitor::monitor_from_window(hwnd);
                            wm_state.add_window(hwnd, mon_id);
                            wm_state.apply_all_layouts();
                            log::info!("Window added: {:?}", hwnd);
                        }
                    }
                    windows_api::event_hook::WindowEvent::Destroy(hwnd) => {
                        if wm_state.has_window(hwnd) {
                            wm_state.remove_window(hwnd);
                            wm_state.apply_all_layouts();
                            log::info!("Window removed: {:?}", hwnd);
                        }
                    }
                    windows_api::event_hook::WindowEvent::FocusChange(hwnd) => {
                        // A window came to the foreground. If it's a new
                        // manageable window we haven't seen yet, tile it.
                        // This catches apps that don't fire EVENT_OBJECT_SHOW
                        // reliably (e.g., some UWP apps, electron apps).
                        if windows_api::window::is_manageable(hwnd)
                            && !wm_state.has_window(hwnd)
                        {
                            let mon_id = windows_api::monitor::monitor_from_window(hwnd);
                            wm_state.add_window(hwnd, mon_id);
                            wm_state.apply_all_layouts();
                            log::info!("Window added (via focus): {:?}", hwnd);
                        }
                    }
                }
            }
        }
    }

    log::info!("twm exiting");
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This program only runs on Windows");
    std::process::exit(1);
}
