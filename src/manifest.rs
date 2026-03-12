use std::collections::BTreeMap;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiManifest {
    pub source_commit: String,
    pub category_count: usize,
    pub api_count: usize,
    pub categories: Vec<Category>,
    pub apis: Vec<ApiEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Category {
    pub id: String,
    pub config_file: String,
    pub introduce: String,
    pub introduce_append: String,
    pub api_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiEntry {
    pub id: String,
    pub category_id: String,
    pub api_type: String,
    pub command_name: String,
    pub category_label: String,
    pub display_name: String,
    pub method_name: String,
    pub api_path: String,
    pub http_method: String,
    pub github_url: String,
    pub source_file: String,
    pub params: Vec<ApiParam>,
    pub request_fields: Vec<RequestField>,
    pub pagination: Option<Pagination>,
    pub tr_id: TrIdSpec,
    pub post_uses_hashkey: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiParam {
    pub name: String,
    pub cli_name: String,
    pub r#type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: String,
    pub hidden: bool,
    pub auto_source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestField {
    pub request_name: String,
    pub source_param: Option<String>,
    pub literal: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pagination {
    pub ctx_fk_field: Option<String>,
    pub ctx_nk_field: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TrIdSpec {
    None,
    Const { value: String },
    Env { real: String, demo: String },
    Special { resolver: String },
    Unsupported { candidates: Vec<String> },
}

impl ApiManifest {
    pub fn category_by_name(&self, name: &str) -> Option<&Category> {
        self.categories.iter().find(|category| category.id == name)
    }

    pub fn category_entries(&self, category_id: &str) -> Vec<&ApiEntry> {
        self.apis
            .iter()
            .filter(|entry| entry.category_id == category_id)
            .collect()
    }

    pub fn entry_by_command(&self, category_id: &str, command_name: &str) -> Option<&ApiEntry> {
        self.apis.iter().find(|entry| {
            entry.category_id == category_id && display_command_name(entry) == command_name
        })
    }

    pub fn category_counts(&self) -> BTreeMap<&str, usize> {
        self.categories
            .iter()
            .map(|category| (category.id.as_str(), category.api_count))
            .collect()
    }
}

pub fn load_manifest() -> Result<&'static ApiManifest> {
    static MANIFEST: OnceLock<ApiManifest> = OnceLock::new();
    if let Some(manifest) = MANIFEST.get() {
        return Ok(manifest);
    }

    let parsed: ApiManifest = serde_json::from_str(include_str!("../data/kis_api_manifest.json"))
        .context("failed to parse embedded API manifest")?;
    let _ = MANIFEST.set(parsed);

    MANIFEST
        .get()
        .ok_or_else(|| anyhow::anyhow!("failed to initialize embedded API manifest"))
}

pub fn display_command_name(entry: &ApiEntry) -> String {
    if entry.category_id == "auth" {
        if let Some(trimmed) = entry.command_name.strip_prefix("auth-") {
            return trimmed.to_string();
        }
    }

    entry.command_name.clone()
}

pub fn visible_params(entry: &ApiEntry) -> impl Iterator<Item = &ApiParam> {
    entry.params.iter().filter(|param| !param.hidden)
}
