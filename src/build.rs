use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::compiler::{self, SupportedEngine};
use crate::config::Config;
use crate::errors::TexError;
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
    tracing::info!(
        "Compile finished in {:.2}s",
        compile_duration.as_secs_f32()
    );

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
    })
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
