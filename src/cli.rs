use std::fs;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};

use crate::config::Config;
use crate::errors::TexError;
use crate::interactive::{confirm_create_dir, confirm_overwrite, run_init_prompts};
use crate::paths::config_file_path;

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
    let config_path = config_file_path()?;

    if config_path.exists() {
        let ok = confirm_overwrite(&config_path)?;
        if !ok {
            return Err(anyhow::Error::new(TexError::UserAborted));
        }
    }

    let answers = run_init_prompts()?;

    ensure_dir(&answers.templates_dir)?;
    ensure_dir(&answers.output_dir)?;

    check_engine(&answers.engine);

    let cfg = Config::new_from_prompts(
        answers.templates_dir,
        answers.output_dir,
        answers.engine,
    );
    cfg.save_atomic(&config_path)?;

    println!("Config gravado em: {}", config_path.display());
    Ok(())
}

fn ensure_dir(path: &std::path::Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let ok = confirm_create_dir(path)?;
    if !ok {
        return Err(anyhow::Error::new(TexError::UserAborted));
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn check_engine(engine: &str) {
    if which::which(engine).is_err() {
        eprintln!(
            "Aviso: '{engine}' não foi encontrado no PATH. A preferência foi salva, instale o binário depois."
        );
    }
    if engine != "tectonic" {
        eprintln!(
            "Aviso: engine '{engine}' ainda não é executada pelo Tex nesta versão. A preferência foi salva."
        );
    }
}

pub fn handle_config_show(_format: ShowFormat) -> Result<()> {
    Err(anyhow!("handle_config_show: não implementado"))
}

pub fn handle_config_set(_key: String, _value: String) -> Result<()> {
    Err(anyhow!("handle_config_set: não implementado"))
}
