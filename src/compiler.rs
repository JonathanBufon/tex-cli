use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::errors::TexError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedEngine {
    Tectonic,
    Latexmk,
    Pdflatex,
    Xelatex,
    Lualatex,
}

impl SupportedEngine {
    pub const ALL: &'static [SupportedEngine] = &[
        SupportedEngine::Tectonic,
        SupportedEngine::Latexmk,
        SupportedEngine::Pdflatex,
        SupportedEngine::Xelatex,
        SupportedEngine::Lualatex,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            SupportedEngine::Tectonic => "tectonic",
            SupportedEngine::Latexmk => "latexmk",
            SupportedEngine::Pdflatex => "pdflatex",
            SupportedEngine::Xelatex => "xelatex",
            SupportedEngine::Lualatex => "lualatex",
        }
    }

    pub fn binary_name(&self) -> &'static str {
        self.as_str()
    }

    pub fn args_for(&self, tex_filename: &str) -> Vec<String> {
        match self {
            SupportedEngine::Tectonic => vec![
                "--outdir=.".to_string(),
                "--keep-logs".to_string(),
                "--keep-intermediates".to_string(),
                tex_filename.to_string(),
            ],
            SupportedEngine::Latexmk => vec![
                "-pdf".to_string(),
                "-interaction=nonstopmode".to_string(),
                "-halt-on-error".to_string(),
                tex_filename.to_string(),
            ],
            SupportedEngine::Pdflatex
            | SupportedEngine::Xelatex
            | SupportedEngine::Lualatex => vec![
                "-interaction=nonstopmode".to_string(),
                "-halt-on-error".to_string(),
                tex_filename.to_string(),
            ],
        }
    }
}

impl fmt::Display for SupportedEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SupportedEngine {
    type Err = TexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for engine in SupportedEngine::ALL {
            if engine.as_str() == s {
                return Ok(*engine);
            }
        }
        Err(TexError::EngineNotSupported {
            engine: s.to_string(),
            accepted: SupportedEngine::ALL.iter().map(|e| e.as_str()).collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutcome {
    pub pdf_path: PathBuf,
    pub duration: Duration,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
    pub kept_tex: bool,
    pub kept_logs: bool,
}

pub fn resolve_engine(
    cli_flag: Option<&str>,
    config_engine: &str,
) -> Result<SupportedEngine, TexError> {
    match cli_flag {
        Some(s) => s.parse(),
        None => config_engine.parse(),
    }
}

pub fn validate_binary(engine: SupportedEngine) -> Result<PathBuf, TexError> {
    which::which(engine.binary_name()).map_err(|_| TexError::EngineNotInstalled {
        engine: engine.to_string(),
    })
}

pub fn tail_lines(s: &str, n: usize) -> String {
    if s.is_empty() {
        return String::new();
    }
    let lines: Vec<&str> = s.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

pub fn run_engine(
    engine: SupportedEngine,
    cwd: &Path,
    tex_filename: &str,
    verbose: u8,
) -> Result<(), (i32, String)> {
    let args = engine.args_for(tex_filename);
    let mut cmd = Command::new(engine.binary_name());
    cmd.current_dir(cwd).args(&args);

    let stem = Path::new(tex_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let pdf_path = cwd.join(format!("{stem}.pdf"));
    let log_path = cwd.join(format!("{stem}.log"));

    if verbose >= 2 {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let status = cmd.status().map_err(|e| (1, format!("failed to spawn engine: {e}")))?;
        if !status.success() || !pdf_path.exists() {
            let log_tail = read_log_tail(&log_path).unwrap_or_default();
            return Err((status.code().unwrap_or(1), log_tail));
        }
        Ok(())
    } else {
        let output = cmd
            .output()
            .map_err(|e| (1, format!("failed to spawn engine: {e}")))?;
        if !output.status.success() || !pdf_path.exists() {
            let log_tail = read_log_tail(&log_path).unwrap_or_else(|| {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.trim().is_empty() {
                    tail_lines(&stderr, 30)
                } else {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    tail_lines(&stdout, 30)
                }
            });
            let log_tail = if log_tail.trim().is_empty() {
                "Engine retornou sucesso mas nenhum PDF foi produzido.".to_string()
            } else {
                log_tail
            };
            return Err((output.status.code().unwrap_or(1), log_tail));
        }
        Ok(())
    }
}

fn read_log_tail(log_path: &Path) -> Option<String> {
    let contents = fs::read_to_string(log_path).ok()?;
    let tail = tail_lines(&contents, 30);
    if tail.trim().is_empty() {
        None
    } else {
        Some(tail)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn compile_and_write(
    _cfg: &Config,
    tex_path: &Path,
    engine: SupportedEngine,
    output_pdf: &Path,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
) -> Result<CompileOutcome, TexError> {
    if !tex_path.exists() {
        return Err(TexError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("arquivo {} não encontrado", tex_path.display()),
        )));
    }

    validate_binary(engine)?;

    let temp = tempfile::TempDir::new()?;
    let basename = tex_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            TexError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "tex path has no valid file stem",
            ))
        })?
        .to_string();
    let tex_filename = format!("{basename}.tex");
    let temp_tex = temp.path().join(&tex_filename);
    fs::copy(tex_path, &temp_tex)?;

    let start = Instant::now();
    run_engine(engine, temp.path(), &tex_filename, verbose).map_err(|(_, log_tail)| {
        TexError::CompileFailed {
            engine: engine.to_string(),
            tex_path: tex_path.to_path_buf(),
            log_tail,
        }
    })?;
    let duration = start.elapsed();

    let overwrote_existing = output_pdf.exists();
    if overwrote_existing && !force {
        return Err(TexError::UserAborted);
    }

    let temp_pdf = temp.path().join(format!("{basename}.pdf"));
    let pdf_bytes = fs::read(&temp_pdf)?;
    let bytes_written = pdf_bytes.len() as u64;
    crate::atomic::write_atomic(output_pdf, &pdf_bytes, 0o644)?;

    let output_dir = output_pdf.parent().unwrap_or(Path::new("."));

    if keep_tex {
        let tex_bytes = fs::read(tex_path)?;
        let dest = output_dir.join(&tex_filename);
        crate::atomic::write_atomic(&dest, &tex_bytes, 0o644)?;
    }

    if keep_logs {
        let temp_log = temp.path().join(format!("{basename}.log"));
        if let Ok(log_bytes) = fs::read(&temp_log) {
            let dest = output_dir.join(format!("{basename}.log"));
            crate::atomic::write_atomic(&dest, &log_bytes, 0o644)?;
        }
    }

    Ok(CompileOutcome {
        pdf_path: output_pdf.to_path_buf(),
        duration,
        bytes_written,
        overwrote_existing,
        kept_tex: keep_tex,
        kept_logs: keep_logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_engine_from_str_accepts_canonical() {
        for engine in SupportedEngine::ALL {
            let parsed: SupportedEngine = engine.as_str().parse().unwrap();
            assert_eq!(&parsed, engine);
        }
    }

    #[test]
    fn supported_engine_from_str_rejects_unknown() {
        let err = "foo".parse::<SupportedEngine>().unwrap_err();
        assert!(matches!(err, TexError::EngineNotSupported { .. }));
    }

    #[test]
    fn supported_engine_from_str_case_sensitive() {
        assert!("Tectonic".parse::<SupportedEngine>().is_err());
        assert!("TECTONIC".parse::<SupportedEngine>().is_err());
    }

    #[test]
    fn supported_engine_as_str_matches_binary_name() {
        for engine in SupportedEngine::ALL {
            assert_eq!(engine.as_str(), engine.binary_name());
        }
    }

    #[test]
    fn supported_engine_args_for_tectonic() {
        let args = SupportedEngine::Tectonic.args_for("artigo.tex");
        assert!(args.contains(&"--outdir=.".to_string()));
        assert!(args.contains(&"--keep-logs".to_string()));
        assert!(args.contains(&"--keep-intermediates".to_string()));
        assert_eq!(args.last().unwrap(), "artigo.tex");
    }

    #[test]
    fn supported_engine_args_for_latexmk() {
        let args = SupportedEngine::Latexmk.args_for("artigo.tex");
        assert!(args.contains(&"-pdf".to_string()));
        assert!(args.contains(&"-interaction=nonstopmode".to_string()));
        assert!(args.contains(&"-halt-on-error".to_string()));
    }

    #[test]
    fn supported_engine_args_for_pdflatex_variants() {
        for engine in [
            SupportedEngine::Pdflatex,
            SupportedEngine::Xelatex,
            SupportedEngine::Lualatex,
        ] {
            let args = engine.args_for("artigo.tex");
            assert!(args.contains(&"-interaction=nonstopmode".to_string()));
            assert!(args.contains(&"-halt-on-error".to_string()));
            assert_eq!(args.last().unwrap(), "artigo.tex");
            // pdflatex-family should not carry latexmk-only flags:
            assert!(!args.contains(&"-pdf".to_string()));
        }
    }

    #[test]
    fn tail_lines_returns_last_n_lines() {
        let s = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj";
        assert_eq!(tail_lines(s, 3), "h\ni\nj");
    }

    #[test]
    fn tail_lines_handles_fewer_than_n_lines() {
        let s = "a\nb";
        assert_eq!(tail_lines(s, 5), "a\nb");
    }

    #[test]
    fn tail_lines_empty_string_returns_empty() {
        assert_eq!(tail_lines("", 3), "");
    }

    #[test]
    fn all_has_five_entries() {
        assert_eq!(SupportedEngine::ALL.len(), 5);
    }
}
