use crate::secrets_loaders::{
    traits::SecretLoader,
    types::{SecretLoadError, SecretRef},
};

pub struct EnvSecretLoader;

impl SecretLoader for EnvSecretLoader {
    fn supports(&self, secret: &SecretRef) -> bool {
        matches!(secret, SecretRef::Env { .. })
    }

    fn load(&self, secret: &SecretRef) -> Result<String, SecretLoadError> {
        match secret {
            SecretRef::Env { name } => {
                std::env::var(name).map_err(|_| SecretLoadError::MissingEnvVar(name.clone()))
            }
            _ => Err(SecretLoadError::UnsupportedSecretSource),
        }
    }
}
