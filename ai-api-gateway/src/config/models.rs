use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Provider {
    pub id: String, // Added ID for referencing
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Model {
    pub name: String,
    pub provider_ids: Vec<String>, // Maps the N:N relationship
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub providers: Vec<Provider>,
    pub models: Vec<Model>,
}
