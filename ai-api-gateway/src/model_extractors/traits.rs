use axum::Error;
use serde::Deserialize;

use crate::{
    model_extractors::types::ModelExtractionError, requests::request_context::RequestContext,
};

pub trait ModelExtractor: Send + Sync {
    // fn id(&self) -> &'static str;
    // fn matches(&self, ctx: &RequestContext) -> bool;
    fn extract(&self, ctx: &RequestContext) -> Result<Option<String>, ModelExtractionError>;
}
