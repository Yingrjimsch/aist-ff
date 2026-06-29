use axum::{body::Bytes, http::HeaderMap};
use reqwest::Method;

pub struct RequestContext {
    pub method: Method,
    pub path: String,
    pub headers: HeaderMap,
    pub body: Bytes,
    // pub content_type: ContentType,
    // pub endpoint_kind: EndpointKind,
}

impl RequestContext {
    pub fn new(method: Method, path: String, headers: HeaderMap, body: Bytes) -> Self {
        // let content_type = ContentType::from_headers(&headers);
        // let endpoint_kind = EndpointKind::from_path(&path);

        Self {
            method,
            path,
            headers,
            body,
            // content_type,
            // endpoint_kind,
        }
    }
}
