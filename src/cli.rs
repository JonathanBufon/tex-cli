use std::fs;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};

use std::str::FromStr;

use crate::config::{render_humano, Config, ConfigKey};
use crate::errors::TexError;
use crate::interactive::{
    confirm_compile_overwrite, confirm_create_dir, confirm_dry_run, confirm_keep_logs,
    confirm_keep_tex, confirm_overwrite, confirm_overwrite_template, confirm_remove_template,
    prompt_json_source, prompt_source_path, prompt_template_name, prompt_tex_source,
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
    about = "Convert JSON data into LaTeX documents and compile them to PDF.",
    version
)]
pub struct Cli {
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count,
        global = true,
        help = "Increase log verbosity on stderr (repeatable: -v, -vv, -vvv)."
    )]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create the config at ~/.config/tex/config.toml (interactive or via flags).
    Init(InitArgs),

    /// Inspect or update the current config.
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Manage LaTeX templates in `paths.templates_dir`.
    Templates(TemplatesArgs),

    /// Render a template with JSON data, producing a `.tex`.
    Render(RenderArgs),

    /// Compile a `.tex` to PDF using the configured engine.
    Compile(CompileArgs),

    /// JSON → PDF pipeline: render template + compile in one invocation.
    Build(BuildArgs),
}

#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Template name (with or without `.tex`). Absent → interactive mode.
    pub template_name: Option<String>,

    /// Path to JSON on host OR `-` for stdin. Absent → interactive mode.
    pub data_source: Option<String>,

    /// Custom path for the final PDF. Default: `paths.output_dir/<template>.pdf`.
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,

    /// LaTeX engine (`tectonic`, `latexmk`, `pdflatex`, `xelatex`, `lualatex`).
    /// Overrides `compiler.engine` from config for this invocation only.
    #[arg(short = 'e', long)]
    pub engine: Option<String>,

    /// Copy the intermediate `.tex` to `output_dir` BEFORE compile.
    #[arg(long, conflicts_with = "no_keep_tex")]
    pub keep_tex: bool,

    /// Discard the intermediate `.tex` even if config says true.
    #[arg(long = "no-keep-tex", conflicts_with = "keep_tex")]
    pub no_keep_tex: bool,

    /// Copy the engine's `.log` to `output_dir` after compile.
    #[arg(long, conflicts_with = "no_keep_logs")]
    pub keep_logs: bool,

    /// Discard the `.log` even if config says true.
    #[arg(long = "no-keep-logs", conflicts_with = "keep_logs")]
    pub no_keep_logs: bool,

    /// Overwrite an existing PDF without confirming.
    #[arg(long)]
    pub force: bool,

    /// Spec 007 US1: resolve the template from the JSON's `document.type`
    /// or `document.template` field. Mutually exclusive with the positional
    /// `template_name`. Value is a path to a JSON file, or `-` for stdin.
    #[arg(long = "json", conflicts_with = "template_name")]
    pub json: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct CompileArgs {
    /// Path to the `.tex` file to compile. Absent → interactive mode.
    pub tex_file: Option<std::path::PathBuf>,

    /// Custom path for the final PDF. Default: `paths.output_dir/<basename>.pdf`.
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,

    /// LaTeX engine (`tectonic`, `latexmk`, `pdflatex`, `xelatex`, `lualatex`).
    /// Overrides `compiler.engine` from config for this invocation only.
    #[arg(short = 'e', long)]
    pub engine: Option<String>,

    /// Copy the source `.tex` to `output_dir` at the end.
    #[arg(long, conflicts_with = "no_keep_tex")]
    pub keep_tex: bool,

    /// Do not copy the `.tex` (overrides `compiler.keep_tex=true` from config).
    #[arg(long = "no-keep-tex", conflicts_with = "keep_tex")]
    pub no_keep_tex: bool,

    /// Copy the engine's `.log` to `output_dir` at the end.
    #[arg(long, conflicts_with = "no_keep_logs")]
    pub keep_logs: bool,

    /// Do not copy the `.log` (overrides `compiler.keep_logs=true` from config).
    #[arg(long = "no-keep-logs", conflicts_with = "keep_logs")]
    pub no_keep_logs: bool,

    /// Overwrite an existing PDF without confirming.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct RenderArgs {
    /// Template name (with or without `.tex`). Optional for interactive mode.
    pub template_name: Option<String>,

    /// Path to the JSON file on host OR `-` to read from stdin. Optional for interactive mode.
    pub data_source: Option<String>,

    /// Custom path for the final `.tex`. Default: `paths.output_dir/<template>.tex`.
    #[arg(short = 'o', long)]
    pub output: Option<std::path::PathBuf>,

    /// Print the rendered output to stdout instead of writing a file. Ignores `--output`.
    #[arg(long)]
    pub dry_run: bool,

    /// Overwrite an existing file without confirming.
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
    /// List the `.tex` files in `paths.templates_dir`.
    List {
        #[arg(long, default_value = "human")]
        format: ShowFormat,
    },

    /// Print the raw content of a template to stdout.
    Show { name: String },

    /// Add a new template by copying a file from the host.
    Add(AddTemplateArgs),

    /// Remove a template from `paths.templates_dir`.
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, clap::Args)]
pub struct AddTemplateArgs {
    /// Path to the `.tex` file on the host.
    pub source_path: std::path::PathBuf,

    /// Final template name (default = file basename without `.tex`).
    #[arg(short = 'n', long)]
    pub name: Option<String>,

    /// Overwrite an existing template without confirming.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// LaTeX templates directory (skips the corresponding prompt).
    #[arg(short = 't', long)]
    pub templates_dir: Option<String>,

    /// Default output directory for PDFs (skips the corresponding prompt).
    #[arg(short = 'o', long)]
    pub output_dir: Option<String>,

    /// Name of the LaTeX engine (`tectonic`, `latexmk`, `pdflatex`, `xelatex`, `lualatex`).
    #[arg(short = 'e', long)]
    pub engine: Option<String>,

    /// Create missing directories without confirming.
    #[arg(long)]
    pub create_dirs: bool,

    /// Overwrite an existing config without confirming.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Show the current config (human/json/toml).
    Show {
        #[arg(long, default_value = "human")]
        format: ShowFormat,
    },

    /// Change a single value in the config (`<dotted-key> <value>`).
    Set { key: String, value: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShowFormat {
    Human,
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

    println!("Config written to: {}", config_path.display());
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
            "Warning: '{engine}' was not found on PATH. Preference saved; install the binary later."
        );
    }
    if engine != "tectonic" {
        eprintln!(
            "Warning: engine '{engine}' is not yet executed by Tex in this version. Preference saved."
        );
    }
}

pub fn handle_config_show(format: ShowFormat) -> Result<()> {
    let path = config_file_path()?;
    let cfg = Config::load(&path)?;

    let rendered = match format {
        ShowFormat::Human => render_humano(&cfg),
        ShowFormat::Json => {
            serde_json::to_string_pretty(&cfg)
                .map_err(|e| anyhow!("failed to serialize JSON: {e}"))?
                + "\n"
        }
        ShowFormat::Toml => {
            toml::to_string_pretty(&cfg).map_err(|e| anyhow!("failed to serialize TOML: {e}"))?
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
        ShowFormat::Human => {
            print!("{}", render_template_list_humano(dir, &templates));
        }
        ShowFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&templates)?);
        }
        ShowFormat::Toml => {
            return Err(anyhow!(
                "--format=toml is not supported for templates list. Use human or json."
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
            .ok_or_else(|| anyhow!("invalid source file path"))?
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
        "overwritten"
    } else {
        "added"
    };
    println!(
        "Template '{}' {} at {}.",
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
        eprintln!("Template '{name}' not removed: use --force or run in an interactive terminal.");
        false
    };

    if !should_remove {
        return Err(anyhow::Error::new(TexError::UserAborted));
    }

    let removed_path = remove_template(dir, &name, true)?;
    println!("Template '{name}' removed from {}.", removed_path.display());
    Ok(())
}

pub fn handle_build(args: BuildArgs) -> Result<()> {
    use std::io::IsTerminal;

    let path = config_file_path()?;
    let cfg = Config::load(&path)?;

    // Spec 007 US1 route: `--json <source>` — resolve template from the JSON.
    if let Some(json_source) = args.json.as_deref() {
        return handle_build_from_json(&cfg, &args, json_source);
    }

    let (template_name, data_source) = match (&args.template_name, &args.data_source) {
        (Some(t), Some(d)) => (t.clone(), d.clone()),
        _ => return handle_build_menu(&cfg),
    };

    let engine = crate::compiler::resolve_engine(args.engine.as_deref(), &cfg.compiler.engine)?;

    let output_pdf = match args.output.as_ref() {
        Some(p) => crate::paths::expand_user_path(&p.display().to_string())?,
        None => crate::build::resolve_output_pdf(&cfg, &template_name, None),
    };

    let keep_tex = if args.keep_tex {
        true
    } else if args.no_keep_tex {
        false
    } else {
        cfg.compiler.keep_tex
    };
    let keep_logs = if args.keep_logs {
        true
    } else if args.no_keep_logs {
        false
    } else {
        cfg.compiler.keep_logs
    };

    if output_pdf.exists() && !args.force {
        let confirmed = if std::io::stdin().is_terminal() {
            confirm_compile_overwrite(&output_pdf)?
        } else {
            eprintln!(
                "File {} already exists. Use --force or run in an interactive terminal.",
                output_pdf.display()
            );
            false
        };
        if !confirmed {
            return Err(anyhow::Error::new(TexError::UserAborted));
        }
    }

    let outcome = crate::build::build_pipeline(
        &cfg,
        &template_name,
        &data_source,
        &output_pdf,
        engine,
        keep_tex,
        keep_logs,
        args.force,
        0,
    )?;

    let secs = outcome.total_duration.as_secs_f32();
    if outcome.overwrote_existing {
        println!(
            "PDF generated (overwritten) at {}. Pipeline (render + compile) took {:.1}s.",
            outcome.pdf_path.display(),
            secs
        );
    } else {
        println!(
            "PDF generated at {}. Pipeline (render + compile) took {:.1}s.",
            outcome.pdf_path.display(),
            secs
        );
    }
    Ok(())
}

/// Spec 007 US1 (T012–T014): `tex-cli build --json <source>` — resolve
/// the template from the JSON's `document.type` / `document.template`
/// then delegate to the standard build pipeline.
fn handle_build_from_json(cfg: &Config, args: &BuildArgs, json_source: &str) -> Result<()> {
    use std::io::IsTerminal;

    let engine = crate::compiler::resolve_engine(args.engine.as_deref(), &cfg.compiler.engine)?;

    let keep_tex = if args.keep_tex {
        true
    } else if args.no_keep_tex {
        false
    } else {
        cfg.compiler.keep_tex
    };
    let keep_logs = if args.keep_logs {
        true
    } else if args.no_keep_logs {
        false
    } else {
        cfg.compiler.keep_logs
    };

    let output_override = args
        .output
        .as_ref()
        .map(|p| crate::paths::expand_user_path(&p.display().to_string()))
        .transpose()?;

    // For the overwrite guard we need the resolved output path — but that
    // depends on which template the resolver picks. Peek by resolving here
    // first is expensive (loads JSON + enumerates templates twice). Simpler:
    // let the pipeline resolve, then check overwrite only if a caller-provided
    // --output was set (which is the only case we can pre-check without
    // knowing the template name).
    if let Some(target) = output_override.as_deref() {
        if target.exists() && !args.force {
            let confirmed = if std::io::stdin().is_terminal() {
                confirm_compile_overwrite(target)?
            } else {
                eprintln!(
                    "File {} already exists. Use --force or run in an interactive terminal.",
                    target.display()
                );
                false
            };
            if !confirmed {
                return Err(anyhow::Error::new(TexError::UserAborted));
            }
        }
    }

    let outcome = crate::build::build_pipeline_from_json(
        cfg,
        json_source,
        output_override.as_deref(),
        engine,
        keep_tex,
        keep_logs,
        args.force,
        0,
    )?;

    let secs = outcome.total_duration.as_secs_f32();
    let template_desc = match &outcome.template_version {
        Some(ver) => format!("{}@{}", outcome.template_identifier, ver),
        None => outcome.template_identifier.to_string(),
    };
    if outcome.overwrote_existing {
        println!(
            "PDF generated (overwritten) at {}. Template: {}. Pipeline (render + compile) took {:.1}s.",
            outcome.pdf_path.display(),
            template_desc,
            secs
        );
    } else {
        println!(
            "PDF generated at {}. Template: {}. Pipeline (render + compile) took {:.1}s.",
            outcome.pdf_path.display(),
            template_desc,
            secs
        );
    }
    Ok(())
}

fn handle_build_menu(cfg: &Config) -> Result<()> {
    use inquire::Select;
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "Interactive build menu requires a terminal. Use tex-cli build <template> <data.json>."
        ));
    }

    // Spec 007 T015 (Constitution III): the "JSON → resolve → PDF" flow
    // is offered on equal footing with the explicit template+JSON flow.
    let mode = Select::new(
        "How do you want to build?",
        vec![
            "Compile a JSON to PDF (auto-resolve template from document.type)",
            "Pick a template and JSON explicitly",
        ],
    )
    .with_starting_cursor(0)
    .prompt()
    .map_err(|e| {
        anyhow::Error::new(match e {
            inquire::InquireError::OperationCanceled
            | inquire::InquireError::OperationInterrupted => TexError::UserAborted,
            other => TexError::Io(std::io::Error::other(other.to_string())),
        })
    })?;

    if mode.starts_with("Compile a JSON to PDF") {
        let json_source = prompt_json_source()?;
        let keep_tex = confirm_keep_tex(cfg.compiler.keep_tex)?;
        let keep_logs = confirm_keep_logs(cfg.compiler.keep_logs)?;

        let args = BuildArgs {
            template_name: None,
            data_source: None,
            output: None,
            engine: None,
            keep_tex,
            no_keep_tex: !keep_tex,
            keep_logs,
            no_keep_logs: !keep_logs,
            force: false,
            json: Some(json_source),
        };
        return handle_build(args);
    }

    // Explicit path (unchanged pre-007 behaviour).
    let templates = list_templates(&cfg.paths.templates_dir)?;
    if templates.is_empty() {
        return Err(anyhow::Error::new(TexError::TemplatesDirMissing {
            templates_dir: cfg.paths.templates_dir.clone(),
        }));
    }

    let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    let template_name = prompt_template_name(&names)?;
    let data_source = prompt_json_source()?;
    let keep_tex = confirm_keep_tex(cfg.compiler.keep_tex)?;
    let keep_logs = confirm_keep_logs(cfg.compiler.keep_logs)?;

    let args = BuildArgs {
        template_name: Some(template_name),
        data_source: Some(data_source),
        output: None,
        engine: None,
        keep_tex,
        no_keep_tex: !keep_tex,
        keep_logs,
        no_keep_logs: !keep_logs,
        force: false,
        json: None,
    };
    handle_build(args)
}

pub fn handle_compile(args: CompileArgs) -> Result<()> {
    use std::io::IsTerminal;

    let cfg_path = config_file_path()?;
    let cfg = Config::load(&cfg_path)?;

    let tex_file = match args.tex_file.as_ref() {
        Some(p) => p.clone(),
        None => return handle_compile_menu(&cfg),
    };

    let tex_path = crate::paths::expand_user_path(&tex_file.display().to_string())?;

    let engine = crate::compiler::resolve_engine(args.engine.as_deref(), &cfg.compiler.engine)?;

    let output_pdf = match args.output.as_ref() {
        Some(p) => crate::paths::expand_user_path(&p.display().to_string())?,
        None => {
            let basename = tex_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow!("invalid source .tex path"))?;
            cfg.paths.output_dir.join(format!("{basename}.pdf"))
        }
    };

    let keep_tex = if args.keep_tex {
        true
    } else if args.no_keep_tex {
        false
    } else {
        cfg.compiler.keep_tex
    };

    let keep_logs = if args.keep_logs {
        true
    } else if args.no_keep_logs {
        false
    } else {
        cfg.compiler.keep_logs
    };

    if output_pdf.exists() && !args.force {
        let confirmed = if std::io::stdin().is_terminal() {
            confirm_compile_overwrite(&output_pdf)?
        } else {
            eprintln!(
                "File {} already exists. Use --force or run in an interactive terminal.",
                output_pdf.display()
            );
            false
        };
        if !confirmed {
            return Err(anyhow::Error::new(TexError::UserAborted));
        }
    }

    let outcome = crate::compiler::compile_and_write(
        &cfg,
        &tex_path,
        engine,
        &output_pdf,
        keep_tex,
        keep_logs,
        args.force,
        0,
    )?;

    print_compile_outcome(&outcome);
    Ok(())
}

fn print_compile_outcome(outcome: &crate::compiler::CompileOutcome) {
    let secs = outcome.duration.as_secs_f32();
    if outcome.overwrote_existing {
        println!(
            "PDF generated (overwritten) at {}. Compile took {:.1}s.",
            outcome.pdf_path.display(),
            secs
        );
    } else {
        println!(
            "PDF generated at {}. Compile took {:.1}s.",
            outcome.pdf_path.display(),
            secs
        );
    }
}

fn handle_compile_menu(cfg: &Config) -> Result<()> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "Interactive compile menu requires a terminal. Use tex-cli compile <tex-file>."
        ));
    }

    let tex_path = prompt_tex_source()?;
    let keep_tex = confirm_keep_tex(cfg.compiler.keep_tex)?;
    let keep_logs = confirm_keep_logs(cfg.compiler.keep_logs)?;

    let tex_path = crate::paths::expand_user_path(&tex_path.display().to_string())?;

    let engine = crate::compiler::resolve_engine(None, &cfg.compiler.engine)?;

    let basename = tex_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid source .tex path"))?;
    let output_pdf = cfg.paths.output_dir.join(format!("{basename}.pdf"));

    if output_pdf.exists() && !confirm_compile_overwrite(&output_pdf)? {
        return Err(anyhow::Error::new(TexError::UserAborted));
    }

    let outcome = crate::compiler::compile_and_write(
        cfg,
        &tex_path,
        engine,
        &output_pdf,
        keep_tex,
        keep_logs,
        false,
        0,
    )?;

    print_compile_outcome(&outcome);
    Ok(())
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
        tracing::warn!("--output ignored because --dry-run is active");
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
        println!(".tex written (overwritten) to {}.", path.display());
    } else {
        println!(".tex written to {}.", path.display());
    }
    Ok(())
}

fn handle_render_menu(cfg: &Config) -> Result<()> {
    use std::io::IsTerminal;

    if !std::io::stdin().is_terminal() {
        return Err(anyhow!(
            "Interactive render menu requires a terminal. Use tex-cli render <template> <data.json>."
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
            "Interactive templates menu requires a terminal. Use an explicit subcommand: tex-cli templates list|show|add|remove."
        ));
    }

    let path = config_file_path()?;
    let cfg = Config::load(&path)?;
    let dir = cfg.paths.templates_dir.clone();

    match template_menu()? {
        TemplateMenuAction::List => handle_templates_list(ShowFormat::Human),
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
        "Config updated: {} = {}",
        change.key, change.normalized_value
    );
    Ok(())
}
