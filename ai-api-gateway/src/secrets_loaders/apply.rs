use axum::http::{HeaderMap, HeaderName, HeaderValue};

use crate::{
    data::models::Provider,
    secrets_loaders::{
        traits::SecretLoader,
        types::{AuthError, ProviderAuthMethod},
    },
};

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
        // TODO: not implemented yet
        ProviderAuthMethod::BearerToken { token } => Ok(()),
    }
}
