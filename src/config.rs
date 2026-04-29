use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm_siv::{
    aead::{Aead, KeyInit},
    Aes256GcmSiv, Nonce,
};
use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use chrono::Utc;
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
const KEY_FILE_VERSION: u32 = 1;

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

#[derive(Debug, Clone)]
pub struct KeyStatusResult {
    pub key_path: PathBuf,
    pub key_exists: bool,
    pub key_format: Option<&'static str>,
    pub previous_key_count: usize,
    pub encrypted_field_count: usize,
    pub plaintext_field_count: usize,
    pub plaintext_fields: Vec<String>,
    pub seal_required: bool,
    pub suggested_commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct KeyBackupResult {
    pub key_path: PathBuf,
    pub backup_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct KeyImportResult {
    pub key_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub imported_format: &'static str,
    pub previous_key_count: usize,
    pub encrypted_field_count: usize,
}

#[derive(Debug, Clone)]
pub struct KeyRotateResult {
    pub key_path: PathBuf,
    pub backup_path: PathBuf,
    pub rotated_fields: usize,
    pub previous_key_count: usize,
}

#[derive(Debug, Clone)]
pub struct PlaintextSecretError {
    pub config_path: PathBuf,
    pub plaintext_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyFile {
    version: u32,
    active_key: String,
    #[serde(default)]
    previous_keys: Vec<String>,
}

#[derive(Debug, Clone)]
struct KeyMaterial {
    active: [u8; CONFIG_KEY_BYTES],
    previous: Vec<[u8; CONFIG_KEY_BYTES]>,
    format: &'static str,
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

    pub fn dotted_path(self, environment: Environment) -> String {
        format!("profiles.{}.{}", environment.as_str(), self.config_key())
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

pub fn key_status(config_override: Option<&Path>) -> Result<KeyStatusResult> {
    let paths = app_paths(config_override)?;
    let config = load_config_or_default(&paths.config_path)?;
    let encrypted_field_count = count_encrypted_secret_fields(&config);
    let plaintext_fields = collect_plaintext_secret_fields(&config);
    let plaintext_field_count = plaintext_fields.len();
    let seal_required = plaintext_field_count > 0;
    let suggested_commands = if seal_required {
        vec![
            "kis-trading-cli config key status --compact".to_string(),
            "kis-trading-cli config seal".to_string(),
        ]
    } else {
        Vec::new()
    };

    if !paths.key_path.exists() {
        return Ok(KeyStatusResult {
            key_path: paths.key_path,
            key_exists: false,
            key_format: None,
            previous_key_count: 0,
            encrypted_field_count,
            plaintext_field_count,
            plaintext_fields,
            seal_required,
            suggested_commands,
        });
    }

    let key_material = load_key_material_from_path(&paths.key_path)?;
    Ok(KeyStatusResult {
        key_path: paths.key_path,
        key_exists: true,
        key_format: Some(key_material.format),
        previous_key_count: key_material.previous.len(),
        encrypted_field_count,
        plaintext_field_count,
        plaintext_fields,
        seal_required,
        suggested_commands,
    })
}

pub fn backup_key(
    config_override: Option<&Path>,
    output: Option<&Path>,
    force: bool,
) -> Result<KeyBackupResult> {
    let paths = app_paths(config_override)?;
    load_existing_key_material(&paths)?;

    let backup_path = resolve_backup_path(&paths.key_path, output);
    copy_file_checked(&paths.key_path, &backup_path, force)?;

    Ok(KeyBackupResult {
        key_path: paths.key_path,
        backup_path,
    })
}

pub fn import_key(
    config_override: Option<&Path>,
    input_path: &Path,
    backup_output: Option<&Path>,
    force: bool,
) -> Result<KeyImportResult> {
    let paths = app_paths(config_override)?;
    let imported = load_key_material_from_path(input_path)?;
    let config = load_config_or_default(&paths.config_path)?;
    let encrypted_field_count = count_encrypted_secret_fields(&config);
    validate_key_material_against_config(&config, &imported)?;

    let backup_path = if paths.key_path.exists() {
        let path = resolve_backup_path(&paths.key_path, backup_output);
        copy_file_checked(&paths.key_path, &path, force)?;
        Some(path)
    } else {
        None
    };

    write_key_material_to_path(&paths.key_path, &imported)?;

    Ok(KeyImportResult {
        key_path: paths.key_path,
        backup_path,
        imported_format: imported.format,
        previous_key_count: imported.previous.len(),
        encrypted_field_count,
    })
}

pub fn rotate_key(
    config_override: Option<&Path>,
    backup_output: Option<&Path>,
    force: bool,
) -> Result<KeyRotateResult> {
    let paths = app_paths(config_override)?;

    if !paths.config_path.exists() {
        bail!(
            "config file does not exist at {}. Run `kis-trading-cli config init` first.",
            paths.config_path.display()
        );
    }

    let mut config = read_config(&paths.config_path)?;
    let current = load_existing_key_material(&paths)?;
    let backup_path = resolve_backup_path(&paths.key_path, backup_output);
    copy_file_checked(&paths.key_path, &backup_path, force)?;

    let mut next = current.clone();
    next.previous.insert(0, current.active);
    next.active = generate_random_key();
    normalize_key_material(&mut next);
    write_key_material_to_path(&paths.key_path, &next)?;

    let rotated_fields = reencrypt_config_secrets(&mut config, &current, &next)?;
    write_config(&paths.config_path, &config)?;

    Ok(KeyRotateResult {
        key_path: paths.key_path,
        backup_path,
        rotated_fields,
        previous_key_count: next.previous.len(),
    })
}

pub fn load_profile(
    config_override: Option<&Path>,
    environment: Environment,
) -> Result<ResolvedProfile> {
    let paths = app_paths(config_override)?;
    let config = load_config_or_default(&paths.config_path)?;
    ensure_no_plaintext_secret_fields(&config, &paths.config_path)?;
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

impl std::fmt::Display for PlaintextSecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "plaintext sensitive config values detected in {}: {}",
            self.config_path.display(),
            self.plaintext_fields.join(", ")
        )
    }
}

impl std::error::Error for PlaintextSecretError {}

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
    let key_material = load_or_create_key_material(paths)?;
    encrypt_secret_with_key(&key_material.active, plaintext)
}

fn encrypt_secret_with_key(key: &[u8; CONFIG_KEY_BYTES], plaintext: &str) -> Result<String> {
    let cipher =
        Aes256GcmSiv::new_from_slice(key).map_err(|_| anyhow!("invalid config encryption key"))?;
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
    let key_material = load_existing_key_material(paths)?;
    decrypt_secret_with_key_material(&key_material, value)
}

fn decrypt_secret_with_key_material(key_material: &KeyMaterial, value: &str) -> Result<String> {
    let encoded = value
        .strip_prefix(ENCRYPTED_VALUE_PREFIX)
        .ok_or_else(|| anyhow!("unsupported encrypted config value format"))?;
    let payload = STANDARD_NO_PAD
        .decode(encoded)
        .context("invalid encrypted config payload")?;

    if payload.len() <= CONFIG_NONCE_BYTES {
        bail!("encrypted config payload is too short");
    }

    let (nonce_bytes, ciphertext) = payload.split_at(CONFIG_NONCE_BYTES);
    for key in std::iter::once(&key_material.active).chain(key_material.previous.iter()) {
        if let Ok(plaintext) = decrypt_secret_with_key(key, nonce_bytes, ciphertext) {
            return Ok(plaintext);
        }
    }

    bail!("failed to decrypt config secret with available key material")
}

fn decrypt_secret_with_key(
    key: &[u8; CONFIG_KEY_BYTES],
    nonce_bytes: &[u8],
    ciphertext: &[u8],
) -> Result<String> {
    let cipher =
        Aes256GcmSiv::new_from_slice(key).map_err(|_| anyhow!("invalid config encryption key"))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| anyhow!("failed to decrypt config secret"))?;
    String::from_utf8(plaintext).context("config secret is not valid UTF-8")
}

fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_VALUE_PREFIX)
}

fn load_or_create_key_material(paths: &AppPaths) -> Result<KeyMaterial> {
    if paths.key_path.exists() {
        return load_key_material_from_path(&paths.key_path);
    }

    create_key_material(paths)
}

fn load_existing_key_material(paths: &AppPaths) -> Result<KeyMaterial> {
    if !paths.key_path.exists() {
        bail!(
            "missing config encryption key at {}. Restore the original key file or re-enter secrets.",
            paths.key_path.display()
        );
    }

    load_key_material_from_path(&paths.key_path)
}

fn load_key_material_from_path(path: &Path) -> Result<KeyMaterial> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config key file {}", path.display()))?;

    if let Ok(key_file) = toml::from_str::<KeyFile>(&raw) {
        if key_file.version != KEY_FILE_VERSION {
            bail!(
                "unsupported config key file version {} in {}",
                key_file.version,
                path.display()
            );
        }

        let active = decode_key_bytes(&key_file.active_key, path)?;
        let previous = key_file
            .previous_keys
            .iter()
            .map(|value| decode_key_bytes(value, path))
            .collect::<Result<Vec<_>>>()?;
        let mut key_material = KeyMaterial {
            active,
            previous,
            format: "keyring",
        };
        normalize_key_material(&mut key_material);
        return Ok(key_material);
    }

    let encoded = raw
        .trim()
        .strip_prefix(KEY_FILE_PREFIX)
        .unwrap_or(raw.trim());
    let active = decode_key_bytes(encoded, path)?;
    Ok(KeyMaterial {
        active,
        previous: Vec::new(),
        format: "legacy-single-key",
    })
}

fn create_key_material(paths: &AppPaths) -> Result<KeyMaterial> {
    if let Some(parent) = paths.key_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create key directory {}", parent.display()))?;
    }

    let key_material = KeyMaterial {
        active: generate_random_key(),
        previous: Vec::new(),
        format: "keyring",
    };
    write_key_material_to_path(&paths.key_path, &key_material)?;
    Ok(key_material)
}

fn write_key_material_to_path(path: &Path, key_material: &KeyMaterial) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create key directory {}", parent.display()))?;
    }

    let key_file = KeyFile {
        version: KEY_FILE_VERSION,
        active_key: encode_key_bytes(&key_material.active),
        previous_keys: key_material.previous.iter().map(encode_key_bytes).collect(),
    };
    let rendered = toml::to_string_pretty(&key_file).context("failed to render config key TOML")?;
    fs::write(path, format!("{rendered}\n"))
        .with_context(|| format!("failed to write config encryption key {}", path.display()))?;
    restrict_key_permissions(path)?;
    Ok(())
}

fn generate_random_key() -> [u8; CONFIG_KEY_BYTES] {
    let mut key = [0u8; CONFIG_KEY_BYTES];
    rand::rngs::OsRng.fill_bytes(&mut key);
    key
}

fn encode_key_bytes(key: &[u8; CONFIG_KEY_BYTES]) -> String {
    STANDARD_NO_PAD.encode(key)
}

fn decode_key_bytes(encoded: &str, path: &Path) -> Result<[u8; CONFIG_KEY_BYTES]> {
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

fn normalize_key_material(key_material: &mut KeyMaterial) {
    let mut unique = Vec::new();
    for candidate in key_material.previous.drain(..) {
        if candidate == key_material.active || unique.iter().any(|existing| *existing == candidate)
        {
            continue;
        }
        unique.push(candidate);
    }
    key_material.previous = unique;
}

fn copy_file_checked(source: &Path, destination: &Path, force: bool) -> Result<()> {
    if source == destination {
        bail!("source and destination paths must be different");
    }

    if destination.exists() && !force {
        bail!(
            "destination already exists at {} (use --force to overwrite)",
            destination.display()
        );
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create backup/import directory {}",
                parent.display()
            )
        })?;
    }

    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to copy config key file from {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    restrict_key_permissions(destination)?;
    Ok(())
}

fn resolve_backup_path(key_path: &Path, output: Option<&Path>) -> PathBuf {
    output.map(Path::to_path_buf).unwrap_or_else(|| {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        let filename = key_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("config.key");
        key_path.with_file_name(format!("{filename}.backup-{timestamp}"))
    })
}

fn count_encrypted_secret_fields(config: &AppConfig) -> usize {
    [config.profiles.real.as_ref(), config.profiles.demo.as_ref()]
        .into_iter()
        .flatten()
        .map(count_encrypted_fields_in_profile)
        .sum()
}

fn collect_plaintext_secret_fields(config: &AppConfig) -> Vec<String> {
    let mut fields = Vec::new();

    for (environment, profile) in [
        (Environment::Real, config.profiles.real.as_ref()),
        (Environment::Demo, config.profiles.demo.as_ref()),
    ] {
        let Some(profile) = profile else {
            continue;
        };

        for field in SecretField::ALL {
            let slot = match field {
                SecretField::AppKey => profile.app_key.as_ref(),
                SecretField::AppSecret => profile.app_secret.as_ref(),
                SecretField::AccountNo => profile.account_no.as_ref(),
                SecretField::HtsId => profile.hts_id.as_ref(),
            };

            let Some(value) = slot else {
                continue;
            };

            if !value.trim().is_empty() && !is_encrypted(value) {
                fields.push(field.dotted_path(environment));
            }
        }
    }

    fields
}

fn ensure_no_plaintext_secret_fields(config: &AppConfig, config_path: &Path) -> Result<()> {
    let plaintext_fields = collect_plaintext_secret_fields(config);
    if plaintext_fields.is_empty() {
        return Ok(());
    }

    Err(PlaintextSecretError {
        config_path: config_path.to_path_buf(),
        plaintext_fields,
    }
    .into())
}

fn count_encrypted_fields_in_profile(profile: &KisProfile) -> usize {
    [
        profile.app_key.as_ref(),
        profile.app_secret.as_ref(),
        profile.account_no.as_ref(),
        profile.hts_id.as_ref(),
    ]
    .into_iter()
    .flatten()
    .filter(|value| is_encrypted(value))
    .count()
}

fn validate_key_material_against_config(
    config: &AppConfig,
    key_material: &KeyMaterial,
) -> Result<()> {
    for profile in [config.profiles.real.as_ref(), config.profiles.demo.as_ref()]
        .into_iter()
        .flatten()
    {
        validate_profile_secrets(profile, key_material)?;
    }

    Ok(())
}

fn validate_profile_secrets(profile: &KisProfile, key_material: &KeyMaterial) -> Result<()> {
    for value in [
        profile.app_key.as_ref(),
        profile.app_secret.as_ref(),
        profile.account_no.as_ref(),
        profile.hts_id.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if is_encrypted(value) {
            decrypt_secret_with_key_material(key_material, value)?;
        }
    }

    Ok(())
}

fn reencrypt_config_secrets(
    config: &mut AppConfig,
    current: &KeyMaterial,
    next: &KeyMaterial,
) -> Result<usize> {
    let mut rotated_fields = 0;

    for profile in [config.profiles.real.as_mut(), config.profiles.demo.as_mut()]
        .into_iter()
        .flatten()
    {
        rotated_fields += reencrypt_profile_secrets(profile, current, next)?;
    }

    Ok(rotated_fields)
}

fn reencrypt_profile_secrets(
    profile: &mut KisProfile,
    current: &KeyMaterial,
    next: &KeyMaterial,
) -> Result<usize> {
    let mut rotated = 0;

    for slot in [
        &mut profile.app_key,
        &mut profile.app_secret,
        &mut profile.account_no,
        &mut profile.hts_id,
    ] {
        let Some(value) = slot.as_ref() else {
            continue;
        };

        if value.trim().is_empty() {
            continue;
        }

        let plaintext = if is_encrypted(value) {
            decrypt_secret_with_key_material(current, value)?
        } else {
            value.clone()
        };

        *slot = Some(encrypt_secret_with_key(&next.active, &plaintext)?);
        rotated += 1;
    }

    Ok(rotated)
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
            "#   4. API/auth commands refuse plaintext sensitive values until they are sealed.\n",
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

    #[test]
    fn rotate_key_keeps_profile_readable_and_upgrades_key_format() {
        let config_path = temp_config_path("rotate");
        let backup_path = config_path.with_extension("key.backup");
        write_config_template(&config_path, false).expect("template should be written");

        set_secret(
            Some(&config_path),
            Environment::Real,
            SecretField::AppKey,
            "rotating-key",
        )
        .expect("app key should be stored");
        set_secret(
            Some(&config_path),
            Environment::Real,
            SecretField::AppSecret,
            "rotating-secret",
        )
        .expect("app secret should be stored");

        let result =
            rotate_key(Some(&config_path), Some(&backup_path), false).expect("rotate should work");
        assert_eq!(result.rotated_fields, 2);
        assert_eq!(result.previous_key_count, 1);
        assert!(backup_path.exists());

        let status = key_status(Some(&config_path)).expect("status should load");
        assert_eq!(status.key_format, Some("keyring"));
        assert_eq!(status.previous_key_count, 1);

        let profile = load_profile(Some(&config_path), Environment::Real)
            .expect("rotated profile should decrypt");
        assert_eq!(profile.app_key, "rotating-key");
        assert_eq!(profile.app_secret, "rotating-secret");

        let paths = test_paths(&config_path);
        let _ = fs::remove_file(&paths.config_path);
        let _ = fs::remove_file(&paths.key_path);
        let _ = fs::remove_file(&backup_path);
    }

    #[test]
    fn import_key_rejects_backup_that_cannot_decrypt_current_config() {
        let config_path = temp_config_path("import-invalid");
        let backup_path = config_path.with_extension("key.backup");
        write_config_template(&config_path, false).expect("template should be written");

        set_secret(
            Some(&config_path),
            Environment::Demo,
            SecretField::AppKey,
            "demo-rotate-key",
        )
        .expect("app key should be stored");
        set_secret(
            Some(&config_path),
            Environment::Demo,
            SecretField::AppSecret,
            "demo-rotate-secret",
        )
        .expect("app secret should be stored");

        rotate_key(Some(&config_path), Some(&backup_path), false).expect("rotate should work");

        let error = import_key(Some(&config_path), &backup_path, None, false)
            .expect_err("old backup key should not decrypt rotated config");
        assert!(error.to_string().contains("failed to decrypt"));

        let paths = test_paths(&config_path);
        let _ = fs::remove_file(&paths.config_path);
        let _ = fs::remove_file(&paths.key_path);
        let _ = fs::remove_file(&backup_path);
    }

    #[test]
    fn key_status_reports_plaintext_sensitive_fields() {
        let config_path = temp_config_path("plaintext-status");
        write_config_template(&config_path, false).expect("template should be written");

        let mut config = read_config(&config_path).expect("config should parse");
        let profile = config.profile_for_mut(Environment::Demo);
        profile.app_key = Some("demo-key".to_string());
        profile.account_no = Some("12345678".to_string());
        write_config(&config_path, &config).expect("plaintext config should be written");

        let status = key_status(Some(&config_path)).expect("status should work");
        assert_eq!(status.plaintext_field_count, 2);
        assert!(status.seal_required);
        assert!(status
            .plaintext_fields
            .contains(&"profiles.demo.app_key".to_string()));
        assert!(status
            .plaintext_fields
            .contains(&"profiles.demo.account_no".to_string()));
        assert!(status
            .suggested_commands
            .contains(&"kis-trading-cli config seal".to_string()));

        let paths = test_paths(&config_path);
        let _ = fs::remove_file(&paths.config_path);
        let _ = fs::remove_file(&paths.key_path);
    }

    #[test]
    fn load_profile_rejects_plaintext_sensitive_fields() {
        let config_path = temp_config_path("plaintext-reject");
        write_config_template(&config_path, false).expect("template should be written");

        let mut config = read_config(&config_path).expect("config should parse");
        let profile = config.profile_for_mut(Environment::Real);
        profile.app_key = Some("real-key".to_string());
        profile.app_secret = Some("real-secret".to_string());
        write_config(&config_path, &config).expect("plaintext config should be written");

        let error = load_profile(Some(&config_path), Environment::Real)
            .expect_err("plaintext secrets should be rejected");
        let plaintext = error
            .downcast_ref::<PlaintextSecretError>()
            .expect("should be plaintext secret error");
        assert!(plaintext
            .plaintext_fields
            .contains(&"profiles.real.app_key".to_string()));
        assert!(plaintext
            .plaintext_fields
            .contains(&"profiles.real.app_secret".to_string()));

        let paths = test_paths(&config_path);
        let _ = fs::remove_file(&paths.config_path);
        let _ = fs::remove_file(&paths.key_path);
    }
}
