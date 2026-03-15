use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
};

/// Registers a system-wide hotkey.
///
/// Returns `true` if the hotkey was registered successfully.
pub fn register_hotkey(id: i32, modifiers: HOT_KEY_MODIFIERS, vk: u32) -> bool {
    unsafe {
        let result = RegisterHotKey(None, id, modifiers, vk);
        if let Err(ref e) = result {
            log::warn!("RegisterHotKey(id={}) failed: {}", id, e);
        }
        result.is_ok()
    }
}

/// Unregisters a previously registered hotkey.
pub fn unregister_hotkey(id: i32) {
    unsafe {
        if let Err(e) = UnregisterHotKey(None, id) {
            log::warn!("UnregisterHotKey(id={}) failed: {}", id, e);
        }
    }
}
