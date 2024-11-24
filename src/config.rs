use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_command")]
    pub open_command: String,
}

fn default_command() -> String {
    "cursor".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            open_command: default_command(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = dirs::home_dir()
            .map(|mut path| {
                path.push(".glancr.yml");
                path
            })
            .unwrap_or_else(|| PathBuf::from(".glancr.yml"));

        if let Ok(contents) = std::fs::read_to_string(config_path) {
            serde_yaml::from_str(&contents).unwrap_or_default()
        } else {
            Config::default()
        }
    }
}
