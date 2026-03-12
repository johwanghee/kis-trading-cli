use std::fmt;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(name = "kis-trading-cli")]
#[command(about = "Cross-platform CLI for Korea Investment & Securities Open API")]
#[command(version)]
pub struct Cli {
    #[arg(long, global = true, env = "KIS_ENV", value_enum, default_value_t = Environment::Demo)]
    pub env: Environment,

    #[arg(long, global = true, env = "KIS_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(long, global = true)]
    pub compact: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Quote {
        #[command(subcommand)]
        command: QuoteCommand,
    },
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Real,
    Demo,
}

impl Environment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Real => "real",
            Self::Demo => "demo",
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    Init(ConfigInitArgs),
    Path,
}

#[derive(Debug, Args)]
pub struct ConfigInitArgs {
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Token(TokenArgs),
}

#[derive(Debug, Args)]
pub struct TokenArgs {
    #[arg(long)]
    pub force_refresh: bool,

    #[arg(long)]
    pub only_token: bool,
}

#[derive(Debug, Subcommand)]
pub enum QuoteCommand {
    DomesticPrice(DomesticPriceArgs),
}

#[derive(Debug, Args)]
pub struct DomesticPriceArgs {
    #[arg(long)]
    pub symbol: String,

    #[arg(long, default_value = "J")]
    pub market: String,

    #[arg(long)]
    pub select: Option<String>,

    #[arg(long)]
    pub full_response: bool,
}

#[derive(Debug, Subcommand)]
pub enum ApiCommand {
    Get(ApiGetArgs),
    Post(ApiPostArgs),
}

#[derive(Debug, Args)]
pub struct ApiGetArgs {
    #[arg(long)]
    pub path: String,

    #[arg(long)]
    pub tr_id: String,

    #[arg(long = "query", value_parser = parse_key_val)]
    pub query: Vec<(String, String)>,

    #[arg(long, default_value = "")]
    pub tr_cont: String,

    #[arg(long)]
    pub select: Option<String>,
}

#[derive(Debug, Args)]
pub struct ApiPostArgs {
    #[arg(long)]
    pub path: String,

    #[arg(long)]
    pub tr_id: String,

    #[arg(long = "query", value_parser = parse_key_val)]
    pub query: Vec<(String, String)>,

    #[arg(long, conflicts_with_all = ["body_file", "field"])]
    pub body: Option<String>,

    #[arg(long, conflicts_with_all = ["body", "field"])]
    pub body_file: Option<PathBuf>,

    #[arg(long = "field", value_parser = parse_key_val, conflicts_with_all = ["body", "body_file"])]
    pub field: Vec<(String, String)>,

    #[arg(long, default_value = "")]
    pub tr_cont: String,

    #[arg(long)]
    pub hashkey: bool,

    #[arg(long)]
    pub select: Option<String>,
}

fn parse_key_val(input: &str) -> Result<(String, String), String> {
    let Some((key, value)) = input.split_once('=') else {
        return Err(format!("expected KEY=VALUE, got: {input}"));
    };

    if key.trim().is_empty() {
        return Err("KEY cannot be empty".to_string());
    }

    Ok((key.trim().to_string(), value.to_string()))
}
