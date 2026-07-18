use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::errors::TexError;
use crate::templates::read_template;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutcome {
    pub output_path: Option<PathBuf>,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
    pub dry_run: bool,
}

pub fn render_template(
    template_name: &str,
    template_src: &str,
    json_value: &serde_json::Value,
) -> Result<String, TexError> {
    if !json_value.is_object() {
        return Err(TexError::InvalidJson {
            source_name: format!("template {template_name}"),
            detail: format!(
                "expected top-level object, got {}",
                type_name(json_value)
            ),
        });
    }

    let ctx = tera::Context::from_value(json_value.clone()).map_err(|e| TexError::InvalidJson {
        source_name: format!("template {template_name}"),
        detail: e.to_string(),
    })?;

    tera::Tera::one_off(template_src, &ctx, false).map_err(|e| TexError::TeraRenderError {
        template_name: template_name.to_string(),
        detail: friendly_tera_error(&e),
    })
}

fn friendly_tera_error(e: &tera::Error) -> String {
    let mut msg = e.to_string();
    let mut source: Option<&dyn std::error::Error> = std::error::Error::source(e);
    while let Some(inner) = source {
        msg.push_str(" — ");
        msg.push_str(&inner.to_string());
        source = std::error::Error::source(inner);
    }
    msg
}

fn type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
    }
}

pub fn load_json_source(source: &str) -> Result<serde_json::Value, TexError> {
    let (content, source_display) = if source == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        (buf, "stdin".to_string())
    } else {
        let content = fs::read_to_string(source).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
                path: PathBuf::from(source),
            },
            _ => TexError::Io(e),
        })?;
        (content, source.to_string())
    };

    if content.trim().is_empty() {
        return Err(TexError::InvalidJson {
            source_name: source_display,
            detail: "empty content".to_string(),
        });
    }

    serde_json::from_str(&content).map_err(|e| TexError::InvalidJson {
        source_name: source_display,
        detail: e.to_string(),
    })
}

pub fn resolve_output_path(cfg: &Config, template_name: &str, output: Option<&Path>) -> PathBuf {
    match output {
        Some(p) => p.to_path_buf(),
        None => cfg.paths.output_dir.join(format!("{template_name}.tex")),
    }
}

/// Executes the full render pipeline: load template + JSON, render, write.
///
/// `force` and `dry_run` semantics live at the caller — this function
/// signals overwrite intent purely through the return value.
///
/// When `dry_run` is true, writes to stdout instead of the filesystem.
pub fn render_and_write(
    cfg: &Config,
    template_name: &str,
    data_source: &str,
    output: Option<&Path>,
    force: bool,
    dry_run: bool,
) -> Result<RenderOutcome, TexError> {
    use std::io::Write as _;

    let template_bytes = read_template(&cfg.paths.templates_dir, template_name)?;
    let template_src = std::str::from_utf8(&template_bytes).map_err(|e| TexError::InvalidUtf8 {
        source_path: cfg.paths.templates_dir.join(format!("{template_name}.tex")),
        detail: e.to_string(),
    })?;

    let json = load_json_source(data_source)?;
    let rendered = render_template(template_name, template_src, &json)?;

    if dry_run {
        std::io::stdout().write_all(rendered.as_bytes())?;
        return Ok(RenderOutcome {
            output_path: None,
            bytes_written: rendered.len() as u64,
            overwrote_existing: false,
            dry_run: true,
        });
    }

    let output_path = resolve_output_path(cfg, template_name, output);
    let overwrote_existing = output_path.exists();

    if overwrote_existing && !force {
        return Err(TexError::UserAborted);
    }

    crate::atomic::write_atomic(&output_path, rendered.as_bytes(), 0o644)?;

    Ok(RenderOutcome {
        output_path: Some(output_path),
        bytes_written: rendered.len() as u64,
        overwrote_existing,
        dry_run: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_template_basic() {
        let out = render_template("test", "Olá, {{ nome }}.", &json!({"nome":"X"})).unwrap();
        assert_eq!(out, "Olá, X.");
    }

    #[test]
    fn render_template_missing_var_returns_tera_error() {
        let err = render_template("test", "Olá, {{ ausente }}.", &json!({})).unwrap_err();
        match err {
            TexError::TeraRenderError {
                template_name,
                detail,
            } => {
                assert_eq!(template_name, "test");
                assert!(detail.to_lowercase().contains("ausente") || detail.contains("Variable"));
            }
            other => panic!("expected TeraRenderError, got {other:?}"),
        }
    }

    #[test]
    fn render_template_uses_default_filter() {
        let out = render_template("test", r#"{{ x | default(value="Y") }}"#, &json!({})).unwrap();
        assert_eq!(out, "Y");
    }

    #[test]
    fn render_template_nested_access() {
        let out =
            render_template("test", "{{ obj.chave }}", &json!({"obj":{"chave":"Z"}})).unwrap();
        assert_eq!(out, "Z");
    }

    #[test]
    fn render_template_no_autoescape_for_latex() {
        let out = render_template("test", "{{ x }}", &json!({"x":"& % $"})).unwrap();
        assert_eq!(out, "& % $");
    }

    #[test]
    fn render_template_top_level_array_returns_invalid_json() {
        let err = render_template("test", "x", &json!(["a"])).unwrap_err();
        match err {
            TexError::InvalidJson { detail, .. } => {
                assert!(detail.contains("array") || detail.contains("object"));
            }
            other => panic!("expected InvalidJson, got {other:?}"),
        }
    }

    #[test]
    fn load_json_source_reads_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("d.json");
        std::fs::write(&path, br#"{"a":1}"#).unwrap();
        let v = load_json_source(path.to_str().unwrap()).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn load_json_source_empty_returns_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("empty.json");
        std::fs::write(&path, b"").unwrap();
        let err = load_json_source(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, TexError::InvalidJson { .. }));
    }

    #[test]
    fn load_json_source_malformed_returns_invalid_json_with_line() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, b"{ this is not\n valid").unwrap();
        let err = load_json_source(path.to_str().unwrap()).unwrap_err();
        match err {
            TexError::InvalidJson { detail, .. } => {
                assert!(detail.contains("line") || detail.contains("column"));
            }
            other => panic!("expected InvalidJson, got {other:?}"),
        }
    }
}
