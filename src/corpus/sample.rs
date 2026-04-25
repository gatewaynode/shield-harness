use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type SampleId = String;
pub type Category = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Clean,
    Threat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    RawText,
    Markdown,
    Html,
    ChatHistory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sidecar {
    pub id: SampleId,
    pub text_path: String,
    pub cohort: String,
    pub verdict: Verdict,
    pub format: Format,
    pub source: String,
    pub license: String,
    #[serde(default)]
    pub expected_categories: Vec<Category>,
    #[serde(default)]
    pub expected_min_severity: Option<Severity>,
    #[serde(default)]
    pub seed_id: Option<SampleId>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub sidecar: Sidecar,
    pub text_full_path: PathBuf,
}

impl Sample {
    pub fn read_bytes(&self) -> std::io::Result<Vec<u8>> {
        std::fs::read(&self.text_full_path)
    }
}
