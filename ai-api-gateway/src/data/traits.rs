use crate::data::models::{Model, Provider};
use std::error::Error;

pub type DataResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[async_trait::async_trait]
pub trait DataRepository: Send + Sync {
    async fn get_provider(&self, id: &str) -> DataResult<Option<Provider>>;
    async fn get_model(&self, id: &str) -> DataResult<Option<Model>>;

    async fn get_models_for_provider(&self, provider_id: &str) -> DataResult<Vec<Model>>;
    async fn get_providers_for_model(&self, model_id: &str) -> DataResult<Vec<Provider>>;
}
