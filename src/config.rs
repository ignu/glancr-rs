use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_command")]
    pub open_command: String,
    #[serde(default = "default_ignored_dirs")]
    pub ignored_dirs: Vec<String>,
    #[serde(default = "default_ignored_patterns")]
    pub ignored_patterns: Vec<String>,
}

fn default_command() -> String {
    "cursor".to_string()
}

fn default_ignored_dirs() -> Vec<String> {
    vec![
        "/.git/".to_string(),
        "/node_modules/".to_string(),
        "/target/".to_string(),
        "/dist/".to_string(),
        "/build/".to_string(),
        "/.idea/".to_string(),
        "/.vscode/".to_string(),
        "/vendor/".to_string(),
        "/.next/".to_string(),
        "/coverage/".to_string(),
        "/yarn.lock".to_string(),
        "/.yarn/".to_string(),
    ]
}

fn default_ignored_patterns() -> Vec<String> {
    vec![
        ".lock".to_string(),
        ".log".to_string(),
        ".map".to_string(),
        ".min.js".to_string(),
        ".min.css".to_string(),
        ".bundle.".to_string(),
        ".cache".to_string(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            open_command: default_command(),
            ignored_dirs: default_ignored_dirs(),
            ignored_patterns: default_ignored_patterns(),
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
