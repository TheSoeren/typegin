use serde::Deserialize;

/// The top-level shape of a `globals.yaml` file.
#[derive(Debug, Deserialize)]
pub(crate) struct GlobalFile {
    #[serde(default)]
    pub(crate) flags: Vec<String>,
}
