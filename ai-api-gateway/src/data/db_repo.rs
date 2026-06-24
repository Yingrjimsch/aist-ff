use crate::data::models::{Model, Provider};
use crate::data::traits::{DataRepository, DataResult};
use sqlx::PgPool;

pub struct DbRepository {
    pool: PgPool,
}

impl DbRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl DataRepository for DbRepository {
    async fn get_provider(&self, id: &str) -> DataResult<Option<Provider>> {
        let provider = sqlx::query_as::<_, Provider>(
            "SELECT id, name, url FROM providers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(provider)
    }

    async fn get_model(&self, id: &str) -> DataResult<Option<Model>> {
        let model = sqlx::query_as::<_, Model>("SELECT id, name FROM models WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(model)
    }

    async fn get_models_for_provider(&self, provider_id: &str) -> DataResult<Vec<Model>> {
        let models = sqlx::query_as::<_, Model>(
            "SELECT m.id, m.name FROM models m
             JOIN provider_models pm ON m.id = pm.model_id 
             WHERE pm.provider_id = $1",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(models)
    }

    async fn get_providers_for_model(&self, model_id: &str) -> DataResult<Vec<Provider>> {
        let providers = sqlx::query_as::<_, Provider>(
            "SELECT p.id, p.name, p.url FROM providers p
             JOIN provider_models pm ON p.id = pm.provider_id
             WHERE pm.model_id = $1",
        )
        .bind(model_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(providers)
    }
}
