use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct GlobalFile {
    #[serde(default)]
    pub(crate) flags: Vec<String>,
}
