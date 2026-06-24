use crate::config::{AppConfig, ConfigLoader};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

pub struct TomlConfigLoader {
    file_path: PathBuf,
}

impl TomlConfigLoader {
    pub fn new(path: &str) -> Self {
        Self {
            file_path: PathBuf::from(path),
        }
    }
}

// Implementing the abstract trait
impl ConfigLoader for TomlConfigLoader {
    fn load(&self) -> Result<AppConfig, Box<dyn Error>> {
        let content = fs::read_to_string(&self.file_path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
