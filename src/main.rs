mod config;
mod hotkey;
mod layout;
#[cfg(target_os = "windows")]
mod windows_api;
mod wm;

#[cfg(target_os = "windows")]
fn main() {
    use config::keybinding::{parse_key_string, HotkeyBinding};
    use config::parser::{get_default_keybindings, load_config};
    use layout::bsp::Direction;
    use wm::commands::{parse_command, WmCommand};
    use wm::state::WmState;

    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG, WM_HOTKEY,
    };

    env_logger::init();
    log::info!("twm starting...");

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

    // Parse keybindings into HotkeyBindings
    let mut hotkey_bindings: Vec<HotkeyBinding> = Vec::new();
    for (i, entry) in keybinding_entries.iter().enumerate() {
        if let Some((modifiers, vk)) = parse_key_string(&entry.key) {
            hotkey_bindings.push(HotkeyBinding {
                id: (i + 1) as i32,
                modifiers,
                vk,
                command: entry.command.clone(),
            });
        } else {
            log::warn!("Failed to parse keybinding: {}", entry.key);
        }
    }

    // Get work area
    let wa = windows_api::monitor::get_work_area();
    let work_area = layout::bsp::Rect {
        x: wa.left,
        y: wa.top,
        w: wa.right - wa.left,
        h: wa.bottom - wa.top,
    };
    log::info!("Work area: {:?}", work_area);

    // Initialize WM state
    let mut wm_state =
        WmState::new(work_area, config.general.gaps.inner, config.general.gaps.outer);

    // Set up event hooks
    let _hooks = windows_api::event_hook::setup_event_hooks();
    log::info!("Event hooks installed");

    // Register hotkeys
    for binding in &hotkey_bindings {
        if windows_api::hotkey::register_hotkey(binding.id, binding.modifiers, binding.vk) {
            log::info!(
                "Registered hotkey id={}: {} -> {}",
                binding.id,
                keybinding_entries
                    .get((binding.id - 1) as usize)
                    .map(|e| e.key.as_str())
                    .unwrap_or("?"),
                binding.command
            );
        }
    }

    // Add existing visible windows
    let visible_windows = windows_api::window::get_visible_windows();
    log::info!("Found {} visible windows", visible_windows.len());
    for hwnd in visible_windows {
        wm_state.add_window(hwnd);
    }

    // Apply initial layout
    wm_state.apply_layout();
    log::info!("Initial layout applied");

    // Win32 message loop
    log::info!("Entering message loop");
    unsafe {
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 <= 0 {
                // 0 = WM_QUIT, negative = error
                break;
            }

            if msg.message == WM_HOTKEY {
                let hotkey_id = msg.wParam.0 as i32;
                if let Some(binding) = hotkey_bindings.iter().find(|b| b.id == hotkey_id) {
                    log::info!("Hotkey pressed: {}", binding.command);
                    if let Some(cmd) = parse_command(&binding.command) {
                        execute_command(&mut wm_state, cmd);
                    }
                }
            } else {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Drain event queue and process window events
            let events = windows_api::event_hook::drain_events();
            for event in events {
                match event {
                    windows_api::event_hook::WindowEvent::Show(hwnd) => {
                        if windows_api::window::is_manageable(hwnd)
                            && wm_state.find_workspace_for_window(hwnd).is_none()
                        {
                            wm_state.add_window(hwnd);
                            wm_state.apply_layout();
                        }
                    }
                    windows_api::event_hook::WindowEvent::Destroy(hwnd) => {
                        if wm_state.find_workspace_for_window(hwnd).is_some() {
                            wm_state.remove_window(hwnd);
                            wm_state.apply_layout();
                        }
                    }
                    windows_api::event_hook::WindowEvent::FocusChange(hwnd) => {
                        if wm_state.find_workspace_for_window(hwnd).is_some() {
                            wm_state.focused_window = Some(hwnd);
                        }
                    }
                }
            }
        }
    }

    // Unregister hotkeys on exit
    for binding in &hotkey_bindings {
        windows_api::hotkey::unregister_hotkey(binding.id);
    }
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
