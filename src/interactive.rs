use std::path::{Path, PathBuf};

use inquire::{Confirm, InquireError, Select, Text};

use crate::errors::TexError;
use crate::paths::expand_user_path;

pub const SUPPORTED_ENGINES: &[&str] = &["tectonic", "latexmk", "pdflatex", "xelatex", "lualatex"];

pub struct InitAnswers {
    pub templates_dir: PathBuf,
    pub output_dir: PathBuf,
    pub engine: String,
}

pub fn run_init_prompts() -> Result<InitAnswers, TexError> {
    let templates_raw = Text::new("LaTeX templates directory:")
        .prompt()
        .map_err(map_inquire_err)?;
    let templates_dir = expand_user_path(&templates_raw)?;

    let output_raw = Text::new("Default output directory for PDFs:")
        .prompt()
        .map_err(map_inquire_err)?;
    let output_dir = expand_user_path(&output_raw)?;

    let engine = Select::new("LaTeX engine:", SUPPORTED_ENGINES.to_vec())
        .with_starting_cursor(0)
        .prompt()
        .map_err(map_inquire_err)?
        .to_string();

    Ok(InitAnswers {
        templates_dir,
        output_dir,
        engine,
    })
}

pub fn confirm_overwrite(path: &Path) -> Result<bool, TexError> {
    Confirm::new(&format!("Overwrite config at {}?", path.display()))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_create_dir(path: &Path) -> Result<bool, TexError> {
    Confirm::new(&format!("Create directory {}?", path.display()))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_overwrite_template(name: &str) -> Result<bool, TexError> {
    Confirm::new(&format!("Overwrite template '{name}'?"))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_remove_template(name: &str) -> Result<bool, TexError> {
    Confirm::new(&format!("Remove template '{name}'?"))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_compile_overwrite(path: &Path) -> Result<bool, TexError> {
    Confirm::new(&format!("Overwrite {}?", path.display()))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn prompt_tex_source() -> Result<std::path::PathBuf, TexError> {
    let raw = Text::new("Path to the .tex to compile:")
        .prompt()
        .map_err(map_inquire_err)?;
    Ok(std::path::PathBuf::from(raw.trim()))
}

pub fn confirm_keep_tex(default: bool) -> Result<bool, TexError> {
    Confirm::new("Keep a copy of the .tex in output_dir?")
        .with_default(default)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_keep_logs(default: bool) -> Result<bool, TexError> {
    Confirm::new("Keep a copy of the .log in output_dir?")
        .with_default(default)
        .prompt()
        .map_err(map_inquire_err)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateMenuAction {
    List,
    Show,
    Add,
    Remove,
    Quit,
}

pub fn template_menu() -> Result<TemplateMenuAction, TexError> {
    let options = vec![
        "List templates",
        "Inspect template",
        "Add template",
        "Remove template",
        "Quit",
    ];
    let choice = Select::new("What would you like to do with templates?", options)
        .with_starting_cursor(0)
        .prompt()
        .map_err(map_inquire_err)?;

    Ok(match choice {
        "List templates" => TemplateMenuAction::List,
        "Inspect template" => TemplateMenuAction::Show,
        "Add template" => TemplateMenuAction::Add,
        "Remove template" => TemplateMenuAction::Remove,
        _ => TemplateMenuAction::Quit,
    })
}

pub fn prompt_template_name(available: &[String]) -> Result<String, TexError> {
    if available.is_empty() {
        return Err(TexError::Io(std::io::Error::other(
            "no templates available for selection",
        )));
    }
    let names: Vec<String> = available.to_vec();
    let choice = Select::new("Template name:", names)
        .prompt()
        .map_err(map_inquire_err)?;
    Ok(choice)
}

pub fn prompt_source_path() -> Result<std::path::PathBuf, TexError> {
    let raw = Text::new("Path to the .tex file:")
        .prompt()
        .map_err(map_inquire_err)?;
    Ok(std::path::PathBuf::from(raw.trim()))
}

pub fn prompt_json_source() -> Result<String, TexError> {
    Text::new("Path to the JSON file (- for stdin):")
        .prompt()
        .map(|s| s.trim().to_string())
        .map_err(map_inquire_err)
}

pub fn confirm_dry_run() -> Result<bool, TexError> {
    Confirm::new("Dry-run mode (prints to stdout, does not write)?")
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

fn map_inquire_err(err: InquireError) -> TexError {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            TexError::UserAborted
        }
        InquireError::IO(e) => TexError::Io(e),
        other => TexError::Io(std::io::Error::other(other.to_string())),
    }
}
