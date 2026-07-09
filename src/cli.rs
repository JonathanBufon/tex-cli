use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "tex-cli",
    about = "cli tool to compile and export tex files to PDF",
    version
)]
pub struct Cli {
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        global = true,
        help = "Aumenta o nível de log em stderr (repetível: -v, -vv, -vvv)."
    )]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Cria o config em ~/.config/tex/config.toml de forma interativa.
    Init,

    /// Inspeciona ou altera o config atual.
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Exibe o config atual (humano/json/toml).
    Show {
        #[arg(long, default_value = "humano")]
        format: ShowFormat,
    },

    /// Altera um único valor no config (`<chave-dotted> <valor>`).
    Set { key: String, value: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShowFormat {
    Humano,
    Json,
    Toml,
}

pub fn handle_init() -> Result<()> {
    Err(anyhow!("handle_init: não implementado"))
}

pub fn handle_config_show(_format: ShowFormat) -> Result<()> {
    Err(anyhow!("handle_config_show: não implementado"))
}

pub fn handle_config_set(_key: String, _value: String) -> Result<()> {
    Err(anyhow!("handle_config_set: não implementado"))
}
