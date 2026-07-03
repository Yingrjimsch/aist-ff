#[derive(Debug)]
pub enum ModelExtractionError {
    InvalidJson(serde_json::Error),
    MissingModel,
    UnsupportedContentType,
}

impl std::fmt::Display for ModelExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(err) => write!(f, "request body is not valid JSON: {err}"),
            Self::MissingModel => {
                write!(f, "request body does not contain a top-level 'model' field")
            }
            Self::UnsupportedContentType => write!(
                f,
                "request content type is not supported for model extraction"
            ),
        }
    }
}

impl std::error::Error for ModelExtractionError {}

impl From<serde_json::Error> for ModelExtractionError {
    fn from(err: serde_json::Error) -> Self {
        Self::InvalidJson(err)
    }
}
