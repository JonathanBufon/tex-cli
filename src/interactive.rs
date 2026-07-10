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
    let templates_raw = Text::new("Diretório de templates LaTeX:")
        .prompt()
        .map_err(map_inquire_err)?;
    let templates_dir = expand_user_path(&templates_raw)?;

    let output_raw = Text::new("Diretório padrão de saída dos PDFs:")
        .prompt()
        .map_err(map_inquire_err)?;
    let output_dir = expand_user_path(&output_raw)?;

    let engine = Select::new("Compilador LaTeX:", SUPPORTED_ENGINES.to_vec())
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
    Confirm::new(&format!("Sobrescrever config em {}?", path.display()))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_create_dir(path: &Path) -> Result<bool, TexError> {
    Confirm::new(&format!("Criar diretório {}?", path.display()))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_overwrite_template(name: &str) -> Result<bool, TexError> {
    Confirm::new(&format!("Sobrescrever template '{name}'?"))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_remove_template(name: &str) -> Result<bool, TexError> {
    Confirm::new(&format!("Remover template '{name}'?"))
        .with_default(false)
        .prompt()
        .map_err(map_inquire_err)
}

pub fn confirm_compile_overwrite(path: &Path) -> Result<bool, TexError> {
    Confirm::new(&format!("Sobrescrever {}?", path.display()))
        .with_default(false)
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
        "Listar templates",
        "Inspecionar template",
        "Adicionar template",
        "Remover template",
        "Sair",
    ];
    let choice = Select::new("O que fazer com os templates?", options)
        .with_starting_cursor(0)
        .prompt()
        .map_err(map_inquire_err)?;

    Ok(match choice {
        "Listar templates" => TemplateMenuAction::List,
        "Inspecionar template" => TemplateMenuAction::Show,
        "Adicionar template" => TemplateMenuAction::Add,
        "Remover template" => TemplateMenuAction::Remove,
        _ => TemplateMenuAction::Quit,
    })
}

pub fn prompt_template_name(available: &[String]) -> Result<String, TexError> {
    if available.is_empty() {
        return Err(TexError::Io(std::io::Error::other(
            "nenhum template disponível para seleção",
        )));
    }
    let names: Vec<String> = available.to_vec();
    let choice = Select::new("Nome do template:", names)
        .prompt()
        .map_err(map_inquire_err)?;
    Ok(choice)
}

pub fn prompt_source_path() -> Result<std::path::PathBuf, TexError> {
    let raw = Text::new("Caminho do arquivo .tex:")
        .prompt()
        .map_err(map_inquire_err)?;
    Ok(std::path::PathBuf::from(raw.trim()))
}

pub fn prompt_json_source() -> Result<String, TexError> {
    Text::new("Caminho do arquivo JSON (- para stdin):")
        .prompt()
        .map(|s| s.trim().to_string())
        .map_err(map_inquire_err)
}

pub fn confirm_dry_run() -> Result<bool, TexError> {
    Confirm::new("Modo dry-run (imprime em stdout, não grava)?")
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
