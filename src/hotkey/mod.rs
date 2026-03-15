#[cfg(target_os = "windows")]
pub use crate::windows_api::hotkey::*;

#[cfg(not(target_os = "windows"))]
pub mod stub {
    pub const MOD_FLAG_ALT: u8 = 0x01;
    pub const MOD_FLAG_SHIFT: u8 = 0x02;
    pub const MOD_FLAG_CTRL: u8 = 0x04;

    #[derive(Debug, Clone)]
    pub struct KeyBinding {
        pub modifiers: u8,
        pub vk: u32,
        pub command: String,
    }
}

#[cfg(not(target_os = "windows"))]
pub use stub::*;
