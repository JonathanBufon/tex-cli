use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::compiler::{self, SandboxDirective, SupportedEngine};
use crate::config::Config;
use crate::discovery::{self, Identifier, InstalledCandidate, ResolveInputs};
use crate::errors::TexError;
use crate::install::{self, InstalledPackage};
use crate::templates::{Manifest, Version};
use crate::{atomic, paths, render, templates, trust};

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

/// Spec 007 US1 + US2 entry point: read JSON, resolve the template via
/// [`discovery::resolve`], then dispatch to the built-in or installed
/// third-party compile path.
///
/// For an installed third-party match this function additionally:
///   * consults the trust file and prompts the user if the (identifier,
///     version) pair is not already approved (FR-016);
///   * reads the template body from the installed package's manifested
///     entrypoint;
///   * runs the compile inside the FR-017 sandbox.
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
    let installed_root = paths::templates_dir()?;
    let installed_packages = install::list_installed(&installed_root);
    let installed_candidates: Vec<InstalledCandidate<'_>> = installed_packages
        .iter()
        .map(|p| InstalledCandidate {
            identifier: &p.identifier,
            version: &p.version,
        })
        .collect();

    let resolved = discovery::resolve(
        &json,
        ResolveInputs {
            builtins: &builtins,
            installed: &installed_candidates,
        },
    )?;

    if resolved.identifier.is_third_party() {
        let package = installed_packages
            .iter()
            .find(|p| p.identifier == resolved.identifier)
            .ok_or_else(|| TexError::ExplicitTemplateMissing {
                requested: resolved.identifier.to_string(),
            })?;

        // FR-016 trust gate.
        let trust_path = paths::trust_file()?;
        trust::ensure_trusted(&trust_path, &package.identifier, &package.version)?;

        let output_pdf = match output_override {
            Some(p) => p.to_path_buf(),
            None => resolve_output_pdf(cfg, &resolved.identifier.name, None),
        };
        return build_pipeline_installed(
            cfg,
            package,
            &json,
            &output_pdf,
            engine,
            keep_tex,
            keep_logs,
            force,
            verbose,
        );
    }

    // Built-in path.
    let template_name = resolved.identifier.name.clone();
    let output_pdf = match output_override {
        Some(p) => p.to_path_buf(),
        None => resolve_output_pdf(cfg, &template_name, None),
    };

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
    outcome.template_identifier = resolved.identifier;
    outcome.template_version = resolved.version;
    Ok(outcome)
}

/// Compile an installed third-party template — reads the entrypoint file
/// from the package dir, renders with the JSON, and invokes the sandboxed
/// compile per FR-017.
#[allow(clippy::too_many_arguments)]
fn build_pipeline_installed(
    cfg: &Config,
    package: &InstalledPackage,
    json_value: &serde_json::Value,
    output_pdf: &Path,
    engine: SupportedEngine,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
) -> Result<BuildOutcome, TexError> {
    let start_total = Instant::now();

    // Re-load the manifest to get the entrypoint absolute path
    // (list_installed's InstalledPackage carries version + dest_dir only).
    let manifest = Manifest::load(&package.dest_dir)?;
    let template_bytes = fs::read(&manifest.entrypoint_abs).map_err(TexError::Io)?;
    let template_src = std::str::from_utf8(&template_bytes).map_err(|e| TexError::InvalidUtf8 {
        source_path: manifest.entrypoint_abs.clone(),
        detail: e.to_string(),
    })?;

    let template_name = package.identifier.name.clone();

    let start_render = Instant::now();
    let rendered = render::render_template(&template_name, template_src, json_value)?;
    let render_duration = start_render.elapsed();

    let intermediate_tex_path = if keep_tex {
        let path = resolve_intermediate_tex_path(cfg, &template_name);
        atomic::write_atomic(&path, rendered.as_bytes(), 0o644)?;
        Some(path)
    } else {
        None
    };

    let temp = tempfile::TempDir::new()?;
    let tex_filename = format!("{template_name}.tex");
    let tex_in_temp = temp.path().join(&tex_filename);
    fs::write(&tex_in_temp, &rendered)?;

    let start_compile = Instant::now();
    let sandbox = SandboxDirective::for_third_party();
    let compile_outcome = compiler::compile_and_write_sandboxed(
        cfg,
        &tex_in_temp,
        engine,
        output_pdf,
        false,
        keep_logs,
        force,
        verbose,
        sandbox,
        Some(&package.identifier.to_string()),
    )?;
    let compile_duration = start_compile.elapsed();

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
        template_identifier: package.identifier.clone(),
        template_version: Some(package.version.clone()),
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
