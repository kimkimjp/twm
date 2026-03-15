use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub gaps: GapConfig,
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
}

#[derive(Debug, Deserialize)]
pub struct GapConfig {
    #[serde(default = "default_inner_gap")]
    pub inner: i32,
    #[serde(default = "default_outer_gap")]
    pub outer: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WindowRule {
    pub class: Option<String>,
    pub title: Option<String>,
    pub command: String,
}

fn default_inner_gap() -> i32 {
    5
}
fn default_outer_gap() -> i32 {
    10
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
            gaps: GapConfig::default(),
            window_rules: Vec::new(),
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("twm").join("config.yaml")
}

pub fn load_config() -> Config {
    let path = config_path();

    if !path.exists() {
        log::info!("Config file not found at {}, using defaults", path.display());
        return Config::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_yaml::from_str::<Config>(&contents) {
            Ok(config) => {
                log::info!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                log::warn!("Failed to parse config {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(e) => {
            log::warn!("Failed to read config {}: {}", path.display(), e);
            Config::default()
        }
    }
}
