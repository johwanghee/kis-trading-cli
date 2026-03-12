use anyhow::Error as AnyError;
use clap::error::ErrorKind;
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::Value;

use crate::config::PlaintextSecretError;

pub const PROGRAM_ERROR_EXIT_CODE: i32 = 2;
pub const API_ERROR_EXIT_CODE: i32 = 3;
const RESPONSE_EXCERPT_LIMIT: usize = 1_000;

#[derive(Debug)]
pub struct KisApiError {
    pub operation: String,
    pub path: String,
    pub http_status: Option<u16>,
    pub rt_cd: Option<String>,
    pub msg_cd: Option<String>,
    pub msg1: Option<String>,
    pub response_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub ok: bool,
    pub error_type: &'static str,
    pub exit_code: i32,
    pub message: String,
    pub llm_hint: LlmHint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_error: Option<ApiErrorPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_error: Option<ProgramErrorPayload>,
    pub causes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmHint {
    pub summary: String,
    pub retryable: bool,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorPayload {
    pub operation: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rt_cd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_cd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgramErrorPayload {
    pub category: &'static str,
    pub retryable: bool,
    pub detail: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub plaintext_secrets: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggested_commands: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ProgramClassification {
    category: &'static str,
    retryable: bool,
    next_action: &'static str,
}

impl KisApiError {
    pub fn from_http_response(
        operation: impl Into<String>,
        path: impl Into<String>,
        status: StatusCode,
        response_text: &str,
    ) -> Self {
        let (rt_cd, msg_cd, msg1) = parse_kis_error_fields_from_text(response_text);
        Self {
            operation: operation.into(),
            path: path.into(),
            http_status: Some(status.as_u16()),
            rt_cd,
            msg_cd,
            msg1,
            response_excerpt: Some(response_excerpt(response_text)),
        }
    }

    pub fn from_response_value(
        operation: impl Into<String>,
        path: impl Into<String>,
        http_status: Option<u16>,
        value: &Value,
    ) -> Self {
        let rt_cd = value
            .get("rt_cd")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let msg_cd = value
            .get("msg_cd")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let msg1 = value
            .get("msg1")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let response_excerpt = serde_json::to_string(value)
            .ok()
            .map(|text| response_excerpt(&text));

        Self {
            operation: operation.into(),
            path: path.into(),
            http_status,
            rt_cd,
            msg_cd,
            msg1,
            response_excerpt,
        }
    }

    pub fn retryable(&self) -> bool {
        self.http_status
            .map(|status| status == 429 || status >= 500)
            .unwrap_or(false)
    }
}

impl std::fmt::Display for KisApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(msg_cd), Some(msg1)) = (&self.msg_cd, &self.msg1) {
            write!(
                f,
                "KIS API error {msg_cd} for {} {}: {msg1}",
                self.operation, self.path
            )
        } else if let Some(status) = self.http_status {
            write!(
                f,
                "KIS HTTP error {status} for {} {}",
                self.operation, self.path
            )
        } else {
            write!(f, "KIS API error for {} {}", self.operation, self.path)
        }
    }
}

impl std::error::Error for KisApiError {}

pub fn error_report_from_anyhow(error: &AnyError) -> ErrorEnvelope {
    if let Some(api_error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<KisApiError>())
    {
        return api_error_report(api_error, error.chain().map(ToString::to_string).collect());
    }

    if let Some(plaintext_error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<PlaintextSecretError>())
    {
        return plaintext_secret_report(
            plaintext_error,
            error.chain().map(ToString::to_string).collect(),
        );
    }

    let detail = error.to_string();
    let classification = classify_program_error(error, &detail);

    ErrorEnvelope {
        ok: false,
        error_type: "program_error",
        exit_code: PROGRAM_ERROR_EXIT_CODE,
        message: detail.clone(),
        llm_hint: LlmHint {
            summary: program_summary(classification.category, &detail),
            retryable: classification.retryable,
            next_action: classification.next_action.to_string(),
        },
        api_error: None,
        program_error: Some(ProgramErrorPayload {
            category: classification.category,
            retryable: classification.retryable,
            detail,
            plaintext_secrets: Vec::new(),
            suggested_commands: Vec::new(),
        }),
        causes: error.chain().map(ToString::to_string).collect(),
    }
}

pub fn error_report_from_clap(error: &clap::Error) -> ErrorEnvelope {
    let rendered = error.to_string();
    let detail = rendered.trim().to_string();
    let next_action = if matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        "No action required."
    } else {
        "Read the command help shown in `detail`, then fix the CLI arguments and retry."
    };

    ErrorEnvelope {
        ok: false,
        error_type: "program_error",
        exit_code: PROGRAM_ERROR_EXIT_CODE,
        message: detail.clone(),
        llm_hint: LlmHint {
            summary: "The CLI arguments did not match the command definition.".to_string(),
            retryable: false,
            next_action: next_action.to_string(),
        },
        api_error: None,
        program_error: Some(ProgramErrorPayload {
            category: "invalid_input",
            retryable: false,
            detail,
            plaintext_secrets: Vec::new(),
            suggested_commands: Vec::new(),
        }),
        causes: vec![error.kind().to_string()],
    }
}

pub fn render_error_report(report: &ErrorEnvelope, compact: bool) -> String {
    let rendered = if compact {
        serde_json::to_string(report)
    } else {
        serde_json::to_string_pretty(report)
    };

    rendered.unwrap_or_else(|serialization_error| {
        format!(
            "{{\"ok\":false,\"error_type\":\"program_error\",\"exit_code\":{PROGRAM_ERROR_EXIT_CODE},\"message\":\"failed to serialize error report: {serialization_error}\"}}"
        )
    })
}

fn api_error_report(api_error: &KisApiError, causes: Vec<String>) -> ErrorEnvelope {
    let retryable = api_error.retryable();
    let summary = if let (Some(msg_cd), Some(msg1)) = (&api_error.msg_cd, &api_error.msg1) {
        format!(
            "KIS rejected the request for {} with msg_cd={} and msg1={}.",
            api_error.path, msg_cd, msg1
        )
    } else if let Some(status) = api_error.http_status {
        format!(
            "KIS returned HTTP {} for {} {}.",
            status, api_error.operation, api_error.path
        )
    } else {
        format!(
            "KIS returned an API-side failure for {} {}.",
            api_error.operation, api_error.path
        )
    };

    let next_action = if retryable {
        "This looks retryable. Retry after a short delay and check KIS service availability or rate limits.".to_string()
    } else if api_error.msg_cd.is_some() || api_error.msg1.is_some() {
        "Inspect `msg_cd`, `msg1`, auth state, account state, and request parameters before retrying.".to_string()
    } else {
        "Inspect the HTTP status and response excerpt, then adjust the request or authentication before retrying.".to_string()
    };

    ErrorEnvelope {
        ok: false,
        error_type: "api_error",
        exit_code: API_ERROR_EXIT_CODE,
        message: api_error.to_string(),
        llm_hint: LlmHint {
            summary,
            retryable,
            next_action,
        },
        api_error: Some(ApiErrorPayload {
            operation: api_error.operation.clone(),
            path: api_error.path.clone(),
            http_status: api_error.http_status,
            rt_cd: api_error.rt_cd.clone(),
            msg_cd: api_error.msg_cd.clone(),
            msg1: api_error.msg1.clone(),
            response_excerpt: api_error.response_excerpt.clone(),
        }),
        program_error: None,
        causes,
    }
}

fn plaintext_secret_report(
    plaintext_error: &PlaintextSecretError,
    causes: Vec<String>,
) -> ErrorEnvelope {
    let suggested_commands = vec![
        "kis-trading-cli config key status --compact".to_string(),
        "kis-trading-cli config seal".to_string(),
    ];

    ErrorEnvelope {
        ok: false,
        error_type: "program_error",
        exit_code: PROGRAM_ERROR_EXIT_CODE,
        message: plaintext_error.to_string(),
        llm_hint: LlmHint {
            summary: format!(
                "Sensitive config values are still stored in plaintext in {}.",
                plaintext_error.config_path.display()
            ),
            retryable: false,
            next_action: "Run `kis-trading-cli config key status --compact` to inspect plaintext fields, then run `kis-trading-cli config seal` or `kis-trading-cli config set-secret ...` to encrypt them.".to_string(),
        },
        api_error: None,
        program_error: Some(ProgramErrorPayload {
            category: "plaintext_secret_detected",
            retryable: false,
            detail: plaintext_error.to_string(),
            plaintext_secrets: plaintext_error.plaintext_fields.clone(),
            suggested_commands,
        }),
        causes,
    }
}

fn parse_kis_error_fields_from_text(
    text: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    serde_json::from_str::<Value>(text)
        .ok()
        .map(|value| {
            (
                value
                    .get("rt_cd")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                value
                    .get("msg_cd")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                value
                    .get("msg1")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            )
        })
        .unwrap_or((None, None, None))
}

fn response_excerpt(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= RESPONSE_EXCERPT_LIMIT {
        return trimmed.to_string();
    }

    let mut end = RESPONSE_EXCERPT_LIMIT;
    while !trimmed.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!("{}...", &trimmed[..end])
}

fn classify_program_error(error: &AnyError, detail: &str) -> ProgramClassification {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<reqwest::Error>().is_some())
    {
        return ProgramClassification {
            category: "network_error",
            retryable: true,
            next_action: "Check network connectivity, DNS, proxy settings, base_url, or KIS availability, then retry.",
        };
    }

    let lowered = detail.to_ascii_lowercase();
    if lowered.contains("missing required argument")
        || lowered.contains("unsupported environment")
        || lowered.contains("unsupported secret field")
        || lowered.contains("secret value cannot be empty")
        || lowered.contains("provide `--value` or `--stdin`")
        || lowered.contains("unknown category")
        || lowered.contains("unknown api command")
        || lowered.contains("no command provided")
        || lowered.contains("ord_dv must be")
        || lowered.contains("pd_dv must be")
        || lowered.contains("day_dv must be")
    {
        return ProgramClassification {
            category: "invalid_input",
            retryable: false,
            next_action: "Read the command help and fix the CLI arguments or parameter values before retrying.",
        };
    }

    if lowered.contains("config")
        || lowered.contains("key file")
        || lowered.contains("app_key")
        || lowered.contains("app_secret")
        || lowered.contains("account_no")
        || lowered.contains("hts_id")
        || lowered.contains("failed to decrypt")
        || lowered.contains("missing config encryption key")
    {
        return ProgramClassification {
            category: "configuration_error",
            retryable: false,
            next_action: "Fix the config file, key file, or environment overrides, then retry.",
        };
    }

    if lowered.contains("failed to read")
        || lowered.contains("failed to write")
        || lowered.contains("failed to create")
        || lowered.contains("failed to open")
        || lowered.contains("failed to lock")
        || lowered.contains("permission")
    {
        return ProgramClassification {
            category: "local_io_error",
            retryable: false,
            next_action: "Check local file paths, permissions, and disk state, then retry.",
        };
    }

    ProgramClassification {
        category: "runtime_error",
        retryable: false,
        next_action:
            "Inspect the error chain and update the CLI or local environment before retrying.",
    }
}

fn program_summary(category: &str, detail: &str) -> String {
    match category {
        "invalid_input" => "The CLI command was syntactically valid enough to run, but the provided arguments or values were not acceptable.".to_string(),
        "configuration_error" => "The CLI could not proceed because the local configuration or key material is invalid or incomplete.".to_string(),
        "network_error" => "The CLI could not reach KIS successfully because of a transport-level failure.".to_string(),
        "local_io_error" => "The CLI failed while reading or writing local files needed for execution.".to_string(),
        _ => format!("The CLI failed before a successful KIS API response was produced: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn api_error_is_rendered_with_api_classification() {
        let error = KisApiError::from_http_response(
            "rest_request",
            "/uapi/test",
            StatusCode::BAD_REQUEST,
            r#"{"rt_cd":"2","msg_cd":"OPSQ0001","msg1":"bad request"}"#,
        );

        let report = error_report_from_anyhow(&AnyError::new(error));
        assert_eq!(report.error_type, "api_error");
        assert_eq!(report.exit_code, API_ERROR_EXIT_CODE);
        assert_eq!(
            report
                .api_error
                .as_ref()
                .and_then(|payload| payload.msg_cd.as_deref()),
            Some("OPSQ0001")
        );
    }

    #[test]
    fn invalid_input_is_classified_as_program_error() {
        let report = error_report_from_anyhow(&anyhow!("missing required argument `--symbol`"));
        assert_eq!(report.error_type, "program_error");
        assert_eq!(report.exit_code, PROGRAM_ERROR_EXIT_CODE);
        assert_eq!(
            report
                .program_error
                .as_ref()
                .map(|payload| payload.category),
            Some("invalid_input")
        );
    }

    #[test]
    fn plaintext_secret_error_includes_fix_commands() {
        let report = error_report_from_anyhow(&AnyError::new(PlaintextSecretError {
            config_path: PathBuf::from("/tmp/config.toml"),
            plaintext_fields: vec![
                "profiles.demo.app_key".to_string(),
                "profiles.demo.app_secret".to_string(),
            ],
        }));

        assert_eq!(report.error_type, "program_error");
        assert_eq!(
            report
                .program_error
                .as_ref()
                .map(|payload| payload.category),
            Some("plaintext_secret_detected")
        );
        assert_eq!(
            report
                .program_error
                .as_ref()
                .map(|payload| payload.plaintext_secrets.len()),
            Some(2)
        );
        assert!(report
            .program_error
            .as_ref()
            .map(|payload| payload
                .suggested_commands
                .contains(&"kis-trading-cli config seal".to_string()))
            .unwrap_or(false));
    }
}
