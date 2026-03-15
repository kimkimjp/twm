/// Modifier flags matching windows_api::hotkey constants
pub const MOD_FLAG_ALT: u8 = 0x01;
pub const MOD_FLAG_SHIFT: u8 = 0x02;
pub const MOD_FLAG_CTRL: u8 = 0x04;

// VK constants
const VK_RETURN: u32 = 0x0D;
const VK_ESCAPE: u32 = 0x1B;
const VK_SPACE: u32 = 0x20;
const VK_LEFT: u32 = 0x25;
const VK_UP: u32 = 0x26;
const VK_RIGHT: u32 = 0x27;
const VK_DOWN: u32 = 0x28;

/// Parses a key string like "Mod+H", "Mod+Shift+Q" into (modifiers, vk).
///
/// "Mod" maps to Alt by default.
/// Returns None if the key string cannot be parsed.
pub fn parse_key_string(key: &str) -> Option<(u8, u32)> {
    let tokens: Vec<&str> = key.split('+').collect();
    if tokens.is_empty() {
        return None;
    }

    let mut modifiers: u8 = 0;
    for &token in &tokens[..tokens.len() - 1] {
        match token.trim() {
            "Mod" => modifiers |= MOD_FLAG_ALT,
            "Shift" => modifiers |= MOD_FLAG_SHIFT,
            "Ctrl" | "Control" => modifiers |= MOD_FLAG_CTRL,
            _ => {
                log::warn!("Unknown modifier: {}", token);
                return None;
            }
        }
    }

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
        "Comma" => Some(0xBC),  // VK_OEM_COMMA
        "Period" => Some(0xBE), // VK_OEM_PERIOD
        _ => {
            log::warn!("Unknown key: {}", key);
            None
        }
    }
}
