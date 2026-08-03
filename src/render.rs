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

/// Spec 007 FR-005 / A-06: one glyph substitution the renderer applied so
/// the caller can surface it to the user (Constitution V: never silently drop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharSubstitution {
    /// The original glyph in the source JSON string value.
    pub original: char,
    /// The LaTeX-safe replacement written to the rendered output.
    pub replacement: String,
    /// Dot-path in the JSON where the substitution happened (best-effort;
    /// empty for the top-level value).
    pub json_path: String,
}

pub fn render_template(
    template_name: &str,
    template_src: &str,
    json_value: &serde_json::Value,
) -> Result<String, TexError> {
    let (out, _warnings) = render_template_safe(template_name, template_src, json_value)?;
    Ok(out)
}

/// Spec 007 US3 entry point — universally escapes every JSON string value
/// (FR-005) before feeding tera, detects known template-engine ↔ LaTeX
/// syntax collisions BEFORE parsing (FR-006), and returns the list of
/// glyph substitutions performed (A-06) for the caller to surface.
pub fn render_template_safe(
    template_name: &str,
    template_src: &str,
    json_value: &serde_json::Value,
) -> Result<(String, Vec<CharSubstitution>), TexError> {
    if !json_value.is_object() {
        return Err(TexError::InvalidJson {
            source_name: format!("template {template_name}"),
            detail: format!("expected top-level object, got {}", type_name(json_value)),
        });
    }

    // FR-006: fail fast with a targeted diagnostic on known collisions.
    crate::templates::check_template_collisions(template_name, template_src)?;

    // FR-005 + A-06: escape/substitute every JSON string value.
    let mut warnings: Vec<CharSubstitution> = Vec::new();
    let escaped = escape_value_recursive(json_value.clone(), &mut warnings, String::new());
    for w in &warnings {
        tracing::warn!(
            char = %w.original,
            replacement = %w.replacement,
            path = %w.json_path,
            "substituted unrepresentable glyph in rendered output (spec 007 A-06)"
        );
    }

    let ctx = tera::Context::from_value(escaped).map_err(|e| TexError::InvalidJson {
        source_name: format!("template {template_name}"),
        detail: e.to_string(),
    })?;

    let rendered =
        tera::Tera::one_off(template_src, &ctx, false).map_err(|e| TexError::TeraRenderError {
            template_name: template_name.to_string(),
            detail: friendly_tera_error(&e),
        })?;
    Ok((rendered, warnings))
}

/// FR-005: walk a JSON value and escape every string in-tree, recording
/// glyph substitutions (A-06) in `warnings`.
fn escape_value_recursive(
    v: serde_json::Value,
    warnings: &mut Vec<CharSubstitution>,
    path: String,
) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::String(s) => Value::String(escape_latex_string(&s, warnings, &path)),
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .enumerate()
                .map(|(i, x)| {
                    let child = if path.is_empty() {
                        format!("[{i}]")
                    } else {
                        format!("{path}[{i}]")
                    };
                    escape_value_recursive(x, warnings, child)
                })
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, x)| {
                    let child = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    let escaped = escape_value_recursive(x, warnings, child);
                    (k, escaped)
                })
                .collect(),
        ),
        other => other,
    }
}

/// FR-005: escape the ten LaTeX specials. A-06: substitute em/en-dashes
/// with LaTeX shorthand and record the substitution.
fn escape_latex_string(s: &str, warnings: &mut Vec<CharSubstitution>, path: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str(r"\textbackslash{}"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            '—' => {
                out.push_str("---");
                warnings.push(CharSubstitution {
                    original: '—',
                    replacement: "---".into(),
                    json_path: path.to_string(),
                });
            }
            '–' => {
                out.push_str("--");
                warnings.push(CharSubstitution {
                    original: '–',
                    replacement: "--".into(),
                    json_path: path.to_string(),
                });
            }
            other => out.push(other),
        }
    }
    out
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

    // Spec 007 FR-005 replaces the pre-007 "no-autoescape" contract with a
    // stronger promise: EVERY LaTeX special in a JSON string value is
    // escaped by the renderer before Tera sees the value.
    #[test]
    fn render_template_escapes_latex_specials_universally() {
        let out = render_template("test", "{{ x }}", &json!({"x":"& % $"})).unwrap();
        assert_eq!(out, r"\& \% \$");
    }

    #[test]
    fn render_template_escapes_all_ten_specials() {
        let out = render_template("test", "{{ x }}", &json!({"x":"& % $ # _ { } \\ ~ ^"})).unwrap();
        assert_eq!(
            out,
            r"\& \% \$ \# \_ \{ \} \textbackslash{} \textasciitilde{} \textasciicircum{}"
        );
    }

    #[test]
    fn render_template_substitutes_em_dash_and_reports() {
        let (out, warnings) = render_template_safe("test", "{{ x }}", &json!({"x":"a—b"})).unwrap();
        assert_eq!(out, "a---b");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].original, '—');
        assert_eq!(warnings[0].replacement, "---");
        assert_eq!(warnings[0].json_path, "x");
    }

    #[test]
    fn render_template_substitutes_en_dash_and_reports() {
        let (out, warnings) = render_template_safe("test", "{{ x }}", &json!({"x":"a–b"})).unwrap();
        assert_eq!(out, "a--b");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].original, '–');
        assert_eq!(warnings[0].replacement, "--");
    }

    #[test]
    fn render_template_escapes_nested_objects_and_arrays_with_paths() {
        let (out, warnings) = render_template_safe(
            "test",
            "{{ items.0.name }}",
            &json!({"items":[{"name":"a—b & c"}]}),
        )
        .unwrap();
        assert_eq!(out, r"a---b \& c");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].json_path, "items[0].name");
    }

    #[test]
    fn render_template_no_warnings_when_no_substitutions_needed() {
        let (_out, warnings) =
            render_template_safe("test", "{{ x }}", &json!({"x":"just plain ascii"})).unwrap();
        assert!(warnings.is_empty());
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
