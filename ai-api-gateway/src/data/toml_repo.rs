use crate::data::models::{Model, Provider, TomlFileStructure};
use crate::data::traits::{DataRepository, DataResult};
use std::fs; // TomlFileStructure matches your flat TOML file layout

pub struct TomlRepository {
    cache: TomlFileStructure,
}

impl TomlRepository {
    pub fn new(path: &str) -> DataResult<Self> {
        let content = fs::read_to_string(path)?;
        let cache: TomlFileStructure = toml::from_str(&content)?;
        Ok(Self { cache })
    }
}

#[async_trait::async_trait]
impl DataRepository for TomlRepository {
    async fn get_provider(&self, id: &str) -> DataResult<Option<Provider>> {
        let provider: Option<Provider> = self.cache.providers.iter().find(|p| p.id == id).cloned();
        Ok(provider)
    }

    async fn get_model(&self, id: &str) -> DataResult<Option<Model>> {
        let model: Option<Model> = self.cache.models.iter().find(|m| m.id == id).cloned();
        Ok(model)
    }

    async fn get_models_for_provider(&self, provider_id: &str) -> DataResult<Vec<Model>> {
        // Find matching model IDs from the intermediate mappings
        let model_ids: Vec<&str> = self
            .cache
            .provider_models
            .iter()
            .filter(|pm: &&super::models::ProviderModelMapping| pm.provider_id == provider_id)
            .map(|pm: &super::models::ProviderModelMapping| pm.model_id.as_str())
            .collect();

        // Filter and return the full model objects
        let models: Vec<Model> = self
            .cache
            .models
            .iter()
            .filter(|m: &&Model| model_ids.contains(&m.id.as_str()))
            .cloned()
            .collect();

        Ok(models)
    }

    async fn get_providers_for_model(&self, model_id: &str) -> DataResult<Vec<Provider>> {
        let provider_ids: Vec<&str> = self
            .cache
            .provider_models
            .iter()
            .filter(|pm| pm.model_id == model_id)
            .map(|pm| pm.provider_id.as_str())
            .collect();

        let providers: Vec<Provider> = self
            .cache
            .providers
            .iter()
            .filter(|p: &&Provider| provider_ids.contains(&p.id.as_str()))
            .cloned()
            .collect();

        Ok(providers)
    }
}
