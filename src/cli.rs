use std::fs;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};

use std::str::FromStr;

use crate::config::{render_humano, Config, ConfigKey};
use crate::errors::TexError;
use crate::interactive::{confirm_create_dir, confirm_overwrite, run_init_prompts};
use crate::paths::config_file_path;
use crate::templates::{list_templates, read_template, render_template_list_humano};

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

    /// Gerencia templates LaTeX em `paths.templates_dir`.
    Templates(TemplatesArgs),
}

#[derive(Debug, clap::Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    pub command: Option<TemplatesCmd>,
}

#[derive(Debug, Subcommand)]
pub enum TemplatesCmd {
    /// Lista os arquivos `.tex` em `paths.templates_dir`.
    List {
        #[arg(long, default_value = "humano")]
        format: ShowFormat,
    },

    /// Imprime o conteúdo bruto de um template no stdout.
    Show { name: String },

    /// Adiciona um novo template copiando um arquivo do host.
    Add(AddTemplateArgs),

    /// Remove um template do `paths.templates_dir`.
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, clap::Args)]
pub struct AddTemplateArgs {
    /// Caminho do arquivo `.tex` no host.
    pub source_path: std::path::PathBuf,

    /// Nome final do template (default = basename do arquivo, sem `.tex`).
    #[arg(short = 'n', long)]
    pub name: Option<String>,

    /// Sobrescreve template existente sem pedir confirmação.
    #[arg(long)]
    pub force: bool,
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

fn resolve_answers(args: &InitArgs) -> Result<(std::path::PathBuf, std::path::PathBuf, String)> {
    let fully_specified =
        args.templates_dir.is_some() && args.output_dir.is_some() && args.engine.is_some();

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

pub fn handle_config_show(format: ShowFormat) -> Result<()> {
    let path = config_file_path()?;
    let cfg = Config::load(&path)?;

    let rendered = match format {
        ShowFormat::Humano => render_humano(&cfg),
        ShowFormat::Json => {
            serde_json::to_string_pretty(&cfg)
                .map_err(|e| anyhow!("falha ao serializar JSON: {e}"))?
                + "\n"
        }
        ShowFormat::Toml => {
            toml::to_string_pretty(&cfg).map_err(|e| anyhow!("falha ao serializar TOML: {e}"))?
        }
    };

    print!("{rendered}");
    Ok(())
}

pub fn handle_templates_list(format: ShowFormat) -> Result<()> {
    let path = config_file_path()?;
    let cfg = Config::load(&path)?;
    let dir = &cfg.paths.templates_dir;
    let templates = list_templates(dir)?;

    match format {
        ShowFormat::Humano => {
            print!("{}", render_template_list_humano(dir, &templates));
        }
        ShowFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&templates)?);
        }
        ShowFormat::Toml => {
            return Err(anyhow!(
                "--format=toml não é suportado em templates list. Use humano ou json."
            ));
        }
    }
    Ok(())
}

pub fn handle_templates_show(name: String) -> Result<()> {
    use std::io::Write;

    let path = config_file_path()?;
    let cfg = Config::load(&path)?;
    let bytes = read_template(&cfg.paths.templates_dir, &name)?;
    std::io::stdout().write_all(&bytes)?;
    Ok(())
}

pub fn handle_templates_add(_args: AddTemplateArgs) -> Result<()> {
    Err(anyhow!("handle_templates_add: não implementado"))
}

pub fn handle_templates_remove(_name: String, _force: bool) -> Result<()> {
    Err(anyhow!("handle_templates_remove: não implementado"))
}

pub fn handle_templates_menu() -> Result<()> {
    Err(anyhow!("handle_templates_menu: não implementado"))
}

pub fn handle_config_set(key: String, value: String) -> Result<()> {
    let path = config_file_path()?;
    let mut cfg = Config::load(&path)?;
    let parsed_key = ConfigKey::from_str(&key)?;
    let change = cfg.apply(parsed_key, &value)?;

    for warning in &change.warnings {
        eprintln!("{warning}");
    }

    cfg.save_atomic(&path)?;

    println!(
        "Config atualizado: {} = {}",
        change.key, change.normalized_value
    );
    Ok(())
}
