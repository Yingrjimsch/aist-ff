use serde::Deserialize;

#[derive(Debug)]
pub enum SecretLoadError {
    MissingEnvVar(String),
    VaultError(String),
    UnsupportedSecretSource,
    ReadFile {
        path: String,
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub enum AuthError {
    SecretLoad(SecretLoadError),
    InvalidHeaderName(axum::http::header::InvalidHeaderName),
    InvalidHeaderValue(axum::http::header::InvalidHeaderValue),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum SecretRef {
    #[serde(rename = "env")]
    Env { name: String },
    #[serde(rename = "file")]
    File { path: String },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ProviderAuthMethod {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "bearer")]
    BearerToken { token: SecretRef },
    #[serde(rename = "header")]
    Header { name: String, value: SecretRef },
}

impl From<SecretLoadError> for AuthError {
    fn from(err: SecretLoadError) -> Self {
        Self::SecretLoad(err)
    }
}

impl From<axum::http::header::InvalidHeaderName> for AuthError {
    fn from(err: axum::http::header::InvalidHeaderName) -> Self {
        Self::InvalidHeaderName(err)
    }
}

impl From<axum::http::header::InvalidHeaderValue> for AuthError {
    fn from(err: axum::http::header::InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValue(err)
    }
}
