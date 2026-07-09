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

fn map_inquire_err(err: InquireError) -> TexError {
    match err {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            TexError::UserAborted
        }
        InquireError::IO(e) => TexError::Io(e),
        other => TexError::Io(std::io::Error::other(other.to_string())),
    }
}
