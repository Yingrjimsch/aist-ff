use crate::secrets_loaders::{
    traits::SecretLoader,
    types::{SecretLoadError, SecretRef},
};

pub struct CompositeSecretLoader {
    loaders: Vec<Box<dyn SecretLoader>>,
}

impl CompositeSecretLoader {
    pub fn new(loaders: Vec<Box<dyn SecretLoader>>) -> Self {
        Self { loaders }
    }
}

impl SecretLoader for CompositeSecretLoader {
    fn supports(&self, secret: &SecretRef) -> bool {
        self.loaders.iter().any(|loader| loader.supports(secret))
    }

    fn load(&self, secret: &SecretRef) -> Result<String, SecretLoadError> {
        let loader = self
            .loaders
            .iter()
            .find(|loader| loader.supports(secret))
            .ok_or(SecretLoadError::UnsupportedSecretSource)?;

        loader.load(secret)
    }
}
