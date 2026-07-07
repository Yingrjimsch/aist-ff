use axum::http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::header::AUTHORIZATION;

use crate::{
    data::models::Provider,
    secrets_loaders::{
        traits::SecretLoader,
        types::{AuthError, ProviderAuthMethod},
    },
};

const BEARER_PREFIX: &str = "Bearer ";

pub fn apply_auth(
    headers: &mut HeaderMap,
    auth: &ProviderAuthMethod,
    secret_loader: &dyn SecretLoader,
) -> Result<(), AuthError> {
    match auth {
        ProviderAuthMethod::None => Ok(()),
        ProviderAuthMethod::Header { name, value } => {
            let value = secret_loader.load(value)?;
            headers.insert(name.parse::<HeaderName>()?, value.parse::<HeaderValue>()?);
            Ok(())
        }
        ProviderAuthMethod::BearerSecret { secret } => {
            let value = secret_loader.load(secret)?;
            let bearer_header_value = format!("{BEARER_PREFIX}{value}").parse::<HeaderValue>()?;
            headers.insert(AUTHORIZATION, bearer_header_value);
            Ok(())
        }
    }
}
