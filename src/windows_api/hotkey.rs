use std::sync::Mutex;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// Modifier flags (bitmask)
pub const MOD_FLAG_ALT: u8 = 0x01;
pub const MOD_FLAG_SHIFT: u8 = 0x02;
pub const MOD_FLAG_CTRL: u8 = 0x04;

/// A key binding: modifiers + virtual key code -> command string
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub modifiers: u8,
    pub vk: u32,
    pub command: String,
}

/// Global state for the keyboard hook
struct HookState {
    bindings: Vec<KeyBinding>,
    pending_commands: Vec<String>,
    // Track which modifiers are currently held
    alt_down: bool,
    shift_down: bool,
    ctrl_down: bool,
}

// SAFETY: HookState contains only plain data types (Vec, String, bool).
// It is protected by a Mutex and only accessed from the hook callback
// (which runs on the thread that installed the hook) and from drain_commands.
unsafe impl Send for HookState {}
unsafe impl Sync for HookState {}

static HOOK_STATE: Mutex<HookState> = Mutex::new(HookState {
    bindings: Vec::new(),
    pending_commands: Vec::new(),
    alt_down: false,
    shift_down: false,
    ctrl_down: false,
});

// Store the hook handle as a raw isize to avoid Send/Sync issues with HHOOK.
static HOOK_HANDLE: Mutex<isize> = Mutex::new(0);

/// VK codes for modifier keys
const VK_LMENU: u32 = 0xA4;
const VK_RMENU: u32 = 0xA5;
const VK_MENU: u32 = 0x12;
const VK_LSHIFT: u32 = 0xA0;
const VK_RSHIFT: u32 = 0xA1;
const VK_SHIFT: u32 = 0x10;
const VK_LCONTROL: u32 = 0xA2;
const VK_RCONTROL: u32 = 0xA3;
const VK_CONTROL: u32 = 0x11;

/// Install the low-level keyboard hook with the given bindings.
pub fn install_hook(bindings: Vec<KeyBinding>) {
    // Store bindings
    if let Ok(mut state) = HOOK_STATE.lock() {
        state.bindings = bindings;
    }

    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) };

    match hook {
        Ok(h) => {
            if let Ok(mut handle) = HOOK_HANDLE.lock() {
                *handle = h.0 as isize;
            }
            log::info!("Low-level keyboard hook installed");
        }
        Err(e) => {
            log::warn!("Failed to install keyboard hook: {}", e);
        }
    }
}

/// Uninstall the keyboard hook.
pub fn uninstall_hook() {
    if let Ok(mut handle) = HOOK_HANDLE.lock() {
        if *handle != 0 {
            let hhook = HHOOK(*handle as *mut _);
            unsafe {
                let _ = UnhookWindowsHookEx(hhook);
            }
            *handle = 0;
            log::info!("Keyboard hook uninstalled");
        }
    }
}

/// Drain pending commands from the hook callback.
pub fn drain_commands() -> Vec<String> {
    if let Ok(mut state) = HOOK_STATE.lock() {
        state.pending_commands.drain(..).collect()
    } else {
        Vec::new()
    }
}

fn is_modifier_key(vk: u32) -> bool {
    matches!(
        vk,
        VK_LMENU
            | VK_RMENU
            | VK_MENU
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_SHIFT
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_CONTROL
    )
}

/// The low-level keyboard hook callback.
unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    // HC_ACTION = 0
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let kb = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
    let vk = kb.vkCode;
    let msg = w_param.0 as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    if let Ok(mut state) = HOOK_STATE.lock() {
        // Update modifier state
        match vk {
            VK_LMENU | VK_RMENU | VK_MENU => {
                if is_down {
                    state.alt_down = true;
                } else if is_up {
                    state.alt_down = false;
                }
            }
            VK_LSHIFT | VK_RSHIFT | VK_SHIFT => {
                if is_down {
                    state.shift_down = true;
                } else if is_up {
                    state.shift_down = false;
                }
            }
            VK_LCONTROL | VK_RCONTROL | VK_CONTROL => {
                if is_down {
                    state.ctrl_down = true;
                } else if is_up {
                    state.ctrl_down = false;
                }
            }
            _ => {}
        }

        // Only process key-down events for non-modifier keys
        if is_down && !is_modifier_key(vk) {
            // Build current modifier state
            let mut current_mods: u8 = 0;
            if state.alt_down {
                current_mods |= MOD_FLAG_ALT;
            }
            if state.shift_down {
                current_mods |= MOD_FLAG_SHIFT;
            }
            if state.ctrl_down {
                current_mods |= MOD_FLAG_CTRL;
            }

            // Check against bindings — clone command to avoid borrow conflict
            let matched_command = state
                .bindings
                .iter()
                .find(|b| b.vk == vk && b.modifiers == current_mods)
                .map(|b| b.command.clone());

            if let Some(cmd) = matched_command {
                state.pending_commands.push(cmd);
                // Consume the key event - don't pass to other apps
                return LRESULT(1);
            }
        }
    }

    // Pass unmatched keys through
    CallNextHookEx(None, n_code, w_param, l_param)
}
