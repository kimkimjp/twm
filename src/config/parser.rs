use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub keybindings: Vec<KeybindingEntry>,
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
}

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_mod_key")]
    pub mod_key: String,
    #[serde(default)]
    pub gaps: GapConfig,
    #[serde(default = "default_terminal")]
    pub terminal: String,
}

#[derive(Debug, Deserialize)]
pub struct GapConfig {
    #[serde(default = "default_inner_gap")]
    pub inner: i32,
    #[serde(default = "default_outer_gap")]
    pub outer: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KeybindingEntry {
    pub key: String,
    pub command: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowRule {
    pub class: Option<String>,
    pub title: Option<String>,
    pub command: String,
}

fn default_mod_key() -> String {
    "Alt".to_string()
}
fn default_inner_gap() -> i32 {
    5
}
fn default_outer_gap() -> i32 {
    10
}
fn default_terminal() -> String {
    "wt.exe".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            mod_key: default_mod_key(),
            gaps: GapConfig::default(),
            terminal: default_terminal(),
        }
    }
}

impl Default for GapConfig {
    fn default() -> Self {
        GapConfig {
            inner: default_inner_gap(),
            outer: default_outer_gap(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: GeneralConfig::default(),
            keybindings: Vec::new(),
            window_rules: Vec::new(),
        }
    }
}

/// Returns the path to the config file: ~/.config/twm/config.yaml
fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("twm").join("config.yaml")
}

/// Loads the configuration from ~/.config/twm/config.yaml.
/// Returns the default configuration if the file does not exist or cannot be parsed.
pub fn load_config() -> Config {
    let path = config_path();

    if !path.exists() {
        log::info!(
            "Config file not found at {}, using defaults",
            path.display()
        );
        return Config::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_yaml::from_str::<Config>(&contents) {
            Ok(config) => {
                log::info!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                log::warn!("Failed to parse config file {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(e) => {
            log::warn!("Failed to read config file {}: {}", path.display(), e);
            Config::default()
        }
    }
}

/// Returns the default i3-compatible keybindings.
pub fn get_default_keybindings() -> Vec<KeybindingEntry> {
    let mut bindings = Vec::new();

    // Focus
    bindings.push(kb("Mod+H", "focus left"));
    bindings.push(kb("Mod+J", "focus down"));
    bindings.push(kb("Mod+K", "focus up"));
    bindings.push(kb("Mod+L", "focus right"));

    // Move
    bindings.push(kb("Mod+Shift+H", "move left"));
    bindings.push(kb("Mod+Shift+J", "move down"));
    bindings.push(kb("Mod+Shift+K", "move up"));
    bindings.push(kb("Mod+Shift+L", "move right"));

    // Workspaces 1-9
    for i in 1..=9 {
        bindings.push(kb(
            &format!("Mod+{}", i),
            &format!("workspace {}", i),
        ));
    }

    // Move to workspaces 1-9
    for i in 1..=9 {
        bindings.push(kb(
            &format!("Mod+Shift+{}", i),
            &format!("move_to_workspace {}", i),
        ));
    }

    // Actions
    bindings.push(kb("Mod+Shift+Q", "close"));
    bindings.push(kb("Mod+F", "fullscreen"));
    bindings.push(kb("Mod+V", "split_toggle"));
    bindings.push(kb("Mod+Return", "exec wt.exe"));
    bindings.push(kb("Mod+Shift+E", "exit"));
    bindings.push(kb("Mod+Shift+C", "reload"));

    // Monitor focus/move
    bindings.push(kb("Mod+Comma", "focus_monitor prev"));
    bindings.push(kb("Mod+Period", "focus_monitor next"));
    bindings.push(kb("Mod+Shift+Comma", "move_to_monitor prev"));
    bindings.push(kb("Mod+Shift+Period", "move_to_monitor next"));

    bindings
}

fn kb(key: &str, command: &str) -> KeybindingEntry {
    KeybindingEntry {
        key: key.to_string(),
        command: command.to_string(),
    }
}
