use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::errors::TexError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedTemplate {
    pub name: String,
    pub path: PathBuf,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Template {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_at_epoch: u64,
}

pub fn list_templates(dir: &Path) -> Result<Vec<Template>, TexError> {
    if !dir.exists() || !dir.is_dir() {
        return Err(TexError::TemplatesDirMissing {
            templates_dir: dir.to_path_buf(),
        });
    }

    let read_dir = fs::read_dir(dir).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: dir.to_path_buf(),
        },
        _ => TexError::Io(e),
    })?;

    let mut templates = Vec::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("tex") {
            continue;
        }

        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => {
                tracing::warn!(?path, "skipping template with non-UTF8 name");
                continue;
            }
        };

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified_at_epoch = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        templates.push(Template {
            name: file_stem,
            path,
            size_bytes: meta.len(),
            modified_at_epoch,
        });
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

pub fn resolve_template(dir: &Path, name: &str) -> Result<PathBuf, TexError> {
    if !dir.exists() || !dir.is_dir() {
        return Err(TexError::TemplatesDirMissing {
            templates_dir: dir.to_path_buf(),
        });
    }

    let candidate = if name.ends_with(".tex") {
        dir.join(name)
    } else {
        dir.join(format!("{name}.tex"))
    };

    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(TexError::TemplateNotFound {
            name: name.to_string(),
            templates_dir: dir.to_path_buf(),
        })
    }
}

pub fn is_utf8_ok(bytes: &[u8]) -> Result<(), String> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn add_template(
    dir: &Path,
    source: &Path,
    name: Option<&str>,
    force: bool,
) -> Result<AddedTemplate, TexError> {
    if !dir.exists() || !dir.is_dir() {
        return Err(TexError::TemplatesDirMissing {
            templates_dir: dir.to_path_buf(),
        });
    }

    let bytes = fs::read(source).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: source.to_path_buf(),
        },
        _ => TexError::Io(e),
    })?;

    if let Err(detail) = is_utf8_ok(&bytes) {
        return Err(TexError::InvalidUtf8 {
            source_path: source.to_path_buf(),
            detail,
        });
    }

    let dest_name = match name {
        Some(n) => n.to_string(),
        None => source
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                TexError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "source path has no usable file stem",
                ))
            })?
            .to_string(),
    };

    let dest_path = dir.join(format!("{dest_name}.tex"));
    let overwrote_existing = dest_path.exists();

    if overwrote_existing && !force {
        return Err(TexError::UserAborted);
    }

    write_atomic_0644(&dest_path, &bytes)?;

    Ok(AddedTemplate {
        name: dest_name,
        path: dest_path,
        bytes_written: bytes.len() as u64,
        overwrote_existing,
    })
}

fn write_atomic_0644(target: &Path, bytes: &[u8]) -> Result<(), TexError> {
    let parent = target.parent().ok_or_else(|| {
        TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "target has no parent dir",
        ))
    })?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: parent.to_path_buf(),
        },
        _ => TexError::Io(e),
    })?;

    tmp.as_file_mut().write_all(bytes)?;
    tmp.as_file_mut().sync_all()?;

    let mut perms = tmp.as_file().metadata()?.permissions();
    perms.set_mode(0o644);
    tmp.as_file().set_permissions(perms)?;

    tmp.persist(target).map_err(|e| match e.error.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
            path: target.to_path_buf(),
        },
        _ => TexError::Io(e.error),
    })?;

    Ok(())
}

pub fn read_template(dir: &Path, name: &str) -> Result<Vec<u8>, TexError> {
    let path = resolve_template(dir, name)?;
    fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied { path },
        _ => TexError::Io(e),
    })
}

pub fn render_template_list_humano(dir: &Path, templates: &[Template]) -> String {
    if templates.is_empty() {
        return format!("Nenhum template encontrado em {}.\n", dir.display());
    }

    let mut out = String::new();
    out.push_str("NOME                  TAMANHO   MODIFICADO\n");
    for t in templates {
        let name = truncate_name(&t.name, 20);
        let modified = format_epoch_local(t.modified_at_epoch);
        out.push_str(&format!("{:<20}  {:>8}   {}\n", name, t.size_bytes, modified));
    }
    out
}

fn truncate_name(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        name.to_string()
    } else {
        let mut s: String = name.chars().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

/// Formats a Unix epoch as `YYYY-MM-DD HH:MM` in UTC.
///
/// Manual conversion — avoids adding chrono/time to the canonical
/// stack (research D-01). Precision is minute-level; that's the
/// granularity we surface to humans anyway.
fn format_epoch_local(epoch: u64) -> String {
    if epoch == 0 {
        return "-".to_string();
    }
    let (year, month, day, hour, minute) = epoch_to_ymd_hm(epoch);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

fn epoch_to_ymd_hm(mut epoch: u64) -> (u64, u64, u64, u64, u64) {
    let minute = (epoch / 60) % 60;
    let hour = (epoch / 3600) % 24;
    let mut days = epoch / 86_400;
    epoch %= 86_400;
    let _ = epoch;

    let mut year: u64 = 1970;
    loop {
        let year_days = if is_leap(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }

    let mut month: u64 = 1;
    loop {
        let dim = days_in_month(year, month);
        if days < dim {
            break;
        }
        days -= dim;
        month += 1;
    }
    let day = days + 1;

    (year, month, day, hour, minute)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn days_in_month(y: u64, m: u64) -> u64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn list_templates_returns_only_tex() {
        let tmp = tempfile::tempdir().unwrap();
        seed(tmp.path(), "artigo.tex", "% ok");
        seed(tmp.path(), "refs.bib", "% not a template");
        seed(tmp.path(), "img.png", "\x00\x01");

        let templates = list_templates(tmp.path()).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "artigo");
    }

    #[test]
    fn list_templates_sorts_alphabetically() {
        let tmp = tempfile::tempdir().unwrap();
        seed(tmp.path(), "zebra.tex", "");
        seed(tmp.path(), "alpha.tex", "");
        seed(tmp.path(), "mid.tex", "");

        let templates = list_templates(tmp.path()).unwrap();
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mid", "zebra"]);
    }

    #[test]
    fn list_templates_missing_dir_returns_templates_dir_missing() {
        let missing = std::path::PathBuf::from("/definitely/does/not/exist/tex-cli");
        let err = list_templates(&missing).unwrap_err();
        match err {
            TexError::TemplatesDirMissing { templates_dir } => {
                assert_eq!(templates_dir, missing);
            }
            other => panic!("expected TemplatesDirMissing, got {other:?}"),
        }
    }

    #[test]
    fn list_templates_empty_dir_returns_empty_vec() {
        let tmp = tempfile::tempdir().unwrap();
        let templates = list_templates(tmp.path()).unwrap();
        assert_eq!(templates.len(), 0);
    }

    #[test]
    fn list_templates_ignores_subdirs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        seed(tmp.path(), "root.tex", "");
        seed(&tmp.path().join("sub"), "nested.tex", "");

        let templates = list_templates(tmp.path()).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "root");
    }

    #[test]
    fn template_serializes_to_expected_json_shape() {
        let t = Template {
            name: "artigo".into(),
            path: PathBuf::from("/tpl/artigo.tex"),
            size_bytes: 42,
            modified_at_epoch: 1_700_000_000,
        };
        let s = serde_json::to_string(&t).unwrap();
        assert!(s.contains(r#""name":"artigo""#));
        assert!(s.contains(r#""path":"/tpl/artigo.tex""#));
        assert!(s.contains(r#""size_bytes":42"#));
        assert!(s.contains(r#""modified_at_epoch":1700000000"#));
    }

    #[test]
    fn render_humano_empty_dir_prints_placeholder() {
        let out = render_template_list_humano(Path::new("/tpl"), &[]);
        assert!(out.contains("Nenhum template encontrado em /tpl"));
    }

    #[test]
    fn render_humano_lists_all_columns() {
        let templates = vec![Template {
            name: "artigo".into(),
            path: PathBuf::from("/tpl/artigo.tex"),
            size_bytes: 1234,
            modified_at_epoch: 1_700_000_000,
        }];
        let out = render_template_list_humano(Path::new("/tpl"), &templates);
        assert!(out.contains("NOME"));
        assert!(out.contains("TAMANHO"));
        assert!(out.contains("MODIFICADO"));
        assert!(out.contains("artigo"));
        assert!(out.contains("1234"));
    }

    #[test]
    fn epoch_conversion_known_value() {
        // 2023-11-14 22:13:20 UTC = 1700000000
        let (y, mo, d, h, mi) = epoch_to_ymd_hm(1_700_000_000);
        assert_eq!((y, mo, d, h, mi), (2023, 11, 14, 22, 13));
    }

    #[test]
    fn epoch_conversion_epoch_zero() {
        let (y, mo, d, h, mi) = epoch_to_ymd_hm(0);
        assert_eq!((y, mo, d, h, mi), (1970, 1, 1, 0, 0));
    }

    #[test]
    fn resolve_with_extension_finds_file() {
        let tmp = tempfile::tempdir().unwrap();
        seed(tmp.path(), "artigo.tex", "");
        let path = resolve_template(tmp.path(), "artigo.tex").unwrap();
        assert_eq!(path, tmp.path().join("artigo.tex"));
    }

    #[test]
    fn resolve_without_extension_finds_file() {
        let tmp = tempfile::tempdir().unwrap();
        seed(tmp.path(), "artigo.tex", "");
        let path = resolve_template(tmp.path(), "artigo").unwrap();
        assert_eq!(path, tmp.path().join("artigo.tex"));
    }

    #[test]
    fn resolve_case_sensitive() {
        let tmp = tempfile::tempdir().unwrap();
        seed(tmp.path(), "artigo.tex", "lower");
        seed(tmp.path(), "Artigo.tex", "upper");
        assert_eq!(
            resolve_template(tmp.path(), "artigo").unwrap(),
            tmp.path().join("artigo.tex")
        );
        assert_eq!(
            resolve_template(tmp.path(), "Artigo").unwrap(),
            tmp.path().join("Artigo.tex")
        );
    }

    #[test]
    fn resolve_missing_returns_template_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_template(tmp.path(), "parecer").unwrap_err();
        match err {
            TexError::TemplateNotFound { name, templates_dir } => {
                assert_eq!(name, "parecer");
                assert_eq!(templates_dir, tmp.path());
            }
            other => panic!("expected TemplateNotFound, got {other:?}"),
        }
    }

    #[test]
    fn resolve_missing_dir_returns_templates_dir_missing() {
        let missing = PathBuf::from("/definitely/not/here/tex-cli");
        let err = resolve_template(&missing, "anything").unwrap_err();
        assert!(matches!(err, TexError::TemplatesDirMissing { .. }));
    }

    #[test]
    fn is_utf8_ok_accepts_ascii() {
        assert!(is_utf8_ok(b"hello world\n").is_ok());
    }

    #[test]
    fn is_utf8_ok_accepts_multibyte() {
        assert!(is_utf8_ok("olá, mundo — 🚀".as_bytes()).is_ok());
    }

    #[test]
    fn is_utf8_ok_rejects_invalid_continuation() {
        assert!(is_utf8_ok(&[0xFFu8, 0xFEu8, 0xFDu8]).is_err());
    }

    #[test]
    fn add_template_creates_new_with_0644() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src.tex");
        std::fs::write(&source, b"% content\n").unwrap();
        let templates = tmp.path().join("dest");
        std::fs::create_dir(&templates).unwrap();

        let outcome = add_template(&templates, &source, None, false).unwrap();
        assert_eq!(outcome.name, "src");
        assert!(!outcome.overwrote_existing);
        assert_eq!(outcome.bytes_written, 10);

        let mode = std::fs::metadata(&outcome.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o644);
    }

    #[test]
    fn add_template_binary_returns_invalid_utf8() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("bin.tex");
        std::fs::write(&source, &[0xFFu8, 0xFEu8]).unwrap();
        let templates = tmp.path().join("dest");
        std::fs::create_dir(&templates).unwrap();

        let err = add_template(&templates, &source, None, false).unwrap_err();
        assert!(matches!(err, TexError::InvalidUtf8 { .. }));
        assert!(!templates.join("bin.tex").exists());
    }

    #[test]
    fn add_template_existing_without_force_aborts() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src.tex");
        std::fs::write(&source, b"new").unwrap();
        let templates = tmp.path().join("dest");
        std::fs::create_dir(&templates).unwrap();
        std::fs::write(templates.join("src.tex"), b"old").unwrap();

        let err = add_template(&templates, &source, None, false).unwrap_err();
        assert!(matches!(err, TexError::UserAborted));
        assert_eq!(std::fs::read(templates.join("src.tex")).unwrap(), b"old");
    }

    #[test]
    fn add_template_force_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src.tex");
        std::fs::write(&source, b"new").unwrap();
        let templates = tmp.path().join("dest");
        std::fs::create_dir(&templates).unwrap();
        std::fs::write(templates.join("src.tex"), b"old").unwrap();

        let outcome = add_template(&templates, &source, None, true).unwrap();
        assert!(outcome.overwrote_existing);
        assert_eq!(std::fs::read(&outcome.path).unwrap(), b"new");
    }

    #[test]
    fn add_template_custom_name() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src.tex");
        std::fs::write(&source, b"body").unwrap();
        let templates = tmp.path().join("dest");
        std::fs::create_dir(&templates).unwrap();

        let outcome = add_template(&templates, &source, Some("outro"), false).unwrap();
        assert_eq!(outcome.name, "outro");
        assert!(templates.join("outro.tex").exists());
        assert!(!templates.join("src.tex").exists());
    }

    #[test]
    fn read_template_returns_bytes() {
        let tmp = tempfile::tempdir().unwrap();
        let payload = b"\\documentclass{article}\n";
        std::fs::write(tmp.path().join("artigo.tex"), payload).unwrap();
        let bytes = read_template(tmp.path(), "artigo").unwrap();
        assert_eq!(bytes, payload);
    }

    #[test]
    fn leap_year_handling() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
    }
}
