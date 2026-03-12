mod api;
mod cli;
mod config;

use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reqwest::Method;
use serde_json::{Map, Value};

use crate::api::{ApiRequest, KisClient};
use crate::cli::{
    ApiCommand, ApiGetArgs, ApiPostArgs, AuthCommand, Cli, Commands, ConfigCommand, QuoteCommand,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Cli {
        env,
        config,
        compact,
        command,
    } = Cli::parse();

    match command {
        Commands::Config { command } => match command {
            ConfigCommand::Init(args) => {
                let paths = config::app_paths(config.as_deref())?;
                config::write_config_template(&paths.config_path, args.force)?;

                let payload = serde_json::json!({
                    "config_path": paths.config_path,
                    "cache_path": paths.cache_path,
                });
                print_json(&payload, compact)
            }
            ConfigCommand::Path => {
                let paths = config::app_paths(config.as_deref())?;
                let payload = serde_json::json!({
                    "config_path": paths.config_path,
                    "cache_path": paths.cache_path,
                });
                print_json(&payload, compact)
            }
        },
        Commands::Auth { command } => {
            let client = build_client(config.as_deref(), env)?;

            match command {
                AuthCommand::Token(args) => {
                    let token = client.access_token(args.force_refresh)?;
                    if args.only_token {
                        println!("{}", token.access_token);
                        Ok(())
                    } else {
                        let payload = serde_json::json!({
                            "environment": token.environment,
                            "access_token": token.access_token,
                            "expires_at": token.expires_at,
                            "issued_at": token.issued_at,
                            "source": token.source,
                            "cache_path": client.cache_path(),
                        });
                        print_json(&payload, compact)
                    }
                }
            }
        }
        Commands::Quote { command } => {
            let client = build_client(config.as_deref(), env)?;

            match command {
                QuoteCommand::DomesticPrice(args) => {
                    let response = client.send_request(ApiRequest {
                        method: Method::GET,
                        path: "/uapi/domestic-stock/v1/quotations/inquire-price".to_string(),
                        tr_id: Some("FHKST01010100".to_string()),
                        tr_cont: String::new(),
                        query: vec![
                            ("FID_COND_MRKT_DIV_CODE".to_string(), args.market),
                            ("FID_INPUT_ISCD".to_string(), args.symbol),
                        ],
                        body: None,
                        hashkey: false,
                    })?;

                    let payload = if args.full_response {
                        response
                    } else {
                        response.get("output").cloned().unwrap_or(response)
                    };

                    let payload = select_path(payload, args.select.as_deref())?;
                    print_json(&payload, compact)
                }
            }
        }
        Commands::Api { command } => {
            let client = build_client(config.as_deref(), env)?;

            match command {
                ApiCommand::Get(args) => run_api_get(&client, args, compact),
                ApiCommand::Post(args) => run_api_post(&client, args, compact),
            }
        }
    }
}

fn build_client(config_path: Option<&Path>, env: cli::Environment) -> Result<KisClient> {
    let paths = config::app_paths(config_path)?;
    let profile = config::load_profile(config_path, env)?;
    let _ = (
        &profile.websocket_url,
        &profile.account_no,
        &profile.account_product_code,
        &profile.hts_id,
    );

    KisClient::new(profile, paths)
}

fn run_api_get(client: &KisClient, args: ApiGetArgs, compact: bool) -> Result<()> {
    let response = client.send_request(ApiRequest {
        method: Method::GET,
        path: args.path,
        tr_id: Some(args.tr_id),
        tr_cont: args.tr_cont,
        query: args.query,
        body: None,
        hashkey: false,
    })?;

    let response = select_path(response, args.select.as_deref())?;
    print_json(&response, compact)
}

fn run_api_post(client: &KisClient, args: ApiPostArgs, compact: bool) -> Result<()> {
    let body = load_body(args.body.as_deref(), args.body_file.as_deref(), &args.field)?;
    let response = client.send_request(ApiRequest {
        method: Method::POST,
        path: args.path,
        tr_id: Some(args.tr_id),
        tr_cont: args.tr_cont,
        query: args.query,
        body: Some(body),
        hashkey: args.hashkey,
    })?;

    let response = select_path(response, args.select.as_deref())?;
    print_json(&response, compact)
}

fn load_body(
    body: Option<&str>,
    body_file: Option<&Path>,
    fields: &[(String, String)],
) -> Result<Value> {
    if let Some(raw) = body {
        return serde_json::from_str(raw).context("failed to parse --body as JSON");
    }

    if let Some(path) = body_file {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read body file {}", path.display()))?;
        return serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse JSON body file {}", path.display()));
    }

    let mut object = Map::new();
    for (key, value) in fields {
        object.insert(key.clone(), Value::String(value.clone()));
    }

    Ok(Value::Object(object))
}

fn select_path(value: Value, path: Option<&str>) -> Result<Value> {
    let Some(path) = path else {
        return Ok(value);
    };

    let mut current = &value;
    for segment in path.split('.') {
        current = if let Ok(index) = segment.parse::<usize>() {
            current
                .get(index)
                .ok_or_else(|| anyhow!("path segment `{segment}` not found"))?
        } else {
            current
                .get(segment)
                .ok_or_else(|| anyhow!("path segment `{segment}` not found"))?
        };
    }

    Ok(current.clone())
}

fn print_json(value: &Value, compact: bool) -> Result<()> {
    let rendered = if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .context("failed to serialize JSON output")?;

    println!("{rendered}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_path_reads_nested_object_field() {
        let value = serde_json::json!({
            "output": {
                "stck_prpr": "71500",
                "nested": {
                    "value": "ok"
                }
            }
        });

        let selected = select_path(value, Some("output.nested.value")).expect("path should exist");
        assert_eq!(selected, Value::String("ok".to_string()));
    }

    #[test]
    fn load_body_builds_object_from_fields() {
        let body = load_body(
            None,
            None,
            &[
                ("APPKEY".to_string(), "key".to_string()),
                ("APPSECRET".to_string(), "secret".to_string()),
            ],
        )
        .expect("body should build");

        assert_eq!(body["APPKEY"], Value::String("key".to_string()));
        assert_eq!(body["APPSECRET"], Value::String("secret".to_string()));
    }
}
