use std::fs;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};

use std::str::FromStr;

use crate::config::{render_humano, Config, ConfigKey};
use crate::errors::TexError;
use crate::interactive::{
    confirm_create_dir, confirm_dry_run, confirm_overwrite, confirm_overwrite_template,
    confirm_remove_template, prompt_json_source, prompt_source_path, prompt_template_name,
    run_init_prompts, template_menu, TemplateMenuAction,
};
use crate::paths::config_file_path;
use crate::templates::{
    add_template, list_templates, read_template, remove_template, render_template_list_humano,
    resolve_template,
};

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

    /// Renderiza um template com dados JSON, produzindo um `.tex`.
    Render(RenderArgs),

    /// Compila um `.tex` para PDF usando o engine configurado.
    Compile(CompileArgs),
}

#[derive(Debug, clap::Args)]
pub struct CompileArgs {
    /// Path do arquivo `.tex` a compilar. Ausente → modo interativo.
    pub tex_file: Option<std::path::PathBuf>,

    /// Caminho custom do PDF final. Default: `paths.output_dir/<basename>.pdf`.
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,

    /// Engine LaTeX (`tectonic`, `latexmk`, `pdflatex`, `xelatex`, `lualatex`).
    /// Sobrescreve `compiler.engine` do config só nesta invocação.
    #[arg(short = 'e', long)]
    pub engine: Option<String>,

    /// Copia o `.tex` fonte para `output_dir` ao final.
    #[arg(long, conflicts_with = "no_keep_tex")]
    pub keep_tex: bool,

    /// Não copia o `.tex` (sobrescreve `compiler.keep_tex=true` do config).
    #[arg(long = "no-keep-tex", conflicts_with = "keep_tex")]
    pub no_keep_tex: bool,

    /// Copia o `.log` do engine para `output_dir` ao final.
    #[arg(long, conflicts_with = "no_keep_logs")]
    pub keep_logs: bool,

    /// Não copia o `.log` (sobrescreve `compiler.keep_logs=true` do config).
    #[arg(long = "no-keep-logs", conflicts_with = "keep_logs")]
    pub no_keep_logs: bool,

    /// Sobrescreve PDF existente sem pedir confirmação.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct RenderArgs {
    /// Nome do template (com ou sem `.tex`). Opcional para modo interativo.
    pub template_name: Option<String>,

    /// Path do arquivo JSON no host OU `-` para ler de stdin. Opcional para modo interativo.
    pub data_source: Option<String>,

    /// Caminho custom do `.tex` final. Default: `paths.output_dir/<template>.tex`.
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,

    /// Imprime o renderizado em stdout, não grava arquivo. Ignora `--output`.
    #[arg(long)]
    pub dry_run: bool,

    /// Sobrescreve arquivo existente sem pedir confirmação.
    #[arg(long)]
    pub force: bool,
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

pub fn handle_templates_add(args: AddTemplateArgs) -> Result<()> {
    use std::io::IsTerminal;

    let path = config_file_path()?;
    let cfg = Config::load(&path)?;
    let dir = &cfg.paths.templates_dir;

    let dest_name = match &args.name {
        Some(n) => n.clone(),
        None => args
            .source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("caminho do arquivo fonte inválido"))?
            .to_string(),
    };
    let dest_path = dir.join(format!("{dest_name}.tex"));

    let should_overwrite = if dest_path.exists() && !args.force {
        if std::io::stdin().is_terminal() {
            confirm_overwrite_template(&dest_name)?
        } else {
            false
        }
    } else {
        true
    };

    if !should_overwrite {
        return Err(anyhow::Error::new(TexError::UserAborted));
    }

    let outcome = add_template(
        dir,
        &args.source_path,
        args.name.as_deref(),
        args.force || should_overwrite,
    )?;

    let verb = if outcome.overwrote_existing {
        "sobrescrito"
    } else {
        "adicionado"
    };
    println!(
        "Template '{}' {} em {}.",
        outcome.name,
        verb,
        outcome.path.display()
    );
    Ok(())
}

pub fn handle_templates_remove(name: String, force: bool) -> Result<()> {
    use std::io::IsTerminal;

    let path = config_file_path()?;
    let cfg = Config::load(&path)?;
    let dir = &cfg.paths.templates_dir;

    // Resolve first so a nonexistent name returns exit 20 even with --force.
    let _ = resolve_template(dir, &name)?;

    let should_remove = if force {
        true
    } else if std::io::stdin().is_terminal() {
        confirm_remove_template(&name)?
    } else {
        eprintln!("Template '{name}' não removido: use --force ou execute em terminal interativo.");
        false
    };

    if !should_remove {
        return Err(anyhow::Error::new(TexError::UserAborted));
    }

    let removed_path = remove_template(dir, &name, true)?;
    println!("Template '{name}' removido de {}.", removed_path.display());
    Ok(())
}

pub fn handle_compile(_args: CompileArgs) -> Result<()> {
    Err(anyhow!("handle_compile: não implementado"))
}

pub fn handle_render(args: RenderArgs) -> Result<()> {
    let path = config_file_path()?;
    let cfg = Config::load(&path)?;

    let (template_name, data_source, dry_run, force, output) =
        match (&args.template_name, &args.data_source) {
            (Some(t), Some(d)) => (t.clone(), d.clone(), args.dry_run, args.force, args.output),
            _ => return handle_render_menu(&cfg),
        };

    if dry_run && output.is_some() {
        tracing::warn!("--output ignorado porque --dry-run está ativo");
    }

    let expanded_output = match output.as_deref() {
        Some(p) => Some(crate::paths::expand_user_path(&p.display().to_string())?),
        None => None,
    };

    render_and_print(
        &cfg,
        &template_name,
        &data_source,
        expanded_output.as_deref(),
        force,
        dry_run,
    )
}

fn render_and_print(
    cfg: &Config,
    template_name: &str,
    data_source: &str,
    output: Option<&std::path::Path>,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    let outcome =
        crate::render::render_and_write(cfg, template_name, data_source, output, force, dry_run)?;
    if outcome.dry_run {
        return Ok(());
    }
    let path = outcome
        .output_path
        .as_ref()
        .expect("output_path is Some when dry_run is false");
    if outcome.overwrote_existing {
        println!("Renderizado (sobrescrito) em {}.", path.display());
    } else {
        println!("Renderizado em {}.", path.display());
    }
    Ok(())
}

fn handle_render_menu(cfg: &Config) -> Result<()> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "Menu interativo de render requer terminal. Use tex-cli render <template> <data.json>."
        ));
    }

    let templates = crate::templates::list_templates(&cfg.paths.templates_dir)?;
    if templates.is_empty() {
        return Err(anyhow::Error::new(TexError::TemplatesDirMissing {
            templates_dir: cfg.paths.templates_dir.clone(),
        }));
    }

    let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    let template_name = prompt_template_name(&names)?;
    let data_source = prompt_json_source()?;
    let dry_run = confirm_dry_run()?;

    render_and_print(cfg, &template_name, &data_source, None, false, dry_run)
}

pub fn handle_templates_menu() -> Result<()> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "Menu interativo de templates requer terminal. Use um subcomando explícito: tex-cli templates list|show|add|remove."
        ));
    }

    let path = config_file_path()?;
    let cfg = Config::load(&path)?;
    let dir = cfg.paths.templates_dir.clone();

    match template_menu()? {
        TemplateMenuAction::List => handle_templates_list(ShowFormat::Humano),
        TemplateMenuAction::Show => {
            let templates = list_templates(&dir)?;
            let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
            let chosen = prompt_template_name(&names)?;
            handle_templates_show(chosen)
        }
        TemplateMenuAction::Add => {
            let source_path = prompt_source_path()?;
            let args = AddTemplateArgs {
                source_path,
                name: None,
                force: false,
            };
            handle_templates_add(args)
        }
        TemplateMenuAction::Remove => {
            let templates = list_templates(&dir)?;
            let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
            let chosen = prompt_template_name(&names)?;
            handle_templates_remove(chosen, false)
        }
        TemplateMenuAction::Quit => Ok(()),
    }
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
