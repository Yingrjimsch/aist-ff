pub mod models;
mod toml_loader;

pub use models::AppConfig;
use std::error::Error;
pub use toml_loader::TomlConfigLoader;

// The abstract trait
pub trait ConfigLoader {
    fn load(&self) -> Result<AppConfig, Box<dyn Error>>;
}
