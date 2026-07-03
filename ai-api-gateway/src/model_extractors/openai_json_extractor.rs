use axum::Error;
use serde::Deserialize;

use crate::{
    model_extractors::{traits::ModelExtractor, types::ModelExtractionError},
    requests::request_context::RequestContext,
};

pub struct OpenAiJsonExtractor;

#[derive(Deserialize)]
struct ModelField {
    model: Option<String>,
}

impl ModelExtractor for OpenAiJsonExtractor {
    // fn id(&self) -> &'static str {
    //     "openai_json"
    // }

    // fn matches(&self, ctx: &RequestContext) -> bool {
    //     matches!(ctx.content_type, ContentType::Json)
    //         && matches!(
    //             ctx.endpoint_kind,
    //             EndpointKind::ChatCompletions
    //                 | EndpointKind::Responses
    //                 | EndpointKind::Embeddings
    //                 | EndpointKind::Images
    //         )
    // }

    fn extract(&self, ctx: &RequestContext) -> Result<Option<String>, ModelExtractionError> {
        let model: ModelField = serde_json::from_slice(&ctx.body)?;
        Ok(model.model)
    }
}
