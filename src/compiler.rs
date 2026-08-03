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
            SupportedEngine::Pdflatex | SupportedEngine::Xelatex | SupportedEngine::Lualatex => {
                vec![
                    "-interaction=nonstopmode".to_string(),
                    "-halt-on-error".to_string(),
                    tex_filename.to_string(),
                ]
            }
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
    sandbox: SandboxDirective,
) -> Result<(), (i32, String)> {
    let mut args = engine.args_for(tex_filename);
    // Spec 007 FR-017 / research R1: for tectonic, `--only-cached` refuses
    // the network bundle fetch. Other engines don't have an equivalent flag
    // in v1 — the sandbox is honored best-effort on tectonic; Docker per
    // A-01 provides an outer isolation layer for any engine.
    if sandbox.only_cached_bundle && matches!(engine, SupportedEngine::Tectonic) {
        args.push("--only-cached".to_string());
    }
    // Spec 007 FR-017: tectonic's first-class untrusted-input switch.
    // Documented as "disable all known-insecure features" — the cleanest
    // way to honor the sandbox intent without depending on external
    // KPathsea env vars behaving under tectonic's bundled setup.
    if sandbox.disable_shell_escape && matches!(engine, SupportedEngine::Tectonic) {
        args.push("--untrusted".to_string());
    }
    let mut cmd = Command::new(engine.binary_name());
    cmd.current_dir(cwd).args(&args);
    // Spec 007 FR-009 / SC-007: force a fixed build timestamp so tectonic
    // (and any other reproducible-build-aware engine) writes a
    // byte-identical PDF for the same input on repeat compiles. The
    // industry convention is the SOURCE_DATE_EPOCH env var; 1 is used
    // rather than 0 because some tooling treats 0 as "unset".
    cmd.env("SOURCE_DATE_EPOCH", "1");
    if sandbox.paranoid_openout {
        // KPathsea paranoid mode: prevents \openout escapes on engines that
        // honor the env var (tectonic passes it through to its bundled
        // KPathsea; a no-op is harmless on engines that don't).
        cmd.env("openout_any", "p");
    }
    // Never opt into shell-escape; explicit assertion for future engines.
    if sandbox.disable_shell_escape {
        cmd.env_remove("shell_escape");
    }

    let stem = Path::new(tex_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let pdf_path = cwd.join(format!("{stem}.pdf"));
    let log_path = cwd.join(format!("{stem}.log"));

    if verbose >= 2 {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        let status = cmd
            .status()
            .map_err(|e| (1, format!("failed to spawn engine: {e}")))?;
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
                "Engine returned success but no PDF was produced.".to_string()
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

/// Spec 007 FR-017: sandbox knobs applied at compile time for third-party
/// templates. Composed inside this module so callers only carry a boolean
/// concern ("is this a trusted built-in or an untrusted install?").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxDirective {
    /// Never pass `-shell-escape` (tectonic defaults to off already; the
    /// flag is a compile-time affirmation for future engines).
    pub disable_shell_escape: bool,
    /// Pass `--only-cached` to tectonic — refuses any bundle-network fetch.
    pub only_cached_bundle: bool,
    /// Set `openout_any=p` in the engine's env (KPathsea paranoid mode).
    pub paranoid_openout: bool,
}

impl SandboxDirective {
    pub const fn for_builtin() -> Self {
        Self {
            disable_shell_escape: false,
            only_cached_bundle: false,
            paranoid_openout: false,
        }
    }

    pub const fn for_third_party() -> Self {
        Self {
            disable_shell_escape: true,
            only_cached_bundle: true,
            paranoid_openout: true,
        }
    }

    pub fn is_sandboxed(&self) -> bool {
        self.disable_shell_escape || self.only_cached_bundle || self.paranoid_openout
    }
}

/// Backwards-compatible wrapper — delegates to `compile_and_write_sandboxed`
/// with a no-op directive. Every pre-spec-007 call site continues to work.
#[allow(clippy::too_many_arguments)]
pub fn compile_and_write(
    cfg: &Config,
    tex_path: &Path,
    engine: SupportedEngine,
    output_pdf: &Path,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
) -> Result<CompileOutcome, TexError> {
    compile_and_write_sandboxed(
        cfg,
        tex_path,
        engine,
        output_pdf,
        keep_tex,
        keep_logs,
        force,
        verbose,
        SandboxDirective::for_builtin(),
        None,
    )
}

/// Spec 007 US2 entry point — same as `compile_and_write` but threads a
/// sandbox directive through the engine invocation. `identifier_for_errors`
/// carries the template identifier so a sandbox violation can be attributed
/// to the offending template (per FR-017 error contract).
#[allow(clippy::too_many_arguments)]
pub fn compile_and_write_sandboxed(
    _cfg: &Config,
    tex_path: &Path,
    engine: SupportedEngine,
    output_pdf: &Path,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
    sandbox: SandboxDirective,
    identifier_for_errors: Option<&str>,
) -> Result<CompileOutcome, TexError> {
    if !tex_path.exists() {
        return Err(TexError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("file {} not found", tex_path.display()),
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
    run_engine(engine, temp.path(), &tex_filename, verbose, sandbox).map_err(|(_, log_tail)| {
        // Detect canonical sandbox-violation signatures in the log tail
        // and surface them as spec-007 SandboxError variants (FR-017).
        if sandbox.is_sandboxed() {
            if let Some(id) = identifier_for_errors {
                let lower = log_tail.to_ascii_lowercase();
                if lower.contains("write18")
                    || lower.contains("shell escape")
                    || lower.contains("shell-escape")
                {
                    return TexError::SandboxShellEscapeAttempted {
                        identifier: id.to_string(),
                    };
                }
                if lower.contains("cache") && lower.contains("bundle") {
                    return TexError::SandboxBundleMissing;
                }
            }
        }
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
