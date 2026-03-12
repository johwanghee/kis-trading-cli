use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm_siv::{
    aead::{Aead, KeyInit},
    Aes256GcmSiv, Nonce,
};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use directories::ProjectDirs;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::cli::Environment;

pub const DEFAULT_REAL_BASE_URL: &str = "https://openapi.koreainvestment.com:9443";
pub const DEFAULT_DEMO_BASE_URL: &str = "https://openapivts.koreainvestment.com:29443";
pub const DEFAULT_REAL_WS_URL: &str = "ws://ops.koreainvestment.com:21000";
pub const DEFAULT_DEMO_WS_URL: &str = "ws://ops.koreainvestment.com:31000";
pub const DEFAULT_USER_AGENT: &str = concat!("kis-trading-cli/", env!("CARGO_PKG_VERSION"));

const CONFIG_KEY_BYTES: usize = 32;
const CONFIG_NONCE_BYTES: usize = 12;
const ENCRYPTED_VALUE_PREFIX: &str = "enc:kis:v1:";
const KEY_FILE_PREFIX: &str = "kis-key-v1:";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_path: PathBuf,
    pub cache_path: PathBuf,
    pub key_path: PathBuf,
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
    #[allow(dead_code)]
    pub websocket_url: Option<String>,
    pub account_no: Option<String>,
    pub account_product_code: Option<String>,
    pub hts_id: Option<String>,
    pub user_agent: String,
}

#[derive(Debug, Clone, Copy)]
pub enum SecretField {
    AppKey,
    AppSecret,
    AccountNo,
    HtsId,
}

#[derive(Debug, Clone)]
pub struct SecretWriteResult {
    pub profile: Environment,
    pub field: SecretField,
    pub config_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SealConfigResult {
    pub encrypted_fields: usize,
    pub profiles_touched: usize,
    pub config_path: PathBuf,
    pub key_path: PathBuf,
}

impl AppConfig {
    fn profile_for(&self, environment: Environment) -> KisProfile {
        match environment {
            Environment::Real => self.profiles.real.clone().unwrap_or_default(),
            Environment::Demo => self.profiles.demo.clone().unwrap_or_default(),
        }
    }

    fn profile_for_mut(&mut self, environment: Environment) -> &mut KisProfile {
        match environment {
            Environment::Real => self.profiles.real.get_or_insert_with(KisProfile::default),
            Environment::Demo => self.profiles.demo.get_or_insert_with(KisProfile::default),
        }
    }
}

impl SecretField {
    pub const ALL: [Self; 4] = [Self::AppKey, Self::AppSecret, Self::AccountNo, Self::HtsId];

    pub fn cli_name(self) -> &'static str {
        match self {
            Self::AppKey => "app-key",
            Self::AppSecret => "app-secret",
            Self::AccountNo => "account-no",
            Self::HtsId => "hts-id",
        }
    }

    pub fn config_key(self) -> &'static str {
        match self {
            Self::AppKey => "app_key",
            Self::AppSecret => "app_secret",
            Self::AccountNo => "account_no",
            Self::HtsId => "hts_id",
        }
    }

    pub fn from_cli_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|field| field.cli_name() == name)
    }

    fn slot_mut<'a>(self, profile: &'a mut KisProfile) -> &'a mut Option<String> {
        match self {
            Self::AppKey => &mut profile.app_key,
            Self::AppSecret => &mut profile.app_secret,
            Self::AccountNo => &mut profile.account_no,
            Self::HtsId => &mut profile.hts_id,
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
    let key_path = match config_override {
        Some(path) => path.with_extension("key"),
        None => dirs.data_local_dir().join("config.key"),
    };

    Ok(AppPaths {
        config_path,
        cache_path,
        key_path,
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

pub fn set_secret(
    config_override: Option<&Path>,
    environment: Environment,
    field: SecretField,
    plaintext: &str,
) -> Result<SecretWriteResult> {
    let paths = app_paths(config_override)?;
    ensure_config_exists(&paths.config_path)?;

    let mut config = read_config(&paths.config_path)?;
    let encrypted = encrypt_secret(&paths, plaintext)?;
    *field.slot_mut(config.profile_for_mut(environment)) = Some(encrypted);
    write_config(&paths.config_path, &config)?;

    Ok(SecretWriteResult {
        profile: environment,
        field,
        config_path: paths.config_path,
        key_path: paths.key_path,
    })
}

pub fn seal_config(
    config_override: Option<&Path>,
    environment: Option<Environment>,
) -> Result<SealConfigResult> {
    let paths = app_paths(config_override)?;

    if !paths.config_path.exists() {
        bail!(
            "config file does not exist at {}. Run `kis-trading-cli config init` first.",
            paths.config_path.display()
        );
    }

    let mut config = read_config(&paths.config_path)?;
    let mut encrypted_fields = 0;
    let mut profiles_touched = 0;

    for current in selected_profiles(environment) {
        let Some(profile) = profile_option_mut(&mut config.profiles, current) else {
            continue;
        };

        let mut touched = false;
        for field in SecretField::ALL {
            let Some(value) = field.slot_mut(profile) else {
                continue;
            };

            if value.trim().is_empty() || is_encrypted(value) {
                continue;
            }

            *value = encrypt_secret(&paths, value)?;
            encrypted_fields += 1;
            touched = true;
        }

        if touched {
            profiles_touched += 1;
        }
    }

    if encrypted_fields > 0 {
        write_config(&paths.config_path, &config)?;
    }

    Ok(SealConfigResult {
        encrypted_fields,
        profiles_touched,
        config_path: paths.config_path,
        key_path: paths.key_path,
    })
}

pub fn load_profile(
    config_override: Option<&Path>,
    environment: Environment,
) -> Result<ResolvedProfile> {
    let paths = app_paths(config_override)?;
    let config = load_config_or_default(&paths.config_path)?;
    let profile = config.profile_for(environment);
    let env_prefix = environment.as_str().to_ascii_uppercase();

    let app_key = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_APP_KEY")),
        env_var("KIS_APP_KEY"),
        resolve_secret_value(profile.app_key, &paths, "app_key")?,
    ])
    .ok_or_else(|| missing_field_error("app_key", &paths.config_path))?;

    let app_secret = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_APP_SECRET")),
        env_var("KIS_APP_SECRET"),
        resolve_secret_value(profile.app_secret, &paths, "app_secret")?,
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
        resolve_secret_value(profile.account_no, &paths, "account_no")?,
    ]);

    let account_product_code = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_ACCOUNT_PRODUCT_CODE")),
        env_var("KIS_ACCOUNT_PRODUCT_CODE"),
        profile.account_product_code,
    ]);

    let hts_id = first_non_empty([
        env_var(&format!("KIS_{env_prefix}_HTS_ID")),
        env_var("KIS_HTS_ID"),
        resolve_secret_value(profile.hts_id, &paths, "hts_id")?,
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

fn load_config_or_default(path: &Path) -> Result<AppConfig> {
    if path.exists() {
        read_config(path)
    } else {
        Ok(AppConfig::default())
    }
}

fn read_config(path: &Path) -> Result<AppConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;

    toml::from_str::<AppConfig>(&raw)
        .with_context(|| format!("failed to parse config file {}", path.display()))
}

fn write_config(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let rendered = toml::to_string_pretty(config).context("failed to render config TOML")?;
    fs::write(path, format!("{rendered}\n"))
        .with_context(|| format!("failed to write config file {}", path.display()))?;
    Ok(())
}

fn ensure_config_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        write_config_template(path, false)?;
    }

    Ok(())
}

fn resolve_secret_value(
    value: Option<String>,
    paths: &AppPaths,
    field: &str,
) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    if value.trim().is_empty() {
        return Ok(None);
    }

    if !is_encrypted(&value) {
        return Ok(Some(value));
    }

    decrypt_secret(paths, &value)
        .with_context(|| format!("failed to decrypt config field `{field}`"))
        .map(Some)
}

fn encrypt_secret(paths: &AppPaths, plaintext: &str) -> Result<String> {
    let key = load_or_create_master_key(paths)?;
    let cipher =
        Aes256GcmSiv::new_from_slice(&key).map_err(|_| anyhow!("invalid config encryption key"))?;
    let mut nonce_bytes = [0u8; CONFIG_NONCE_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| anyhow!("failed to encrypt config secret"))?;

    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);
    Ok(format!(
        "{ENCRYPTED_VALUE_PREFIX}{}",
        STANDARD_NO_PAD.encode(payload)
    ))
}

fn decrypt_secret(paths: &AppPaths, value: &str) -> Result<String> {
    let encoded = value
        .strip_prefix(ENCRYPTED_VALUE_PREFIX)
        .ok_or_else(|| anyhow!("unsupported encrypted config value format"))?;
    let payload = STANDARD_NO_PAD
        .decode(encoded)
        .context("invalid encrypted config payload")?;

    if payload.len() <= CONFIG_NONCE_BYTES {
        bail!("encrypted config payload is too short");
    }

    let key = load_existing_master_key(paths)?;
    let cipher =
        Aes256GcmSiv::new_from_slice(&key).map_err(|_| anyhow!("invalid config encryption key"))?;
    let (nonce_bytes, ciphertext) = payload.split_at(CONFIG_NONCE_BYTES);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| anyhow!("failed to decrypt config secret"))?;

    String::from_utf8(plaintext).context("config secret is not valid UTF-8")
}

fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_VALUE_PREFIX)
}

fn load_or_create_master_key(paths: &AppPaths) -> Result<[u8; CONFIG_KEY_BYTES]> {
    if paths.key_path.exists() {
        return load_master_key_from_path(&paths.key_path);
    }

    create_master_key(paths)
}

fn load_existing_master_key(paths: &AppPaths) -> Result<[u8; CONFIG_KEY_BYTES]> {
    if !paths.key_path.exists() {
        bail!(
            "missing config encryption key at {}. Restore the original key file or re-enter secrets.",
            paths.key_path.display()
        );
    }

    load_master_key_from_path(&paths.key_path)
}

fn load_master_key_from_path(path: &Path) -> Result<[u8; CONFIG_KEY_BYTES]> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config key file {}", path.display()))?;
    let encoded = raw
        .trim()
        .strip_prefix(KEY_FILE_PREFIX)
        .unwrap_or(raw.trim());
    let decoded = STANDARD_NO_PAD
        .decode(encoded)
        .with_context(|| format!("failed to decode config key file {}", path.display()))?;

    if decoded.len() != CONFIG_KEY_BYTES {
        bail!(
            "invalid config key length in {} (expected {} bytes)",
            path.display(),
            CONFIG_KEY_BYTES
        );
    }

    let mut key = [0u8; CONFIG_KEY_BYTES];
    key.copy_from_slice(&decoded);
    Ok(key)
}

fn create_master_key(paths: &AppPaths) -> Result<[u8; CONFIG_KEY_BYTES]> {
    if let Some(parent) = paths.key_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create key directory {}", parent.display()))?;
    }

    let mut key = [0u8; CONFIG_KEY_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut key);

    let contents = format!("{KEY_FILE_PREFIX}{}\n", STANDARD_NO_PAD.encode(key));
    fs::write(&paths.key_path, contents).with_context(|| {
        format!(
            "failed to write config encryption key {}",
            paths.key_path.display()
        )
    })?;
    restrict_key_permissions(&paths.key_path)?;

    Ok(key)
}

#[cfg(unix)]
fn restrict_key_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions).with_context(|| {
        format!(
            "failed to apply restrictive permissions to {}",
            path.display()
        )
    })?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_key_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn profile_option_mut(
    profiles: &mut Profiles,
    environment: Environment,
) -> Option<&mut KisProfile> {
    match environment {
        Environment::Real => profiles.real.as_mut(),
        Environment::Demo => profiles.demo.as_mut(),
    }
}

fn selected_profiles(environment: Option<Environment>) -> Vec<Environment> {
    match environment {
        Some(environment) => vec![environment],
        None => vec![Environment::Real, Environment::Demo],
    }
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
        "missing {field}. Run `kis-trading-cli config init`, `kis-trading-cli config set-secret`, or set an override env var. Expected config file: {}",
        config_path.display()
    )
}

fn template_config() -> String {
    format!(
        concat!(
            "# KIS Trading CLI configuration\n",
            "# Official reference: https://github.com/koreainvestment/open-trading-api\n",
            "#\n",
            "# Recommended secret flow:\n",
            "#   1. Fill non-secret values here.\n",
            "#   2. Store secrets with `kis-trading-cli config set-secret`.\n",
            "#   3. If this file already contains plaintext secrets, run `kis-trading-cli config seal`.\n",
            "#\n",
            "# Environment variable overrides remain supported and are used as plaintext.\n",
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

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_config_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("kis-trading-cli-{name}-{unique}.toml"))
    }

    fn test_paths(config_path: &Path) -> AppPaths {
        let key_path = config_path.with_extension("key");
        let cache_path = config_path.with_extension("cache.json");
        AppPaths {
            config_path: config_path.to_path_buf(),
            cache_path,
            key_path,
        }
    }

    #[test]
    fn set_secret_encrypts_value_and_load_profile_decrypts_it() {
        let config_path = temp_config_path("encrypt");
        write_config_template(&config_path, false).expect("template should be written");

        set_secret(
            Some(&config_path),
            Environment::Real,
            SecretField::AppKey,
            "real-key",
        )
        .expect("app key should be stored");
        set_secret(
            Some(&config_path),
            Environment::Real,
            SecretField::AppSecret,
            "real-secret",
        )
        .expect("app secret should be stored");

        let raw = fs::read_to_string(&config_path).expect("config should be readable");
        assert!(!raw.contains("real-key"));
        assert!(!raw.contains("real-secret"));
        assert!(raw.contains(ENCRYPTED_VALUE_PREFIX));

        let profile = load_profile(Some(&config_path), Environment::Real)
            .expect("encrypted profile should decrypt");
        assert_eq!(profile.app_key, "real-key");
        assert_eq!(profile.app_secret, "real-secret");

        let paths = test_paths(&config_path);
        let _ = fs::remove_file(&paths.config_path);
        let _ = fs::remove_file(&paths.key_path);
    }

    #[test]
    fn seal_config_encrypts_existing_plaintext_fields() {
        let config_path = temp_config_path("seal");
        write_config_template(&config_path, false).expect("template should be written");

        let mut config = read_config(&config_path).expect("config should parse");
        let profile = config.profile_for_mut(Environment::Demo);
        profile.app_key = Some("demo-key".to_string());
        profile.app_secret = Some("demo-secret".to_string());
        profile.account_no = Some("12345678".to_string());
        write_config(&config_path, &config).expect("plaintext config should be written");

        let result =
            seal_config(Some(&config_path), Some(Environment::Demo)).expect("seal should succeed");
        assert_eq!(result.encrypted_fields, 3);
        assert_eq!(result.profiles_touched, 1);

        let raw = fs::read_to_string(&config_path).expect("config should be readable");
        assert!(!raw.contains("demo-key"));
        assert!(!raw.contains("demo-secret"));
        assert!(!raw.contains("12345678"));
        assert!(raw.contains(ENCRYPTED_VALUE_PREFIX));

        let profile = load_profile(Some(&config_path), Environment::Demo)
            .expect("sealed profile should decrypt");
        assert_eq!(profile.app_key, "demo-key");
        assert_eq!(profile.app_secret, "demo-secret");
        assert_eq!(profile.account_no.as_deref(), Some("12345678"));

        let paths = test_paths(&config_path);
        let _ = fs::remove_file(&paths.config_path);
        let _ = fs::remove_file(&paths.key_path);
    }
}
