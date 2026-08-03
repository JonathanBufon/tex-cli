use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::compiler::{self, SupportedEngine};
use crate::config::Config;
use crate::discovery::{self, Identifier, ResolveInputs};
use crate::errors::TexError;
use crate::templates::Version;
use crate::{atomic, render, templates};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    pub pdf_path: PathBuf,
    pub total_duration: Duration,
    pub render_duration: Duration,
    pub compile_duration: Duration,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
    pub kept_tex: bool,
    pub kept_logs: bool,
    pub intermediate_tex_path: Option<PathBuf>,
    /// Identifier of the template used — bare name for built-in, `namespace/name`
    /// for installed third-party (populated by Phase 4). Spec 007 FR-010.
    pub template_identifier: Identifier,
    /// Version of the template used — `None` for built-in, `Some` for installed.
    pub template_version: Option<Version>,
}

pub fn resolve_output_pdf(cfg: &Config, template_name: &str, output: Option<&Path>) -> PathBuf {
    match output {
        Some(p) => p.to_path_buf(),
        None => cfg.paths.output_dir.join(format!("{template_name}.pdf")),
    }
}

pub fn resolve_intermediate_tex_path(cfg: &Config, template_name: &str) -> PathBuf {
    cfg.paths.output_dir.join(format!("{template_name}.tex"))
}

/// Full JSON → PDF pipeline: read template + parse JSON + render +
/// (optionally save intermediate .tex) + compile + write PDF.
///
/// Ordering (research D-02): when `keep_tex=true`, the rendered .tex
/// is written to `<output_dir>/<name>.tex` **before** the compile
/// call, so a compile failure leaves the intermediate file on disk
/// for the user to inspect (FR-15).
#[allow(clippy::too_many_arguments)]
pub fn build_pipeline(
    cfg: &Config,
    template_name: &str,
    data_source: &str,
    output_pdf: &Path,
    engine: SupportedEngine,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
) -> Result<BuildOutcome, TexError> {
    let start_total = Instant::now();

    // 1-2. Read template.
    let template_bytes = templates::read_template(&cfg.paths.templates_dir, template_name)?;
    let template_src = std::str::from_utf8(&template_bytes).map_err(|e| TexError::InvalidUtf8 {
        source_path: cfg.paths.templates_dir.join(format!("{template_name}.tex")),
        detail: e.to_string(),
    })?;

    // 3. Parse JSON.
    let value = render::load_json_source(data_source)?;

    // 4-6. Render.
    let start_render = Instant::now();
    let rendered = render::render_template(template_name, template_src, &value)?;
    let render_duration = start_render.elapsed();
    tracing::info!("Render finished in {}ms", render_duration.as_millis());

    // 7. If keep_tex, write intermediate .tex to output_dir BEFORE compile
    // so a compile failure preserves it for debugging (FR-15).
    let intermediate_tex_path = if keep_tex {
        let path = resolve_intermediate_tex_path(cfg, template_name);
        atomic::write_atomic(&path, rendered.as_bytes(), 0o644)?;
        Some(path)
    } else {
        None
    };

    // 8-10. Write rendered .tex into TempDir for the compile step.
    let temp = tempfile::TempDir::new()?;
    let tex_filename = format!("{template_name}.tex");
    let tex_in_temp = temp.path().join(&tex_filename);
    fs::write(&tex_in_temp, &rendered)?;

    // 11-13. Compile. Pass keep_tex=false because we already handled
    // the .tex ourselves in step 7 (research D-06).
    let start_compile = Instant::now();
    tracing::info!("Compiling with engine '{engine}'...");
    let compile_outcome = compiler::compile_and_write(
        cfg,
        &tex_in_temp,
        engine,
        output_pdf,
        false,
        keep_logs,
        force,
        verbose,
    )?;
    let compile_duration = start_compile.elapsed();
    tracing::info!("Compile finished in {:.2}s", compile_duration.as_secs_f32());

    let template_identifier = Identifier::builtin(strip_tex_suffix(template_name))
        .map_err(|_| TexError::TemplateNotFound {
            name: template_name.to_string(),
            templates_dir: cfg.paths.templates_dir.clone(),
        })?;

    Ok(BuildOutcome {
        pdf_path: output_pdf.to_path_buf(),
        total_duration: start_total.elapsed(),
        render_duration,
        compile_duration,
        bytes_written: compile_outcome.bytes_written,
        overwrote_existing: compile_outcome.overwrote_existing,
        kept_tex: keep_tex,
        kept_logs: keep_logs,
        intermediate_tex_path,
        template_identifier,
        template_version: None,
    })
}

fn strip_tex_suffix(name: &str) -> &str {
    name.strip_suffix(".tex").unwrap_or(name)
}

/// Spec 007 US1 entry point: read JSON, resolve the template via
/// [`discovery::resolve`], then delegate to [`build_pipeline`].
///
/// US2 (installed templates + trust prompt + sandbox) extends this
/// function in Phase 4 by (a) enumerating installed templates for
/// `ResolveInputs::installed`, (b) intercepting third-party matches to
/// check the trust file before compile, and (c) passing a sandbox
/// directive through the compiler.
#[allow(clippy::too_many_arguments)]
pub fn build_pipeline_from_json(
    cfg: &Config,
    json_source: &str,
    output_override: Option<&Path>,
    engine: SupportedEngine,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
) -> Result<BuildOutcome, TexError> {
    let json = render::load_json_source(json_source)?;
    let builtins = templates::list_builtin_identifiers(&cfg.paths.templates_dir)?;
    let installed = Vec::new(); // Phase 4 populates this.
    let resolved = discovery::resolve(
        &json,
        ResolveInputs {
            builtins: &builtins,
            installed: &installed,
        },
    )?;

    let template_name = resolved.identifier.name.clone();
    let output_pdf = match output_override {
        Some(p) => p.to_path_buf(),
        None => resolve_output_pdf(cfg, &template_name, None),
    };

    // Delegate. The delegated pipeline re-parses the JSON — accepted cost
    // (~milliseconds); avoids branching the render/compile core.
    let mut outcome = build_pipeline(
        cfg,
        &template_name,
        json_source,
        &output_pdf,
        engine,
        keep_tex,
        keep_logs,
        force,
        verbose,
    )?;

    // Overwrite the identifier with the fully-formed one from the resolver
    // (namespace preserved for future third-party path; None version for built-in).
    outcome.template_identifier = resolved.identifier;
    outcome.template_version = resolved.version;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BehaviorConfig, CompilerConfig, Config, PathsConfig};

    fn sample_config() -> Config {
        Config {
            paths: PathsConfig {
                templates_dir: PathBuf::from("/tpl"),
                output_dir: PathBuf::from("/out"),
            },
            compiler: CompilerConfig {
                engine: "tectonic".into(),
                keep_tex: false,
                keep_logs: false,
            },
            behavior: BehaviorConfig {
                ask_output_path_every_time: false,
            },
        }
    }

    #[test]
    fn resolve_output_pdf_default() {
        let cfg = sample_config();
        let path = resolve_output_pdf(&cfg, "artigo", None);
        assert_eq!(path, PathBuf::from("/out/artigo.pdf"));
    }

    #[test]
    fn resolve_output_pdf_custom() {
        let cfg = sample_config();
        let custom = PathBuf::from("/tmp/custom.pdf");
        let path = resolve_output_pdf(&cfg, "artigo", Some(&custom));
        assert_eq!(path, custom);
    }

    #[test]
    fn resolve_intermediate_tex_path_uses_output_dir() {
        let cfg = sample_config();
        let path = resolve_intermediate_tex_path(&cfg, "artigo");
        assert_eq!(path, PathBuf::from("/out/artigo.tex"));
    }
}
