// use crate::{model_extractors::traits::ModelExtractor, requests::request_context::RequestContext};

// pub struct OpenAiJsonExtractor;

// impl ModelExtractor for OpenAiJsonExtractor {
//     fn id(&self) -> &'static str {
//         "openai_json"
//     }

//     // fn matches(&self, ctx: &RequestContext) -> bool {
//     //     matches!(ctx.content_type, ContentType::Json)
//     //         && matches!(
//     //             ctx.endpoint_kind,
//     //             EndpointKind::ChatCompletions
//     //                 | EndpointKind::Responses
//     //                 | EndpointKind::Embeddings
//     //                 | EndpointKind::Images
//     //         )
//     // }

//     fn extract(&self, ctx: &mut RequestContext) -> Result<ModelExtraction, ModelExtractionError> {
//         let value: serde_json::Value = serde_json::from_slice(&ctx.body)?;

//         let model = value
//             .get("model")
//             .and_then(|v| v.as_str())
//             .ok_or(ModelExtractionError::MissingModel)?;

//         Ok(ModelExtraction {
//             identifiers: vec![ModelIdentifier {
//                 raw: model.to_owned(),
//                 kind: ModelIdentifierKind::Model,
//                 provider_hint: None,
//             }],
//             source: ModelSource::JsonField("model"),
//             confidence: Confidence::High,
//         })
//     }
// }
