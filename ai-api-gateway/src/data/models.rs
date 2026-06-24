use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Model {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProviderModelMapping {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TomlFileStructure {
    pub providers: Vec<Provider>,
    pub models: Vec<Model>,
    pub provider_models: Vec<ProviderModelMapping>,
}
