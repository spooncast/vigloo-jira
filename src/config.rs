use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_jira")]
    pub jira: JiraConfig,
}

#[derive(Debug, Deserialize)]
pub struct JiraConfig {
    #[serde(default = "default_board_id")]
    pub board_id: u64,
    #[serde(default = "default_host")]
    pub host: String,
}

fn default_board_id() -> u64 {
    272
}

fn default_host() -> String {
    "https://spoonradio.atlassian.net".to_string()
}

fn default_jira() -> JiraConfig {
    JiraConfig {
        board_id: default_board_id(),
        host: default_host(),
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            jira: default_jira(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vigloo-jira")
            .join("config.toml")
    }
}
