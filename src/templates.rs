use std::cmp::Ordering;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::discovery::Identifier;
use crate::errors::TexError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedTemplate {
    pub name: String,
    pub path: PathBuf,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
}

/// Semver-lite: MAJOR.MINOR.PATCH with optional `-<pre>` and `+<build>`.
///
/// Ordering is numeric on (major, minor, patch); pre/build are recorded
/// but do not participate in ordering (v1 keeps the comparison simple —
/// FR-016 only needs equality for the re-prompt-on-upgrade check).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
    pub build: Option<String>,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: None,
            build: None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionParseError {
    pub value: String,
}

impl FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || VersionParseError {
            value: s.to_string(),
        };

        // Split build first: "1.2.3-alpha+ci" -> ("1.2.3-alpha", "ci")
        let (rest, build) = match s.split_once('+') {
            Some((r, b)) if !b.is_empty() && is_dot_ident(b) => (r, Some(b.to_string())),
            Some(_) => return Err(err()),
            None => (s, None),
        };
        // Then pre: "1.2.3-alpha" -> ("1.2.3", "alpha")
        let (core, pre) = match rest.split_once('-') {
            Some((c, p)) if !p.is_empty() && is_dot_ident(p) => (c, Some(p.to_string())),
            Some(_) => return Err(err()),
            None => (rest, None),
        };

        let mut parts = core.split('.');
        let major = parts
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or(err())?;
        let minor = parts
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or(err())?;
        let patch = parts
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or(err())?;
        if parts.next().is_some() {
            return Err(err());
        }
        Ok(Self {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }
}

/// Validate that a pre/build identifier is a non-empty dot-separated list of
/// `[A-Za-z0-9-]+` segments (semver-compatible).
fn is_dot_ident(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|seg| {
            !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
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

    crate::atomic::write_atomic(&dest_path, &bytes, 0o644)?;

    Ok(AddedTemplate {
        name: dest_name,
        path: dest_path,
        bytes_written: bytes.len() as u64,
        overwrote_existing,
    })
}

pub fn remove_template(dir: &Path, name: &str, force: bool) -> Result<PathBuf, TexError> {
    if !dir.exists() || !dir.is_dir() {
        return Err(TexError::TemplatesDirMissing {
            templates_dir: dir.to_path_buf(),
        });
    }

    let path = resolve_template(dir, name)?;

    if !force {
        return Err(TexError::UserAborted);
    }

    fs::remove_file(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied { path: path.clone() },
        _ => TexError::Io(e),
    })?;

    Ok(path)
}

/// Spec 007 FR-006 / SC-004: scan `template_src` for known template-engine ↔
/// LaTeX macro syntax collisions BEFORE rendering, so the user gets a
/// targeted diagnostic that names the offending characters instead of an
/// opaque Tera parse error.
///
/// v1 covers the well-documented collision from spec 007 Context:
/// `\macro{#N}` — the `{#` opens a Tera comment which never closes,
/// producing a parse error unrelated to the LaTeX macro the author wrote.
pub fn check_template_collisions(template_name: &str, src: &str) -> Result<(), TexError> {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Find `\<letters>{#` — a LaTeX macro immediately followed by a
        // Tera comment opener.
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_alphabetic() {
            let macro_start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'#' {
                let (line, col) = line_col_of(src, macro_start);
                let macro_name = std::str::from_utf8(&bytes[macro_start..i]).unwrap_or("<?>");
                return Err(TexError::TeraRenderError {
                    template_name: template_name.to_string(),
                    detail: format!(
                        "template↔LaTeX syntax collision at line {line}, col {col}: \
                         the sequence '{{#' opens a Tera comment but appears inside \
                         a LaTeX macro argument (`{macro_name}{{#…}}`). Rewrite the \
                         macro argument (for example, define it as `\\newcommand{{\\{}[1]{{...}}` \
                         and call it, or move the argument into a variable) so the \
                         '#' is not adjacent to '{{'.",
                        macro_name.trim_start_matches('\\')
                    ),
                });
            }
            continue;
        }
        i += 1;
    }
    Ok(())
}

fn line_col_of(src: &str, idx: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in src.char_indices() {
        if i >= idx {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// The manifest file name required at the root of every third-party template
/// package (spec 007 FR-019 / contracts/manifest-schema.md).
pub const MANIFEST_FILE: &str = "tex-template.toml";

/// Loaded, validated `tex-template.toml` (spec 007 FR-019).
///
/// Access `identifier`, `version`, and the absolute path to the `.tex`
/// entrypoint after successful load.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub identifier: Identifier,
    pub version: Version,
    /// Relative to the package root — validated to remain within it.
    pub entrypoint: PathBuf,
    /// Absolute path resolved against `package_root`.
    pub entrypoint_abs: PathBuf,
}

impl Manifest {
    /// Load and fully validate `<package_root>/tex-template.toml`.
    pub fn load(package_root: &Path) -> Result<Self, TexError> {
        let manifest_path = package_root.join(MANIFEST_FILE);
        if !manifest_path.exists() {
            return Err(TexError::ManifestNotFound {
                path: manifest_path,
            });
        }
        let raw = fs::read_to_string(&manifest_path).map_err(TexError::Io)?;
        let table: toml::Table = toml::from_str(&raw).map_err(|e| TexError::ManifestParse {
            path: manifest_path.clone(),
            detail: e.to_string(),
        })?;

        let identifier_raw = require_str(&table, "identifier", &manifest_path)?;
        let version_raw = require_str(&table, "version", &manifest_path)?;
        let entrypoint_raw = require_str(&table, "entrypoint", &manifest_path)?;

        let identifier: Identifier =
            identifier_raw
                .parse()
                .map_err(|_| TexError::ManifestInvalidIdentifier {
                    value: identifier_raw.clone(),
                })?;
        // Third-party manifests MUST declare a namespace (FR-018/FR-019).
        if !identifier.is_third_party() {
            return Err(TexError::ManifestInvalidIdentifier {
                value: identifier_raw,
            });
        }
        let version: Version =
            version_raw
                .parse()
                .map_err(|_| TexError::ManifestInvalidVersion {
                    value: version_raw.clone(),
                })?;

        let entrypoint = PathBuf::from(&entrypoint_raw);
        if entrypoint.is_absolute() {
            return Err(TexError::ManifestEntrypointEscape {
                entrypoint: entrypoint.clone(),
            });
        }
        for c in entrypoint.components() {
            if matches!(c, std::path::Component::ParentDir) {
                return Err(TexError::ManifestEntrypointEscape {
                    entrypoint: entrypoint.clone(),
                });
            }
        }
        if entrypoint.extension().and_then(|s| s.to_str()) != Some("tex") {
            return Err(TexError::ManifestEntrypointNotTex {
                entrypoint: entrypoint.clone(),
            });
        }
        let entrypoint_abs = package_root.join(&entrypoint);
        if !entrypoint_abs.exists() || !entrypoint_abs.is_file() {
            return Err(TexError::ManifestEntrypointMissing {
                entrypoint: entrypoint.clone(),
            });
        }

        Ok(Self {
            identifier,
            version,
            entrypoint,
            entrypoint_abs,
        })
    }
}

fn require_str(table: &toml::Table, field: &'static str, path: &Path) -> Result<String, TexError> {
    let value = table
        .get(field)
        .ok_or_else(|| TexError::ManifestMissingField {
            path: path.to_path_buf(),
            field,
        })?;
    match value {
        toml::Value::String(s) if !s.is_empty() => Ok(s.clone()),
        _ => Err(TexError::ManifestMissingField {
            path: path.to_path_buf(),
            field,
        }),
    }
}

/// Enumerate every built-in template in `dir` as a bare-name `Identifier`.
///
/// Any file whose stem is not a valid built-in identifier (uppercase,
/// leading special char, …) is skipped with a warning; the pipeline keeps
/// working with the remaining valid templates.
pub fn list_builtin_identifiers(dir: &Path) -> Result<Vec<Identifier>, TexError> {
    let templates = list_templates(dir)?;
    let mut ids = Vec::with_capacity(templates.len());
    for t in templates {
        match Identifier::builtin(&t.name) {
            Ok(id) => ids.push(id),
            Err(_) => tracing::warn!(
                name = %t.name,
                "skipping built-in template with non-identifier name"
            ),
        }
    }
    Ok(ids)
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
        return format!("No templates found in {}.\n", dir.display());
    }

    let mut out = String::new();
    out.push_str("NAME                  SIZE      MODIFIED\n");
    for t in templates {
        let name = truncate_name(&t.name, 20);
        let modified = format_epoch_local(t.modified_at_epoch);
        out.push_str(&format!(
            "{:<20}  {:>8}   {}\n",
            name, t.size_bytes, modified
        ));
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
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
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
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn seed(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    // ---- Spec 007 FR-006 collision detection tests ----

    #[test]
    fn check_template_collisions_flags_makeuppercase_pattern() {
        let src = "\\renewcommand{\\MakeUppercase}[1]{\\uppercase{#1}}";
        let err = check_template_collisions("test", src).unwrap_err();
        match err {
            TexError::TeraRenderError { detail, .. } => {
                assert!(detail.contains("collision"));
                assert!(detail.contains("uppercase") || detail.contains("{#"));
            }
            other => panic!("expected TeraRenderError, got {other:?}"),
        }
    }

    #[test]
    fn check_template_collisions_passes_clean_template() {
        let src = "\\documentclass{article}\\begin{document}{{ x }}\\end{document}";
        assert!(check_template_collisions("test", src).is_ok());
    }

    #[test]
    fn check_template_collisions_ignores_tera_comment_outside_macro() {
        let src = "before {# this is a real Tera comment #} after";
        assert!(check_template_collisions("test", src).is_ok());
    }

    #[test]
    fn check_template_collisions_reports_line_and_column() {
        let src = "\\documentclass{article}\n\\begin{document}\n\\bad{#1}\n\\end{document}";
        let err = check_template_collisions("test", src).unwrap_err();
        match err {
            TexError::TeraRenderError { detail, .. } => {
                assert!(detail.contains("line 3"), "detail: {detail}");
            }
            other => panic!("expected TeraRenderError, got {other:?}"),
        }
    }

    #[test]
    fn version_parses_plain_semver() {
        let v: Version = "1.2.3".parse().unwrap();
        assert_eq!(v, Version::new(1, 2, 3));
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn version_parses_with_pre() {
        let v: Version = "1.0.0-beta".parse().unwrap();
        assert_eq!(v.pre.as_deref(), Some("beta"));
        assert_eq!(v.to_string(), "1.0.0-beta");
    }

    #[test]
    fn version_parses_with_build() {
        let v: Version = "1.0.0+ci".parse().unwrap();
        assert_eq!(v.build.as_deref(), Some("ci"));
        assert_eq!(v.to_string(), "1.0.0+ci");
    }

    #[test]
    fn version_parses_with_pre_and_build() {
        let v: Version = "1.0.0-alpha.1+ci.42".parse().unwrap();
        assert_eq!(v.pre.as_deref(), Some("alpha.1"));
        assert_eq!(v.build.as_deref(), Some("ci.42"));
        assert_eq!(v.to_string(), "1.0.0-alpha.1+ci.42");
    }

    #[test]
    fn version_rejects_missing_patch() {
        assert!("1.0".parse::<Version>().is_err());
    }

    #[test]
    fn version_rejects_extra_segment() {
        assert!("1.0.0.0".parse::<Version>().is_err());
    }

    #[test]
    fn version_rejects_non_numeric_core() {
        assert!("1.a.0".parse::<Version>().is_err());
        assert!("v1.0.0".parse::<Version>().is_err());
    }

    #[test]
    fn version_rejects_empty_pre_or_build() {
        assert!("1.0.0-".parse::<Version>().is_err());
        assert!("1.0.0+".parse::<Version>().is_err());
    }

    #[test]
    fn version_ordering_by_major_minor_patch() {
        let a: Version = "1.0.0".parse().unwrap();
        let b: Version = "1.0.1".parse().unwrap();
        let c: Version = "1.1.0".parse().unwrap();
        let d: Version = "2.0.0".parse().unwrap();
        assert!(a < b);
        assert!(b < c);
        assert!(c < d);
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
        assert!(out.contains("No templates found in /tpl"));
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
        assert!(out.contains("NAME"));
        assert!(out.contains("SIZE"));
        assert!(out.contains("MODIFIED"));
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
            TexError::TemplateNotFound {
                name,
                templates_dir,
            } => {
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

        let mode = std::fs::metadata(&outcome.path)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o644);
    }

    #[test]
    fn add_template_binary_returns_invalid_utf8() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("bin.tex");
        std::fs::write(&source, [0xFFu8, 0xFEu8]).unwrap();
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
    fn remove_template_deletes_when_force() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("x.tex"), b"content").unwrap();
        let removed = remove_template(tmp.path(), "x", true).unwrap();
        assert_eq!(removed, tmp.path().join("x.tex"));
        assert!(!removed.exists());
    }

    #[test]
    fn remove_template_without_force_aborts_and_keeps_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("x.tex"), b"content").unwrap();
        let err = remove_template(tmp.path(), "x", false).unwrap_err();
        assert!(matches!(err, TexError::UserAborted));
        assert!(tmp.path().join("x.tex").exists());
    }

    #[test]
    fn remove_template_nonexistent_returns_template_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let err = remove_template(tmp.path(), "nope", true).unwrap_err();
        assert!(matches!(err, TexError::TemplateNotFound { .. }));
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
