use crate::secrets_loaders::{
    traits::SecretLoader,
    types::{SecretLoadError, SecretRef},
};

pub struct FileSecretLoader;

impl SecretLoader for FileSecretLoader {
    fn supports(&self, secret: &SecretRef) -> bool {
        matches!(secret, SecretRef::File { .. })
    }

    fn load(&self, secret: &SecretRef) -> Result<String, SecretLoadError> {
        match secret {
            SecretRef::File { path } => {
                let secret =
                    std::fs::read_to_string(path).map_err(|err| SecretLoadError::ReadFile {
                        path: path.clone(),
                        source: err,
                    })?;

                Ok(secret.trim_end_matches(['\r', '\n']).to_string())
            }
            _ => Err(SecretLoadError::UnsupportedSecretSource),
        }
    }
}
