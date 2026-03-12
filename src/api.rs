use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Asia::Seoul;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::{header::HeaderMap, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cli::Environment;
use crate::config::{AppPaths, ResolvedProfile};

const KIS_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    Cache,
    Fresh,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccessTokenInfo {
    pub environment: Environment,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
    pub source: TokenSource,
}

#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub method: Method,
    pub path: String,
    pub tr_id: Option<String>,
    pub auto_adjust_tr_id: bool,
    pub tr_cont: String,
    pub query: Vec<(String, String)>,
    pub body: Option<Value>,
    pub hashkey: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiCallResponse {
    pub body: Value,
    pub tr_cont: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenRecord {
    access_token: String,
    expires_at: DateTime<Utc>,
    issued_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TokenCache {
    real: Option<TokenRecord>,
    demo: Option<TokenRecord>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    access_token_token_expired: String,
}

#[derive(Debug, Deserialize)]
struct HashKeyResponse {
    #[serde(rename = "HASH")]
    hash: String,
}

#[derive(Debug, Deserialize)]
struct ApprovalResponse {
    approval_key: String,
}

pub struct KisClient {
    http: Client,
    profile: ResolvedProfile,
    cache_path: PathBuf,
}

struct FileLockGuard {
    file: File,
}

impl KisClient {
    pub fn new(profile: ResolvedProfile, paths: AppPaths) -> Result<Self> {
        let http = Client::builder()
            .user_agent(profile.user_agent.clone())
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            http,
            profile,
            cache_path: paths.cache_path,
        })
    }

    pub fn profile(&self) -> &ResolvedProfile {
        &self.profile
    }

    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    pub fn access_token(&self, force_refresh: bool) -> Result<AccessTokenInfo> {
        let _lock = self.acquire_token_lock()?;
        self.access_token_locked(force_refresh)
    }

    pub fn websocket_approval_key(&self) -> Result<Value> {
        let payload = json!({
            "grant_type": "client_credentials",
            "appkey": self.profile.app_key,
            "secretkey": self.profile.app_secret,
        });

        let url = self.build_url("/oauth2/Approval");
        let response = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .header("accept", "text/plain")
            .header("charset", "UTF-8")
            .json(&payload)
            .send()
            .context("failed to request websocket approval key")?;

        let status = response.status();
        let text = response
            .text()
            .context("failed to read websocket approval response body")?;
        if !status.is_success() {
            return Err(anyhow!(
                "KIS websocket approval HTTP error {status}: {text}"
            ));
        }

        let parsed: ApprovalResponse =
            serde_json::from_str(&text).context("failed to parse websocket approval response")?;

        Ok(json!({
            "environment": self.profile.environment,
            "approval_key": parsed.approval_key,
        }))
    }

    pub fn send_request(&self, request: ApiRequest) -> Result<ApiCallResponse> {
        let token = self.access_token(false)?;
        let url = self.build_url(&request.path);

        let mut builder = self.authorized_request(
            request.method.clone(),
            &url,
            &token.access_token,
            request.tr_id.as_deref(),
            request.auto_adjust_tr_id,
            &request.tr_cont,
        );

        if !request.query.is_empty() {
            builder = builder.query(&request.query);
        }

        if let Some(body) = request.body {
            if request.hashkey {
                let hash = self.fetch_hashkey(&token.access_token, &body)?;
                builder = builder.header("hashkey", hash);
            }

            builder = builder.json(&body);
        }

        let response = builder.send().context("failed to execute KIS request")?;
        parse_api_response(response, &request.path)
    }

    fn access_token_locked(&self, force_refresh: bool) -> Result<AccessTokenInfo> {
        if !force_refresh {
            if let Some(record) = self.read_cache()?.get(self.profile.environment).cloned() {
                if record.is_valid() {
                    return Ok(AccessTokenInfo {
                        environment: self.profile.environment,
                        access_token: record.access_token,
                        expires_at: record.expires_at,
                        issued_at: record.issued_at,
                        source: TokenSource::Cache,
                    });
                }
            }
        }

        let payload = json!({
            "grant_type": "client_credentials",
            "appkey": self.profile.app_key,
            "appsecret": self.profile.app_secret,
        });

        let url = self.build_url("/oauth2/tokenP");
        let response = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .header("accept", "text/plain")
            .header("charset", "UTF-8")
            .json(&payload)
            .send()
            .context("failed to request OAuth token")?;

        let status = response.status();
        let text = response
            .text()
            .context("failed to read OAuth response body")?;
        if !status.is_success() {
            return Err(anyhow!("KIS token HTTP error {status}: {text}"));
        }

        let parsed: TokenResponse =
            serde_json::from_str(&text).context("failed to parse OAuth token response")?;

        let expires_at = parse_kis_datetime(&parsed.access_token_token_expired)?;
        let record = TokenRecord {
            access_token: parsed.access_token,
            expires_at,
            issued_at: Utc::now(),
        };

        self.write_cache_record(record.clone())?;

        Ok(AccessTokenInfo {
            environment: self.profile.environment,
            access_token: record.access_token,
            expires_at: record.expires_at,
            issued_at: record.issued_at,
            source: TokenSource::Fresh,
        })
    }

    fn authorized_request(
        &self,
        method: Method,
        url: &str,
        access_token: &str,
        tr_id: Option<&str>,
        auto_adjust_tr_id: bool,
        tr_cont: &str,
    ) -> RequestBuilder {
        let mut builder = self
            .http
            .request(method, url)
            .header("content-type", "application/json")
            .header("accept", "text/plain")
            .header("charset", "UTF-8")
            .header("authorization", format!("Bearer {access_token}"))
            .header("appkey", &self.profile.app_key)
            .header("appsecret", &self.profile.app_secret)
            .header("custtype", "P")
            .header("tr_cont", tr_cont);

        if let Some(tr_id) = tr_id {
            let header_value = if auto_adjust_tr_id {
                adjust_tr_id(self.profile.environment, tr_id)
            } else {
                tr_id.to_string()
            };
            builder = builder.header("tr_id", header_value);
        }

        builder
    }

    fn fetch_hashkey(&self, access_token: &str, body: &Value) -> Result<String> {
        let url = self.build_url("/uapi/hashkey");
        let response = self
            .authorized_request(Method::POST, &url, access_token, None, false, "")
            .json(body)
            .send()
            .context("failed to request hashkey")?;

        let status = response.status();
        let text = response
            .text()
            .context("failed to read hashkey response body")?;
        if !status.is_success() {
            return Err(anyhow!("KIS hashkey HTTP error {status}: {text}"));
        }

        let parsed: HashKeyResponse =
            serde_json::from_str(&text).context("failed to parse hashkey response")?;

        Ok(parsed.hash)
    }

    fn build_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }

        let base = self.profile.base_url.trim_end_matches('/');
        let suffix = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        format!("{base}{suffix}")
    }

    fn acquire_token_lock(&self) -> Result<FileLockGuard> {
        let lock_path = self.lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create lock directory {}", parent.display()))?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("failed to open lock file {}", lock_path.display()))?;

        file.lock()
            .with_context(|| format!("failed to lock {}", lock_path.display()))?;

        Ok(FileLockGuard { file })
    }

    fn lock_path(&self) -> PathBuf {
        let file_name = self
            .cache_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("token-cache.json");

        self.cache_path.with_file_name(format!("{file_name}.lock"))
    }

    fn read_cache(&self) -> Result<TokenCache> {
        if !self.cache_path.exists() {
            return Ok(TokenCache::default());
        }

        let raw = fs::read_to_string(&self.cache_path)
            .with_context(|| format!("failed to read token cache {}", self.cache_path.display()))?;

        serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse token cache {}", self.cache_path.display()))
    }

    fn write_cache_record(&self, record: TokenRecord) -> Result<()> {
        let mut cache = self.read_cache().unwrap_or_default();
        cache.set(self.profile.environment, record);

        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create token cache directory {}",
                    parent.display()
                )
            })?;
        }

        let raw =
            serde_json::to_string_pretty(&cache).context("failed to serialize token cache")?;

        fs::write(&self.cache_path, raw)
            .with_context(|| format!("failed to write token cache {}", self.cache_path.display()))
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

impl TokenRecord {
    fn is_valid(&self) -> bool {
        self.expires_at - Duration::seconds(60) > Utc::now()
    }
}

impl TokenCache {
    fn get(&self, environment: Environment) -> Option<&TokenRecord> {
        match environment {
            Environment::Real => self.real.as_ref(),
            Environment::Demo => self.demo.as_ref(),
        }
    }

    fn set(&mut self, environment: Environment, record: TokenRecord) {
        match environment {
            Environment::Real => self.real = Some(record),
            Environment::Demo => self.demo = Some(record),
        }
    }
}

pub fn adjust_tr_id(environment: Environment, tr_id: &str) -> String {
    if matches!(environment, Environment::Demo) {
        let mut chars = tr_id.chars();
        if let Some(first) = chars.next() {
            if matches!(first, 'T' | 'J' | 'C') {
                return format!("V{}", chars.collect::<String>());
            }
        }
    }

    tr_id.to_string()
}

fn parse_kis_datetime(raw: &str) -> Result<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(raw, KIS_TIME_FORMAT)
        .with_context(|| format!("failed to parse KIS datetime `{raw}`"))?;
    let zoned = Seoul
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| anyhow!("invalid KIS datetime `{raw}`"))?;

    Ok(zoned.with_timezone(&Utc))
}

fn parse_api_response(
    response: reqwest::blocking::Response,
    path: &str,
) -> Result<ApiCallResponse> {
    let status = response.status();
    let headers = response.headers().clone();
    let text = response
        .text()
        .with_context(|| format!("failed to read response body for {path}"))?;

    if !status.is_success() {
        return Err(anyhow!("KIS HTTP error {status} for {path}: {text}"));
    }

    let value: Value =
        serde_json::from_str(&text).with_context(|| format!("failed to parse JSON for {path}"))?;

    if let Some(rt_cd) = value.get("rt_cd").and_then(Value::as_str) {
        if rt_cd != "0" {
            let msg_cd = value
                .get("msg_cd")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let msg1 = value
                .get("msg1")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            return Err(anyhow!("KIS API error {msg_cd} for {path}: {msg1}"));
        }
    }

    Ok(ApiCallResponse {
        body: value,
        tr_cont: header_string(&headers, "tr_cont").unwrap_or_default(),
    })
}

fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn demo_environment_rewrites_supported_tr_prefix() {
        assert_eq!(adjust_tr_id(Environment::Demo, "TTTC0802U"), "VTTC0802U");
        assert_eq!(adjust_tr_id(Environment::Demo, "JTTC0802U"), "VTTC0802U");
        assert_eq!(adjust_tr_id(Environment::Demo, "CTRP6504R"), "VTRP6504R");
    }

    #[test]
    fn real_environment_keeps_original_tr_id() {
        assert_eq!(adjust_tr_id(Environment::Real, "TTTC0802U"), "TTTC0802U");
        assert_eq!(
            adjust_tr_id(Environment::Real, "FHKST01010100"),
            "FHKST01010100"
        );
    }

    #[test]
    fn parses_kis_datetime_as_seoul_time() {
        let parsed = parse_kis_datetime("2026-03-12 15:30:00").expect("datetime should parse");
        let seoul = parsed.with_timezone(&Seoul);

        assert_eq!(seoul.year(), 2026);
        assert_eq!(seoul.month(), 3);
        assert_eq!(seoul.day(), 12);
        assert_eq!(seoul.hour(), 15);
        assert_eq!(seoul.minute(), 30);
    }

    #[test]
    fn lock_path_uses_cache_file_name_with_lock_suffix() {
        let client = KisClient {
            http: Client::builder().build().expect("client should build"),
            profile: ResolvedProfile {
                environment: Environment::Demo,
                app_key: "app".to_string(),
                app_secret: "secret".to_string(),
                base_url: "https://example.com".to_string(),
                websocket_url: None,
                account_no: None,
                account_product_code: None,
                hts_id: None,
                user_agent: "test".to_string(),
            },
            cache_path: PathBuf::from("/tmp/token-cache.json"),
        };

        assert_eq!(
            client.lock_path(),
            PathBuf::from("/tmp/token-cache.json.lock")
        );
    }
}
