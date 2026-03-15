#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS;

/// Hotkey binding: an id, modifier+key combo, and the command string.
#[derive(Debug, Clone)]
pub struct HotkeyBinding {
    pub id: i32,
    #[cfg(target_os = "windows")]
    pub modifiers: HOT_KEY_MODIFIERS,
    #[cfg(not(target_os = "windows"))]
    pub modifiers: u32,
    pub vk: u32,
    pub command: String,
}

// Windows constants for modifier keys
#[cfg(target_os = "windows")]
mod platform {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
    };

    pub const MODIFIER_ALT: HOT_KEY_MODIFIERS = MOD_ALT;
    pub const MODIFIER_SHIFT: HOT_KEY_MODIFIERS = MOD_SHIFT;
    pub const MODIFIER_CONTROL: HOT_KEY_MODIFIERS = MOD_CONTROL;
    pub const MODIFIER_NOREPEAT: HOT_KEY_MODIFIERS = MOD_NOREPEAT;

    pub fn combine_modifiers(a: HOT_KEY_MODIFIERS, b: HOT_KEY_MODIFIERS) -> HOT_KEY_MODIFIERS {
        HOT_KEY_MODIFIERS(a.0 | b.0)
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    // Stub values matching Windows constants for cross-platform compilation
    pub const MODIFIER_ALT: u32 = 0x0001;
    pub const MODIFIER_SHIFT: u32 = 0x0004;
    pub const MODIFIER_CONTROL: u32 = 0x0002;
    pub const MODIFIER_NOREPEAT: u32 = 0x4000;

    pub fn combine_modifiers(a: u32, b: u32) -> u32 {
        a | b
    }
}

/// Virtual key code constants
const VK_RETURN: u32 = 0x0D;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_LEFT: u32 = 0x25;
const VK_UP: u32 = 0x26;
const VK_RIGHT: u32 = 0x27;
const VK_DOWN: u32 = 0x28;

/// Parses a key string like "Mod+H", "Mod+Shift+Q", "Mod+1" into
/// a (modifiers, virtual_key_code) pair.
///
/// "Mod" maps to Alt by default.
/// Returns None if the key string cannot be parsed.
#[cfg(target_os = "windows")]
pub fn parse_key_string(key: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let tokens: Vec<&str> = key.split('+').collect();
    if tokens.is_empty() {
        return None;
    }

    let mut modifiers = HOT_KEY_MODIFIERS(0);

    // All tokens except the last are modifiers
    for &token in &tokens[..tokens.len() - 1] {
        match token.trim() {
            "Mod" => modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_ALT),
            "Shift" => modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_SHIFT),
            "Ctrl" | "Control" => {
                modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_CONTROL)
            }
            _ => {
                log::warn!("Unknown modifier: {}", token);
                return None;
            }
        }
    }

    // Add MOD_NOREPEAT
    modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_NOREPEAT);

    // Last token is the key
    let key_token = tokens.last()?.trim();
    let vk = parse_vk(key_token)?;

    Some((modifiers, vk))
}

#[cfg(not(target_os = "windows"))]
pub fn parse_key_string(key: &str) -> Option<(u32, u32)> {
    let tokens: Vec<&str> = key.split('+').collect();
    if tokens.is_empty() {
        return None;
    }

    let mut modifiers: u32 = 0;

    for &token in &tokens[..tokens.len() - 1] {
        match token.trim() {
            "Mod" => modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_ALT),
            "Shift" => modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_SHIFT),
            "Ctrl" | "Control" => {
                modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_CONTROL)
            }
            _ => {
                log::warn!("Unknown modifier: {}", token);
                return None;
            }
        }
    }

    modifiers = platform::combine_modifiers(modifiers, platform::MODIFIER_NOREPEAT);

    let key_token = tokens.last()?.trim();
    let vk = parse_vk(key_token)?;

    Some((modifiers, vk))
}

/// Converts a key name to its virtual key code.
fn parse_vk(key: &str) -> Option<u32> {
    match key {
        // Single letters A-Z
        "A" => Some(0x41),
        "B" => Some(0x42),
        "C" => Some(0x43),
        "D" => Some(0x44),
        "E" => Some(0x45),
        "F" => Some(0x46),
        "G" => Some(0x47),
        "H" => Some(0x48),
        "I" => Some(0x49),
        "J" => Some(0x4A),
        "K" => Some(0x4B),
        "L" => Some(0x4C),
        "M" => Some(0x4D),
        "N" => Some(0x4E),
        "O" => Some(0x4F),
        "P" => Some(0x50),
        "Q" => Some(0x51),
        "R" => Some(0x52),
        "S" => Some(0x53),
        "T" => Some(0x54),
        "U" => Some(0x55),
        "V" => Some(0x56),
        "W" => Some(0x57),
        "X" => Some(0x58),
        "Y" => Some(0x59),
        "Z" => Some(0x5A),
        // Digits 0-9
        "0" => Some(0x30),
        "1" => Some(0x31),
        "2" => Some(0x32),
        "3" => Some(0x33),
        "4" => Some(0x34),
        "5" => Some(0x35),
        "6" => Some(0x36),
        "7" => Some(0x37),
        "8" => Some(0x38),
        "9" => Some(0x39),
        // Special keys
        "Return" | "Enter" => Some(VK_RETURN),
        "Escape" | "Esc" => Some(VK_ESCAPE),
        "Space" => Some(VK_SPACE),
        "Left" => Some(VK_LEFT),
        "Up" => Some(VK_UP),
        "Right" => Some(VK_RIGHT),
        "Down" => Some(VK_DOWN),
        "Comma" => Some(0xBC),   // VK_OEM_COMMA
        "Period" => Some(0xBE),  // VK_OEM_PERIOD
        _ => {
            log::warn!("Unknown key: {}", key);
            None
        }
    }
}
