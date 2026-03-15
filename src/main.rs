mod config;
mod hotkey;
mod layout;
#[cfg(target_os = "windows")]
mod windows_api;
mod wm;

#[cfg(target_os = "windows")]
fn main() {
    use config::keybinding::parse_key_string;
    use config::parser::{get_default_keybindings, load_config};
    use layout::bsp::Direction;
    use wm::commands::{parse_command, WmCommand};
    use wm::state::WmState;

    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
    };

    env_logger::init();
    log::info!("twm starting...");

    // Set DPI awareness BEFORE any window/monitor operations.
    // Without this, coordinates are wrong on high-DPI displays.
    unsafe {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        log::info!("DPI awareness set to per-monitor v2");
    }

    // Load configuration
    let config = load_config();
    log::info!("Config loaded: {:?}", config.general);

    // Determine keybindings
    let keybinding_entries = if config.keybindings.is_empty() {
        log::info!("No keybindings in config, using defaults");
        get_default_keybindings()
    } else {
        config.keybindings
    };

    // Parse keybindings into KeyBinding for the low-level hook
    let mut bindings: Vec<windows_api::hotkey::KeyBinding> = Vec::new();
    for entry in &keybinding_entries {
        if let Some((modifiers, vk)) = parse_key_string(&entry.key) {
            bindings.push(windows_api::hotkey::KeyBinding {
                modifiers,
                vk,
                command: entry.command.clone(),
            });
            log::info!("Keybinding: {} -> {}", entry.key, entry.command);
        } else {
            log::warn!("Failed to parse keybinding: {}", entry.key);
        }
    }

    // Install low-level keyboard hook (replaces RegisterHotKey)
    windows_api::hotkey::install_hook(bindings);

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
    let mut wm_state = WmState::new(
        monitor_data,
        config.general.gaps.inner,
        config.general.gaps.outer,
    );

    // Set up event hooks
    let _hooks = windows_api::event_hook::setup_event_hooks();
    log::info!("Event hooks installed");

    // Add existing visible windows
    let visible_windows = windows_api::window::get_visible_windows();
    log::info!("Found {} visible windows", visible_windows.len());
    for hwnd in visible_windows {
        let mon_id = windows_api::monitor::monitor_from_window(hwnd);
        wm_state.add_window_to_monitor(hwnd, mon_id);
    }

    // Apply initial layout
    wm_state.apply_all_layouts();
    log::info!("Initial layout applied");

    // Win32 message loop
    // Use a timer to periodically reinstall the keyboard hook as a safety net.
    // Windows can silently remove WH_KEYBOARD_LL hooks if they timeout.
    let hook_check_interval = std::time::Instant::now();
    let mut last_hook_check = hook_check_interval;

    log::info!("Entering message loop");
    unsafe {
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 <= 0 {
                // 0 = WM_QUIT, negative = error
                break;
            }

            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // Periodically ensure the keyboard hook is alive (every 5 seconds)
            let now = std::time::Instant::now();
            if now.duration_since(last_hook_check).as_secs() >= 5 {
                windows_api::hotkey::ensure_hook_alive();
                last_hook_check = now;
            }

            // Process hotkey commands from the low-level keyboard hook
            let commands = windows_api::hotkey::drain_commands();
            for cmd_str in commands {
                log::info!("Hotkey: {}", cmd_str);
                if let Some(cmd) = parse_command(&cmd_str) {
                    execute_command(&mut wm_state, cmd);
                }
            }

            // Drain event queue and process window events
            let events = windows_api::event_hook::drain_events();
            for event in events {
                match event {
                    windows_api::event_hook::WindowEvent::Show(hwnd) => {
                        if windows_api::window::is_manageable(hwnd)
                            && wm_state.find_workspace_for_window(hwnd).is_none()
                        {
                            let mon_id = windows_api::monitor::monitor_from_window(hwnd);
                            wm_state.add_window_to_monitor(hwnd, mon_id);
                            wm_state.apply_all_layouts();
                        }
                    }
                    windows_api::event_hook::WindowEvent::Destroy(hwnd) => {
                        if wm_state.find_workspace_for_window(hwnd).is_some() {
                            wm_state.remove_window(hwnd);
                            wm_state.apply_all_layouts();
                        }
                    }
                    windows_api::event_hook::WindowEvent::FocusChange(hwnd) => {
                        if let Some((mon_idx, _ws_idx)) =
                            wm_state.find_workspace_for_window(hwnd)
                        {
                            wm_state.focused_window = Some(hwnd);
                            wm_state.active_monitor = mon_idx;
                        }
                    }
                }
            }
        }
    }

    // Uninstall keyboard hook on exit
    windows_api::hotkey::uninstall_hook();
    log::info!("twm exiting");
}

/// Execute a parsed WmCommand.
#[cfg(target_os = "windows")]
fn execute_command(wm_state: &mut wm::state::WmState, cmd: wm::commands::WmCommand) {
    use layout::bsp::Direction;
    use wm::commands::WmCommand;

    use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;

    match cmd {
        WmCommand::FocusLeft => wm_state.focus_direction(Direction::Horizontal, false),
        WmCommand::FocusRight => wm_state.focus_direction(Direction::Horizontal, true),
        WmCommand::FocusUp => wm_state.focus_direction(Direction::Vertical, false),
        WmCommand::FocusDown => wm_state.focus_direction(Direction::Vertical, true),
        WmCommand::MoveLeft => wm_state.move_direction(Direction::Horizontal, false),
        WmCommand::MoveRight => wm_state.move_direction(Direction::Horizontal, true),
        WmCommand::MoveUp => wm_state.move_direction(Direction::Vertical, false),
        WmCommand::MoveDown => wm_state.move_direction(Direction::Vertical, true),
        WmCommand::Workspace(idx) => wm_state.switch_workspace(idx),
        WmCommand::MoveToWorkspace(idx) => {
            if let Some(focused) = wm_state.focused_window {
                wm_state.move_window_to_workspace(focused, idx);
            }
        }
        WmCommand::Close => {
            if let Some(focused) = wm_state.focused_window {
                windows_api::window::close_window(focused);
            }
        }
        WmCommand::ToggleFullscreen => wm_state.toggle_fullscreen(),
        WmCommand::ToggleSplitDirection => {
            let ws = wm_state.current_workspace_mut();
            ws.next_direction = match ws.next_direction {
                Direction::Horizontal => Direction::Vertical,
                Direction::Vertical => Direction::Horizontal,
            };
        }
        WmCommand::FocusMonitorNext => wm_state.focus_monitor_next(),
        WmCommand::FocusMonitorPrev => wm_state.focus_monitor_prev(),
        WmCommand::MoveToMonitorNext => wm_state.move_to_monitor_next(),
        WmCommand::MoveToMonitorPrev => wm_state.move_to_monitor_prev(),
        WmCommand::Exec(cmd_str) => {
            log::info!("Executing: {}", cmd_str);
            if let Err(e) = std::process::Command::new("cmd")
                .args(["/C", &cmd_str])
                .spawn()
            {
                log::warn!("Failed to execute '{}': {}", cmd_str, e);
            }
        }
        WmCommand::ReloadConfig => {
            log::info!("Config reload requested (not yet implemented)");
        }
        WmCommand::Exit => {
            log::info!("Exit requested");
            unsafe {
                PostQuitMessage(0);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("This program only runs on Windows");
    std::process::exit(1);
}
