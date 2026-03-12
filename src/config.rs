use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::cli::Environment;

pub const DEFAULT_REAL_BASE_URL: &str = "https://openapi.koreainvestment.com:9443";
pub const DEFAULT_DEMO_BASE_URL: &str = "https://openapivts.koreainvestment.com:29443";
pub const DEFAULT_REAL_WS_URL: &str = "ws://ops.koreainvestment.com:21000";
pub const DEFAULT_DEMO_WS_URL: &str = "ws://ops.koreainvestment.com:31000";
pub const DEFAULT_USER_AGENT: &str = "kis-trading-cli/0.1";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_path: PathBuf,
    pub cache_path: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AppConfig {
    pub user_agent: Option<String>,
    pub profiles: Profiles,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Profiles {
    pub real: Option<KisProfile>,
    pub demo: Option<KisProfile>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct KisProfile {
    pub app_key: Option<String>,
    pub app_secret: Option<String>,
    pub base_url: Option<String>,
    pub websocket_url: Option<String>,
    pub account_no: Option<String>,
    pub account_product_code: Option<String>,
    pub hts_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub environment: Environment,
    pub app_key: String,
    pub app_secret: String,
    pub base_url: String,
    pub websocket_url: Option<String>,
    pub account_no: Option<String>,
    pub account_product_code: Option<String>,
    pub hts_id: Option<String>,
    pub user_agent: String,
}

impl AppConfig {
    fn profile_for(&self, environment: Environment) -> KisProfile {
        match environment {
            Environment::Real => self.profiles.real.clone().unwrap_or_default(),
            Environment::Demo => self.profiles.demo.clone().unwrap_or_default(),
        }
    }
}

pub fn app_paths(config_override: Option<&Path>) -> Result<AppPaths> {
    let dirs = ProjectDirs::from("com", "johwanghee", "kis-trading-cli")
        .ok_or_else(|| anyhow!("failed to resolve OS-specific app directories"))?;

    let config_path = match config_override {
        Some(path) => path.to_path_buf(),
        None => dirs.config_dir().join("config.toml"),
    };

    let cache_path = dirs.cache_dir().join("token-cache.json");

    Ok(AppPaths {
        config_path,
        cache_path,
    })
}

pub fn write_config_template(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(anyhow!(
            "config already exists at {} (use --force to overwrite)",
            path.display()
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    fs::write(path, template_config())
        .with_context(|| format!("failed to write config template to {}", path.display()))?;

    Ok(())
}

pub fn load_profile(
    config_override: Option<&Path>,
    environment: Environment,
) -> Result<ResolvedProfile> {
    let paths = app_paths(config_override)?;

    let config = if paths.config_path.exists() {
        let raw = fs::read_to_string(&paths.config_path).with_context(|| {
            format!("failed to read config file {}", paths.config_path.display())
        })?;

        toml::from_str::<AppConfig>(&raw).with_context(|| {
            format!(
                "failed to parse config file {}",
                paths.config_path.display()
            )
        })?
    } else {
        AppConfig::default()
    };

    let profile = config.profile_for(environment);
    let env_prefix = environment.as_str().to_ascii_uppercase();

    let app_key = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_APP_KEY")),
        env_var("KIS_APP_KEY"),
        profile.app_key,
    ])
    .ok_or_else(|| missing_field_error("app_key", &paths.config_path))?;

    let app_secret = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_APP_SECRET")),
        env_var("KIS_APP_SECRET"),
        profile.app_secret,
    ])
    .ok_or_else(|| missing_field_error("app_secret", &paths.config_path))?;

    let base_url = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_BASE_URL")),
        env_var("KIS_BASE_URL"),
        profile.base_url,
        Some(default_base_url(environment).to_string()),
    ])
    .ok_or_else(|| anyhow!("failed to resolve base_url"))?;

    let websocket_url = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_WEBSOCKET_URL")),
        env_var("KIS_WEBSOCKET_URL"),
        profile.websocket_url,
        Some(default_websocket_url(environment).to_string()),
    ]);

    let account_no = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_ACCOUNT_NO")),
        env_var("KIS_ACCOUNT_NO"),
        profile.account_no,
    ]);

    let account_product_code = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_ACCOUNT_PRODUCT_CODE")),
        env_var("KIS_ACCOUNT_PRODUCT_CODE"),
        profile.account_product_code,
    ]);

    let hts_id = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_HTS_ID")),
        env_var("KIS_HTS_ID"),
        profile.hts_id,
    ]);

    let user_agent = first_non_empty([
        env_var("KIS_USER_AGENT"),
        config.user_agent,
        Some(DEFAULT_USER_AGENT.to_string()),
    ])
    .ok_or_else(|| anyhow!("failed to resolve user_agent"))?;

    Ok(ResolvedProfile {
        environment,
        app_key,
        app_secret,
        base_url,
        websocket_url,
        account_no,
        account_product_code,
        hts_id,
        user_agent,
    })
}

fn env_var(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn first_non_empty<const N: usize>(candidates: [Option<String>; N]) -> Option<String> {
    candidates
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
}

fn default_base_url(environment: Environment) -> &'static str {
    match environment {
        Environment::Real => DEFAULT_REAL_BASE_URL,
        Environment::Demo => DEFAULT_DEMO_BASE_URL,
    }
}

fn default_websocket_url(environment: Environment) -> &'static str {
    match environment {
        Environment::Real => DEFAULT_REAL_WS_URL,
        Environment::Demo => DEFAULT_DEMO_WS_URL,
    }
}

fn missing_field_error(field: &str, config_path: &Path) -> anyhow::Error {
    anyhow!(
        "missing {field}. Run `kis-trading-cli config init` or set an override env var. Expected config file: {}",
        config_path.display()
    )
}

fn template_config() -> String {
    format!(
        concat!(
            "# KIS Trading CLI configuration\n",
            "# Official reference: https://github.com/koreainvestment/open-trading-api\n",
            "#\n",
            "# Environment variable overrides are supported.\n",
            "# Examples:\n",
            "#   KIS_REAL_APP_KEY\n",
            "#   KIS_REAL_APP_SECRET\n",
            "#   KIS_DEMO_APP_KEY\n",
            "#   KIS_DEMO_APP_SECRET\n",
            "#\n",
            "user_agent = \"{user_agent}\"\n",
            "\n",
            "[profiles.real]\n",
            "base_url = \"{real_url}\"\n",
            "websocket_url = \"{real_ws}\"\n",
            "app_key = \"\"\n",
            "app_secret = \"\"\n",
            "account_no = \"\"\n",
            "account_product_code = \"01\"\n",
            "hts_id = \"\"\n",
            "\n",
            "[profiles.demo]\n",
            "base_url = \"{demo_url}\"\n",
            "websocket_url = \"{demo_ws}\"\n",
            "app_key = \"\"\n",
            "app_secret = \"\"\n",
            "account_no = \"\"\n",
            "account_product_code = \"01\"\n",
            "hts_id = \"\"\n"
        ),
        user_agent = DEFAULT_USER_AGENT,
        real_url = DEFAULT_REAL_BASE_URL,
        real_ws = DEFAULT_REAL_WS_URL,
        demo_url = DEFAULT_DEMO_BASE_URL,
        demo_ws = DEFAULT_DEMO_WS_URL
    )
}
