mod api;
mod cli;
mod config;
mod manifest;

use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::api::{adjust_tr_id, ApiCallResponse, ApiRequest, KisClient};
use crate::cli::Environment;
use crate::manifest::{
    display_command_name, load_manifest, visible_params, ApiEntry, ApiManifest, ApiParam, TrIdSpec,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let manifest = load_manifest()?;
    let matches = build_cli(manifest).get_matches();
    let env = environment_from_matches(&matches)?;
    let config_path = matches.get_one::<String>("config").map(String::as_str);
    let compact = matches.get_flag("compact");

    match matches.subcommand() {
        Some(("config", sub_matches)) => handle_config(sub_matches, config_path, compact),
        Some(("catalog", sub_matches)) => handle_catalog(manifest, sub_matches, compact),
        Some((category_name, category_matches)) => {
            let category = manifest
                .category_by_name(category_name)
                .ok_or_else(|| anyhow!("unknown category `{category_name}`"))?;
            let (api_name, api_matches) = category_matches
                .subcommand()
                .ok_or_else(|| anyhow!("missing API command under category `{}`", category.id))?;

            let entry = manifest
                .entry_by_command(&category.id, api_name)
                .ok_or_else(|| {
                    anyhow!("unknown API command `{api_name}` under `{}`", category.id)
                })?;

            let client = build_client(config_path.map(Path::new), env)?;
            let payload = execute_manifest_api(&client, entry, api_matches)?;
            print_json(&payload, compact)
        }
        None => bail!("no command provided"),
    }
}

fn build_cli(manifest: &ApiManifest) -> Command {
    let mut command = Command::new("kis-trading-cli")
        .about("Manifest-driven CLI for Korea Investment & Securities Open API")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .after_help(top_level_after_help(manifest))
        .arg(global_env_arg())
        .arg(global_config_arg())
        .arg(global_compact_arg())
        .subcommand(config_command())
        .subcommand(catalog_command());

    for category in &manifest.categories {
        let mut category_command = Command::new(leak_string(category.id.clone()))
            .about(category.introduce.clone())
            .long_about(category_long_about(category))
            .subcommand_required(true)
            .arg_required_else_help(true);

        for entry in manifest.category_entries(&category.id) {
            let mut api_command = Command::new(leak_string(display_command_name(entry)))
                .about(entry.display_name.clone())
                .long_about(api_long_about(entry));

            for param in visible_params(entry) {
                api_command = api_command.arg(api_arg(param));
            }

            category_command = category_command.subcommand(api_command);
        }

        command = command.subcommand(category_command);
    }

    command
}

fn global_env_arg() -> Arg {
    Arg::new("env")
        .long("env")
        .global(true)
        .env("KIS_ENV")
        .default_value("demo")
        .value_parser(["real", "demo"])
        .help("KIS environment")
}

fn global_config_arg() -> Arg {
    Arg::new("config")
        .long("config")
        .global(true)
        .env("KIS_CONFIG")
        .help("Override config file path")
        .value_name("PATH")
}

fn global_compact_arg() -> Arg {
    Arg::new("compact")
        .long("compact")
        .global(true)
        .action(ArgAction::SetTrue)
        .help("Print compact JSON")
}

fn config_command() -> Command {
    Command::new("config")
        .about("Manage local CLI configuration")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("init").about("Write a config template").arg(
                Arg::new("force")
                    .long("force")
                    .action(ArgAction::SetTrue)
                    .help("Overwrite an existing config file"),
            ),
        )
        .subcommand(Command::new("path").about("Show config and cache paths"))
}

fn catalog_command() -> Command {
    Command::new("catalog")
        .about("Inspect the embedded official API catalog")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("summary").about("Show category and API counts"))
        .subcommand(Command::new("export").about("Print the embedded manifest as JSON"))
}

fn api_arg(param: &ApiParam) -> Arg {
    let mut help = param.description.clone();
    let required = if param.name == "grant_type" {
        false
    } else {
        param.required
    };

    if let Some(auto_source) = &param.auto_source {
        if !help.is_empty() {
            help.push(' ');
        }
        help.push_str(&format!("[default: config {}]", auto_source));
    } else if param.name == "grant_type" {
        if !help.is_empty() {
            help.push(' ');
        }
        help.push_str("[default: client_credentials]");
    } else if let Some(default) = default_value_string(&param.default_value) {
        if !default.is_empty() {
            if !help.is_empty() {
                help.push(' ');
            }
            help.push_str(&format!("[default: {}]", default));
        }
    }

    let mut arg = Arg::new(leak_string(param.name.clone()))
        .long(leak_string(param.cli_name.clone()))
        .value_name(leak_string(param.name.to_ascii_uppercase()))
        .help(help)
        .required(required)
        .action(ArgAction::Set);

    if should_hide_param(param) {
        arg = arg.hide(true);
    }

    arg
}

fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn should_hide_param(param: &ApiParam) -> bool {
    matches!(
        param.name.as_str(),
        "appkey" | "appsecret" | "secretkey" | "grant_type"
    )
}

fn category_long_about(category: &manifest::Category) -> String {
    let mut text = format!(
        "{}\n\nConfig file: {}\nAPI count: {}",
        category.introduce, category.config_file, category.api_count
    );

    if !category.introduce_append.trim().is_empty() {
        text.push_str("\n\n");
        text.push_str(&category.introduce_append);
    }

    text
}

fn api_long_about(entry: &ApiEntry) -> String {
    format!(
        "{} > {}\n\nAPI path: {}\nHTTP method: {}\nSource: {}\nManifest id: {}",
        entry.category_label,
        entry.display_name,
        entry.api_path,
        entry.http_method,
        entry.github_url,
        entry.id
    )
}

fn top_level_after_help(manifest: &ApiManifest) -> String {
    let mut lines = vec![
        format!(
            "Embedded official catalog: {} categories, {} APIs (source commit {})",
            manifest.category_count, manifest.api_count, manifest.source_commit
        ),
        "Use `<category> --help` to inspect API commands inside that category.".to_string(),
    ];

    for category in &manifest.categories {
        lines.push(format!("  {} ({})", category.id, category.api_count));
    }

    lines.join("\n")
}

fn handle_config(matches: &ArgMatches, config_override: Option<&str>, compact: bool) -> Result<()> {
    match matches.subcommand() {
        Some(("init", init_matches)) => {
            let paths = config::app_paths(config_override.map(Path::new))?;
            config::write_config_template(&paths.config_path, init_matches.get_flag("force"))?;
            print_json(
                &serde_json::json!({
                    "config_path": paths.config_path,
                    "cache_path": paths.cache_path,
                }),
                compact,
            )
        }
        Some(("path", _)) => {
            let paths = config::app_paths(config_override.map(Path::new))?;
            print_json(
                &serde_json::json!({
                    "config_path": paths.config_path,
                    "cache_path": paths.cache_path,
                }),
                compact,
            )
        }
        _ => bail!("unsupported config subcommand"),
    }
}

fn handle_catalog(manifest: &ApiManifest, matches: &ArgMatches, compact: bool) -> Result<()> {
    match matches.subcommand() {
        Some(("summary", _)) => print_json(
            &serde_json::json!({
                "source_commit": manifest.source_commit,
                "category_count": manifest.category_count,
                "api_count": manifest.api_count,
                "categories": manifest.category_counts(),
            }),
            compact,
        ),
        Some(("export", _)) => print_json(&serde_json::to_value(manifest)?, compact),
        _ => bail!("unsupported catalog subcommand"),
    }
}

fn build_client(config_path: Option<&Path>, env: Environment) -> Result<KisClient> {
    let paths = config::app_paths(config_path)?;
    let profile = config::load_profile(config_path, env)?;
    KisClient::new(profile, paths)
}

fn execute_manifest_api(
    client: &KisClient,
    entry: &ApiEntry,
    matches: &ArgMatches,
) -> Result<Value> {
    if entry.category_id == "auth" && entry.api_type == "auth_token" {
        let token = client.access_token(false)?;
        return Ok(serde_json::json!({
            "environment": token.environment,
            "access_token": token.access_token,
            "expires_at": token.expires_at,
            "issued_at": token.issued_at,
            "source": token.source,
            "cache_path": client.cache_path(),
        }));
    }

    if entry.category_id == "auth" && entry.api_type == "auth_ws_token" {
        return client.websocket_approval_key();
    }

    let mut pages = Vec::new();
    let mut tr_cont = String::new();
    let mut pagination_state = PaginationState::default();

    loop {
        let request = build_manifest_request(client, entry, matches, &pagination_state, &tr_cont)?;
        let response = client.send_request(request)?;
        update_pagination_state(entry, &response, &mut pagination_state);

        let current_body = response.body;
        let current_tr_cont = response.tr_cont;
        pages.push(current_body);

        if entry.pagination.is_some() && matches!(current_tr_cont.as_str(), "M" | "F") {
            tr_cont = "N".to_string();
        } else {
            break;
        }
    }

    if pages.len() == 1 {
        Ok(pages.remove(0))
    } else {
        Ok(serde_json::json!({
            "page_count": pages.len(),
            "pages": pages,
        }))
    }
}

fn build_manifest_request(
    client: &KisClient,
    entry: &ApiEntry,
    matches: &ArgMatches,
    pagination_state: &PaginationState,
    tr_cont: &str,
) -> Result<ApiRequest> {
    let (tr_id, auto_adjust_tr_id) = resolve_tr_id(client.profile().environment, entry, matches)?;
    let mut query = Vec::new();
    let mut body = Map::new();

    for field in &entry.request_fields {
        let value = resolve_request_field_value(client, entry, matches, pagination_state, field)?;
        if let Some(value) = value {
            if entry.http_method == "POST" {
                body.insert(field.request_name.clone(), Value::String(value));
            } else {
                query.push((field.request_name.clone(), value));
            }
        }
    }

    Ok(ApiRequest {
        method: http_method(entry)?,
        path: entry.api_path.clone(),
        tr_id,
        auto_adjust_tr_id,
        tr_cont: tr_cont.to_string(),
        query,
        body: if entry.http_method == "POST" {
            Some(Value::Object(body))
        } else {
            None
        },
        hashkey: entry.post_uses_hashkey,
    })
}

fn http_method(entry: &ApiEntry) -> Result<Method> {
    match entry.http_method.as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        other => Err(anyhow!(
            "unsupported HTTP method `{other}` for {}",
            entry.id
        )),
    }
}

fn resolve_tr_id(
    environment: Environment,
    entry: &ApiEntry,
    matches: &ArgMatches,
) -> Result<(Option<String>, bool)> {
    match &entry.tr_id {
        TrIdSpec::None => Ok((None, false)),
        TrIdSpec::Const { value } => Ok((Some(value.clone()), true)),
        TrIdSpec::Env { real, demo } => Ok((
            Some(match environment {
                Environment::Real => real.clone(),
                Environment::Demo => demo.clone(),
            }),
            false,
        )),
        TrIdSpec::Special { resolver } => Ok((
            Some(resolve_special_tr_id(environment, resolver, matches)?),
            false,
        )),
        TrIdSpec::Unsupported { candidates } => Err(anyhow!(
            "unsupported TR ID strategy for {}. Candidates: {}",
            entry.id,
            candidates.join(", ")
        )),
    }
}

fn resolve_special_tr_id(
    environment: Environment,
    resolver: &str,
    matches: &ArgMatches,
) -> Result<String> {
    match resolver {
        "domestic_stock.order_cash" => {
            let ord_dv = required_arg(matches, "ord_dv")?;
            match (environment, ord_dv.as_str()) {
                (Environment::Real, "sell") => Ok("TTTC0011U".to_string()),
                (Environment::Real, "buy") => Ok("TTTC0012U".to_string()),
                (Environment::Demo, "sell") => Ok("VTTC0011U".to_string()),
                (Environment::Demo, "buy") => Ok("VTTC0012U".to_string()),
                _ => Err(anyhow!("ord_dv must be `buy` or `sell`")),
            }
        }
        "domestic_stock.inquire_daily_ccld" => {
            let pd_dv = required_arg(matches, "pd_dv")?;
            match (environment, pd_dv.as_str()) {
                (Environment::Real, "before") => Ok("CTSC9215R".to_string()),
                (Environment::Real, "inner") => Ok("TTTC0081R".to_string()),
                (Environment::Demo, "before") => Ok("VTSC9215R".to_string()),
                (Environment::Demo, "inner") => Ok("VTTC0081R".to_string()),
                _ => Err(anyhow!("pd_dv must be `before` or `inner`")),
            }
        }
        "domestic_futureoption.order" => {
            let ord_dv = required_arg(matches, "ord_dv")?;
            match (environment, ord_dv.as_str()) {
                (Environment::Real, "day") => Ok("TTTO1101U".to_string()),
                (Environment::Real, "night") => Ok("STTN1101U".to_string()),
                (Environment::Demo, "day") => Ok("VTTO1101U".to_string()),
                (Environment::Demo, "night") => {
                    Err(anyhow!("demo only supports `day` for this API"))
                }
                _ => Err(anyhow!("ord_dv must be `day` or `night`")),
            }
        }
        "domestic_futureoption.order_rvsecncl" => {
            let day_dv = required_arg(matches, "day_dv")?;
            match (environment, day_dv.as_str()) {
                (Environment::Real, "day") => Ok("TTTO1103U".to_string()),
                (Environment::Real, "night") => Ok("TTTN1103U".to_string()),
                (Environment::Demo, "day") => Ok("VTTO1103U".to_string()),
                (Environment::Demo, "night") => {
                    Err(anyhow!("demo only supports `day` for this API"))
                }
                _ => Err(anyhow!("day_dv must be `day` or `night`")),
            }
        }
        "overseas_stock.order" => {
            let ord_dv = required_arg(matches, "ord_dv")?;
            let ovrs_excg_cd = required_arg(matches, "ovrs_excg_cd")?;
            let real_tr_id = match (ord_dv.as_str(), ovrs_excg_cd.as_str()) {
                ("buy", "NASD" | "NYSE" | "AMEX") => "TTTT1002U",
                ("buy", "SEHK") => "TTTS1002U",
                ("buy", "SHAA") => "TTTS0202U",
                ("buy", "SZAA") => "TTTS0305U",
                ("buy", "TKSE") => "TTTS0308U",
                ("buy", "HASE" | "VNSE") => "TTTS0311U",
                ("sell", "NASD" | "NYSE" | "AMEX") => "TTTT1006U",
                ("sell", "SEHK") => "TTTS1001U",
                ("sell", "SHAA") => "TTTS1005U",
                ("sell", "SZAA") => "TTTS0304U",
                ("sell", "TKSE") => "TTTS0307U",
                ("sell", "HASE" | "VNSE") => "TTTS0310U",
                _ => {
                    return Err(anyhow!(
                        "unsupported ord_dv/ovrs_excg_cd combination for overseas stock order"
                    ))
                }
            };
            Ok(if matches!(environment, Environment::Demo) {
                adjust_tr_id(environment, real_tr_id)
            } else {
                real_tr_id.to_string()
            })
        }
        "overseas_stock.order_resv" => {
            let ord_dv = required_arg(matches, "ord_dv")?;
            match (environment, ord_dv.as_str()) {
                (Environment::Real, "usBuy") => Ok("TTTT3014U".to_string()),
                (Environment::Real, "usSell") => Ok("TTTT3016U".to_string()),
                (Environment::Real, "asia") => Ok("TTTS3013U".to_string()),
                (Environment::Demo, "usBuy") => Ok("VTTT3014U".to_string()),
                (Environment::Demo, "usSell") => Ok("VTTT3016U".to_string()),
                (Environment::Demo, "asia") => Ok("VTTS3013U".to_string()),
                _ => Err(anyhow!("ord_dv must be `usBuy`, `usSell`, or `asia`")),
            }
        }
        _ => Err(anyhow!("unknown special TR ID resolver `{resolver}`")),
    }
}

fn resolve_request_field_value(
    client: &KisClient,
    entry: &ApiEntry,
    matches: &ArgMatches,
    pagination_state: &PaginationState,
    field: &manifest::RequestField,
) -> Result<Option<String>> {
    if let Some(source_param) = &field.source_param {
        if let Some(value) = pagination_state.get(source_param) {
            return Ok(Some(value));
        }

        if let Some(param) = find_param(entry, source_param) {
            return resolve_param_value(client, param, matches);
        }

        if let Some(value) = resolve_derived_value(entry, source_param, matches)? {
            return Ok(Some(value));
        }

        if is_pagination_seed(source_param) {
            return Ok(Some(String::new()));
        }

        return Ok(None);
    }

    if let Some(literal) = &field.literal {
        return Ok(default_value_string(&Some(literal.clone())));
    }

    Ok(None)
}

fn resolve_param_value(
    client: &KisClient,
    param: &ApiParam,
    matches: &ArgMatches,
) -> Result<Option<String>> {
    if let Some(value) = matches.get_one::<String>(&param.name) {
        return Ok(Some(value.clone()));
    }

    if let Some(auto_source) = &param.auto_source {
        if let Some(value) = auto_value(client, auto_source) {
            return Ok(Some(value));
        }
    }

    if let Some(default) = default_value_string(&param.default_value) {
        return Ok(Some(default));
    }

    if param.required {
        return Err(anyhow!("missing required argument `--{}`", param.cli_name));
    }

    Ok(None)
}

fn resolve_derived_value(
    entry: &ApiEntry,
    source_param: &str,
    matches: &ArgMatches,
) -> Result<Option<String>> {
    match (entry.id.as_str(), source_param) {
        ("overseas-stock.order", "sll_type") => {
            let ord_dv = required_arg(matches, "ord_dv")?;
            match ord_dv.as_str() {
                "buy" => Ok(Some(String::new())),
                "sell" => Ok(Some("00".to_string())),
                _ => Err(anyhow!("ord_dv must be `buy` or `sell`")),
            }
        }
        _ => Ok(None),
    }
}

fn auto_value(client: &KisClient, auto_source: &str) -> Option<String> {
    match auto_source {
        "account_no" => client.profile().account_no.clone(),
        "account_product_code" => client.profile().account_product_code.clone(),
        "app_key" => Some(client.profile().app_key.clone()),
        "app_secret" => Some(client.profile().app_secret.clone()),
        "hts_id" => client.profile().hts_id.clone(),
        _ => None,
    }
}

fn find_param<'a>(entry: &'a ApiEntry, name: &str) -> Option<&'a ApiParam> {
    entry
        .params
        .iter()
        .find(|param| param.name.eq_ignore_ascii_case(name))
}

fn required_arg(matches: &ArgMatches, name: &str) -> Result<String> {
    matches
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| anyhow!("missing required argument `--{}`", name.replace('_', "-")))
}

fn default_value_string(value: &Option<Value>) -> Option<String> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        Some(Value::Bool(boolean)) => Some(boolean.to_string()),
        Some(other) => Some(other.to_string()),
    }
}

#[derive(Default)]
struct PaginationState {
    fk_value: Option<String>,
    nk_value: Option<String>,
}

impl PaginationState {
    fn get(&self, source_param: &str) -> Option<String> {
        match source_param.to_ascii_uppercase().as_str() {
            "FK100" | "FK200" | "FK50" | "CTX_AREA_FK100" | "CTX_AREA_FK200" | "CTX_AREA_FK50"
            | "CTX_AREA_FK" => self.fk_value.clone(),
            "NK100" | "NK200" | "NK50" | "NK30" | "CTX_AREA_NK100" | "CTX_AREA_NK200"
            | "CTX_AREA_NK50" | "CTX_AREA_NK30" | "CTX_AREA_NK" => self.nk_value.clone(),
            _ => None,
        }
    }
}

fn is_pagination_seed(source_param: &str) -> bool {
    matches!(
        source_param.to_ascii_uppercase().as_str(),
        "FK100" | "FK200" | "FK50" | "NK100" | "NK200" | "NK50" | "NK30" | "CTS" | "SEQ"
    )
}

fn update_pagination_state(
    entry: &ApiEntry,
    response: &ApiCallResponse,
    state: &mut PaginationState,
) {
    let Some(pagination) = &entry.pagination else {
        return;
    };

    if let Some(field_name) = &pagination.ctx_fk_field {
        state.fk_value = response
            .body
            .get(field_name.to_ascii_lowercase())
            .and_then(Value::as_str)
            .map(ToString::to_string);
    }

    if let Some(field_name) = &pagination.ctx_nk_field {
        state.nk_value = response
            .body
            .get(field_name.to_ascii_lowercase())
            .and_then(Value::as_str)
            .map(ToString::to_string);
    }
}

fn environment_from_matches(matches: &ArgMatches) -> Result<Environment> {
    match matches.get_one::<String>("env").map(String::as_str) {
        Some("real") => Ok(Environment::Real),
        Some("demo") => Ok(Environment::Demo),
        Some(other) => Err(anyhow!("unsupported environment `{other}`")),
        None => Ok(Environment::Demo),
    }
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
    fn default_value_string_converts_scalar_values() {
        assert_eq!(
            default_value_string(&Some(Value::String("abc".to_string()))),
            Some("abc".to_string())
        );
        assert_eq!(
            default_value_string(&Some(Value::Number(serde_json::Number::from(10)))),
            Some("10".to_string())
        );
    }

    #[test]
    fn pagination_seed_detection_covers_context_aliases() {
        assert!(is_pagination_seed("FK100"));
        assert!(is_pagination_seed("NK30"));
        assert!(is_pagination_seed("CTS"));
        assert!(!is_pagination_seed("PDNO"));
    }
}
