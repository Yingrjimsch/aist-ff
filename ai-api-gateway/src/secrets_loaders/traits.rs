use crate::secrets_loaders::types::{SecretLoadError, SecretRef};

pub trait SecretLoader: Send + Sync {
    fn supports(&self, secret: &SecretRef) -> bool;
    fn load(&self, secret: &SecretRef) -> Result<String, SecretLoadError>;
}
