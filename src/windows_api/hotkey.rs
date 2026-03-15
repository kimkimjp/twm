use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
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

// ============================================================
// Lock-free global state — NO Mutex in the hook callback path
// ============================================================

/// Bindings are set once at startup and never modified. OnceLock is lock-free for reads.
static BINDINGS: OnceLock<Vec<KeyBinding>> = OnceLock::new();

/// Current modifier state (packed bitmask: bit 0=Alt, 1=Shift, 2=Ctrl)
static MOD_STATE: AtomicU8 = AtomicU8::new(0);

/// Whether Alt was used in a consumed combo (prevents menu activation on Alt release)
static ALT_USED_FOR_COMBO: AtomicBool = AtomicBool::new(false);

/// Lock-free command ring buffer: stores binding INDICES (not strings, to avoid allocation)
const QUEUE_SIZE: usize = 64;
const QUEUE_EMPTY: u32 = u32::MAX;
static COMMAND_QUEUE: [AtomicU32; QUEUE_SIZE] = {
    const INIT: AtomicU32 = AtomicU32::new(u32::MAX);
    [INIT; QUEUE_SIZE]
};
static QUEUE_WRITE: AtomicUsize = AtomicUsize::new(0);
static QUEUE_READ: AtomicUsize = AtomicUsize::new(0);

/// Hook handle stored as raw isize (avoids Send/Sync issues)
static HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);

/// Thread ID for PostThreadMessageW wake-up
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

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
    let thread_id = unsafe { GetCurrentThreadId() };
    HOOK_THREAD_ID.store(thread_id, Ordering::SeqCst);

    // Store bindings (only first call succeeds; OnceLock is immutable after init)
    let _ = BINDINGS.set(bindings);

    do_install_hook();
}

fn do_install_hook() {
    // Uninstall existing hook first if any
    let old = HOOK_HANDLE.swap(0, Ordering::SeqCst);
    if old != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(old as *mut _));
        }
    }

    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) };

    match hook {
        Ok(h) => {
            HOOK_HANDLE.store(h.0 as usize, Ordering::SeqCst);
            log::info!("Low-level keyboard hook installed");
        }
        Err(e) => {
            log::warn!("Failed to install keyboard hook: {}", e);
        }
    }
}

/// Uninstall the keyboard hook.
pub fn uninstall_hook() {
    let handle = HOOK_HANDLE.swap(0, Ordering::SeqCst);
    if handle != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(handle as *mut _));
        }
        log::info!("Keyboard hook uninstalled");
    }
}

/// Reinstall the hook. Call this periodically (e.g. every 5 seconds) from the
/// message loop to recover from Windows silently removing the hook due to timeout.
///
/// Windows removes WH_KEYBOARD_LL hooks if the callback takes too long to return
/// (default: LowLevelHooksTimeout = 300ms). With our lock-free design this shouldn't
/// happen, but we reinstall as a safety net.
pub fn ensure_hook_alive() {
    let current = HOOK_HANDLE.load(Ordering::SeqCst);
    if current == 0 {
        log::warn!("Hook handle is null, reinstalling...");
        do_install_hook();
        return;
    }

    // Proactively reinstall to be safe — this is cheap (~1μs)
    do_install_hook();
}

/// Drain pending commands from the lock-free ring buffer.
/// Returns command strings by looking up binding indices.
pub fn drain_commands() -> Vec<String> {
    let Some(bindings) = BINDINGS.get() else {
        return Vec::new();
    };

    let mut cmds = Vec::new();
    let write = QUEUE_WRITE.load(Ordering::Acquire);
    let mut read = QUEUE_READ.load(Ordering::Acquire);

    while read < write {
        let slot = read % QUEUE_SIZE;
        let idx = COMMAND_QUEUE[slot].swap(QUEUE_EMPTY, Ordering::AcqRel);
        if idx != QUEUE_EMPTY {
            if let Some(b) = bindings.get(idx as usize) {
                cmds.push(b.command.clone());
            }
        }
        read += 1;
    }

    QUEUE_READ.store(read, Ordering::Release);
    cmds
}

/// Wake up the main message loop by posting a WM_APP message.
#[inline]
fn wake_message_loop() {
    let tid = HOOK_THREAD_ID.load(Ordering::Relaxed);
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_APP, WPARAM(0), LPARAM(0));
        }
    }
}

/// The low-level keyboard hook callback.
///
/// CRITICAL: This function must return as fast as possible (<1ms).
/// If it takes longer than LowLevelHooksTimeout (default 300ms), Windows
/// will silently remove the hook.
///
/// Design: ZERO locks, ZERO allocations. Only atomic operations.
unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let kb = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
    let vk = kb.vkCode;
    let msg = w_param.0 as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    // --- Handle modifier keys ---
    match vk {
        VK_LMENU | VK_RMENU | VK_MENU => {
            if is_down {
                MOD_STATE.fetch_or(MOD_FLAG_ALT, Ordering::SeqCst);
            } else if is_up {
                MOD_STATE.fetch_and(!MOD_FLAG_ALT, Ordering::SeqCst);
                // Consume Alt release if it was used for a combo
                if ALT_USED_FOR_COMBO.swap(false, Ordering::SeqCst) {
                    return LRESULT(1);
                }
            }
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
        VK_LSHIFT | VK_RSHIFT | VK_SHIFT => {
            if is_down {
                MOD_STATE.fetch_or(MOD_FLAG_SHIFT, Ordering::SeqCst);
            } else if is_up {
                MOD_STATE.fetch_and(!MOD_FLAG_SHIFT, Ordering::SeqCst);
            }
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
        VK_LCONTROL | VK_RCONTROL | VK_CONTROL => {
            if is_down {
                MOD_STATE.fetch_or(MOD_FLAG_CTRL, Ordering::SeqCst);
            } else if is_up {
                MOD_STATE.fetch_and(!MOD_FLAG_CTRL, Ordering::SeqCst);
            }
            return CallNextHookEx(None, n_code, w_param, l_param);
        }
        _ => {}
    }

    // --- Non-modifier key: only process key-down ---
    if !is_down {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let current_mods = MOD_STATE.load(Ordering::SeqCst);

    // No modifiers = normal typing, fast path
    if current_mods == 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    // Check bindings (read-only, no lock needed)
    let Some(bindings) = BINDINGS.get() else {
        return CallNextHookEx(None, n_code, w_param, l_param);
    };

    if let Some(idx) = bindings.iter().position(|b| b.vk == vk && b.modifiers == current_mods) {
        // Mark Alt as used for combo
        if current_mods & MOD_FLAG_ALT != 0 {
            ALT_USED_FOR_COMBO.store(true, Ordering::SeqCst);
        }

        // Push binding index to lock-free ring buffer
        let write = QUEUE_WRITE.fetch_add(1, Ordering::SeqCst);
        COMMAND_QUEUE[write % QUEUE_SIZE].store(idx as u32, Ordering::SeqCst);

        // Wake up the message loop
        wake_message_loop();

        // Consume the key
        return LRESULT(1);
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}
