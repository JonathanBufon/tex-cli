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
    /// Cria o config em ~/.config/tex/config.toml (interativo ou via flags).
    Init(InitArgs),

    /// Inspeciona ou altera o config atual.
    #[command(subcommand)]
    Config(ConfigCmd),
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Diretório de templates LaTeX (pula o prompt correspondente).
    #[arg(short = 't', long)]
    pub templates_dir: Option<String>,

    /// Diretório padrão de saída dos PDFs (pula o prompt correspondente).
    #[arg(short = 'o', long)]
    pub output_dir: Option<String>,

    /// Nome do engine LaTeX (`tectonic`, `latexmk`, `pdflatex`, `xelatex`, `lualatex`).
    #[arg(short = 'e', long)]
    pub engine: Option<String>,

    /// Cria diretórios ausentes sem pedir confirmação.
    #[arg(long)]
    pub create_dirs: bool,

    /// Sobrescreve config existente sem pedir confirmação.
    #[arg(long)]
    pub force: bool,
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

pub fn handle_init(args: InitArgs) -> Result<()> {
    let config_path = config_file_path()?;

    if config_path.exists() {
        let ok = if args.force {
            true
        } else {
            confirm_overwrite(&config_path)?
        };
        if !ok {
            return Err(anyhow::Error::new(TexError::UserAborted));
        }
    }

    let (templates_dir, output_dir, engine) = resolve_answers(&args)?;

    ensure_dir(&templates_dir, args.create_dirs)?;
    ensure_dir(&output_dir, args.create_dirs)?;

    check_engine(&engine);

    let cfg = Config::new_from_prompts(templates_dir, output_dir, engine);
    cfg.save_atomic(&config_path)?;

    println!("Config gravado em: {}", config_path.display());
    Ok(())
}

fn resolve_answers(
    args: &InitArgs,
) -> Result<(std::path::PathBuf, std::path::PathBuf, String)> {
    let fully_specified = args.templates_dir.is_some()
        && args.output_dir.is_some()
        && args.engine.is_some();

    if fully_specified {
        let templates = crate::paths::expand_user_path(args.templates_dir.as_ref().unwrap())?;
        let output = crate::paths::expand_user_path(args.output_dir.as_ref().unwrap())?;
        let engine = args.engine.as_ref().unwrap().clone();
        return Ok((templates, output, engine));
    }

    let answers = run_init_prompts()?;
    Ok((answers.templates_dir, answers.output_dir, answers.engine))
}

fn ensure_dir(path: &std::path::Path, create_flag: bool) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let ok = if create_flag {
        true
    } else {
        confirm_create_dir(path)?
    };
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
