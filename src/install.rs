//! Third-party template installation from Git URL or local filesystem path.
//!
//! See `specs/007-auto-pdf-pipeline/contracts/cli-template-subcommands.md`
//! and `specs/007-auto-pdf-pipeline/data-model.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::discovery::Identifier;
use crate::errors::TexError;
use crate::templates::{Manifest, Version};

/// Outcome of a successful install.
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub identifier: Identifier,
    pub version: Version,
    pub dest_dir: PathBuf,
}

/// Detect whether `source` looks like a Git URL. Deliberately permissive —
/// anything with a scheme separator (`://`), a git-style user@host, or a
/// `.git` suffix is treated as a URL; everything else is a filesystem path.
pub fn looks_like_git_url(source: &str) -> bool {
    source.contains("://")
        || (source.starts_with("git@") && source.contains(':'))
        || source.trim_end_matches('/').ends_with(".git")
}

/// Install from a Git URL: `git clone` into a temp dir, then delegate to the
/// local-path installer against the checkout.
///
/// Requires the `git` binary on `PATH` (spec 007 research R2 — no `git2`
/// crate is added; the fixed dependency stack is preserved).
pub fn install_from_git(
    url: &str,
    templates_root: &Path,
    force: bool,
) -> Result<InstalledPackage, TexError> {
    which::which("git").map_err(|_| TexError::GitBinaryMissing)?;

    let tmp = tempfile::TempDir::new().map_err(TexError::Io)?;
    let checkout = tmp.path().join("pkg");
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(&checkout)
        .status()
        .map_err(|e| TexError::GitCloneFailed {
            url: url.to_string(),
            status: format!("spawn failed: {e}"),
        })?;
    if !status.success() {
        return Err(TexError::GitCloneFailed {
            url: url.to_string(),
            status: format!("exit {}", status.code().unwrap_or(-1)),
        });
    }

    install_from_local_path(&checkout, templates_root, force)
}

/// Install from a filesystem path: validate the manifest, derive destination
/// from `identifier`, honor `--force`, copy recursively.
pub fn install_from_local_path(
    source: &Path,
    templates_root: &Path,
    force: bool,
) -> Result<InstalledPackage, TexError> {
    if !source.exists() {
        return Err(TexError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("install source '{}' does not exist", source.display()),
        )));
    }
    if !source.is_dir() {
        return Err(TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "install source '{}' must be a directory (containing tex-template.toml)",
                source.display()
            ),
        )));
    }

    // Validate BEFORE copying — no partial-install artifacts on failure.
    let manifest = Manifest::load(source)?;
    let namespace = manifest
        .identifier
        .namespace
        .as_deref()
        .expect("Manifest::load enforces namespaced identifier");
    let name = &manifest.identifier.name;
    let dest_dir = templates_root.join(namespace).join(name);

    if dest_dir.exists() {
        if !force {
            return Err(TexError::InstallOverwrite {
                identifier: manifest.identifier.to_string(),
                version: manifest.version.to_string(),
                path: dest_dir,
            });
        }
        fs::remove_dir_all(&dest_dir).map_err(TexError::Io)?;
    }

    fs::create_dir_all(&dest_dir).map_err(TexError::Io)?;
    copy_dir_recursive(source, &dest_dir)?;

    // Re-load manifest from destination to confirm the copy landed correctly
    // and the layout matches — cheap safety net for FR-018/FR-019.
    let installed_manifest = Manifest::load(&dest_dir)?;
    if installed_manifest.identifier != manifest.identifier {
        // Should be unreachable, but defense-in-depth.
        return Err(TexError::InstallPathMismatch {
            manifest_id: installed_manifest.identifier.to_string(),
            dest_path: dest_dir,
        });
    }

    Ok(InstalledPackage {
        identifier: manifest.identifier,
        version: manifest.version,
        dest_dir,
    })
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), TexError> {
    for entry in fs::read_dir(src).map_err(TexError::Io)? {
        let entry = entry.map_err(TexError::Io)?;
        let file_type = entry.file_type().map_err(TexError::Io)?;
        let from = entry.path();
        // Skip common VCS metadata — reduces install footprint and avoids
        // shipping `.git/` from a clone.
        if let Some(name) = from.file_name().and_then(|s| s.to_str()) {
            if name == ".git" {
                continue;
            }
        }
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&to).map_err(TexError::Io)?;
            copy_dir_recursive(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(TexError::Io)?;
        }
        // Symlinks intentionally ignored (would let a malicious package
        // escape the destination directory).
    }
    Ok(())
}

/// Remove an installed template — deletes `<templates_root>/<ns>/<name>/`
/// and any empty parent namespace dir. Returns the path that was removed.
pub fn uninstall(identifier: &Identifier, templates_root: &Path) -> Result<PathBuf, TexError> {
    let namespace = identifier
        .namespace
        .as_deref()
        .ok_or_else(|| TexError::ExplicitTemplateMissing {
            requested: identifier.to_string(),
        })?;
    let dest_dir = templates_root.join(namespace).join(&identifier.name);
    if !dest_dir.exists() {
        return Err(TexError::ExplicitTemplateMissing {
            requested: identifier.to_string(),
        });
    }
    fs::remove_dir_all(&dest_dir).map_err(TexError::Io)?;
    // Best-effort: prune empty namespace dir.
    let ns_dir = templates_root.join(namespace);
    let _ = fs::remove_dir(&ns_dir); // fails silently if not empty — fine
    Ok(dest_dir)
}

/// Enumerate every installed template beneath `templates_root` (or empty vec
/// if the root does not exist yet). Corrupt entries log a warning and are
/// skipped; the pipeline keeps working.
pub fn list_installed(templates_root: &Path) -> Vec<InstalledPackage> {
    let mut out = Vec::new();
    let Ok(namespaces) = fs::read_dir(templates_root) else {
        return out;
    };
    for ns_entry in namespaces.flatten() {
        if !ns_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let ns_path = ns_entry.path();
        let Ok(names) = fs::read_dir(&ns_path) else {
            continue;
        };
        for name_entry in names.flatten() {
            if !name_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let pkg_dir = name_entry.path();
            match Manifest::load(&pkg_dir) {
                Ok(m) => out.push(InstalledPackage {
                    identifier: m.identifier,
                    version: m.version,
                    dest_dir: pkg_dir,
                }),
                Err(e) => {
                    tracing::warn!(
                        path = %pkg_dir.display(),
                        error = %e,
                        "skipping corrupt installed template package"
                    );
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_package(root: &Path, identifier: &str, version: &str) -> PathBuf {
        let pkg = root.join("pkg");
        fs::create_dir_all(&pkg).unwrap();
        let manifest = format!(
            r#"identifier = "{identifier}"
version = "{version}"
entrypoint = "template.tex"
"#
        );
        fs::write(pkg.join("tex-template.toml"), manifest).unwrap();
        fs::write(pkg.join("template.tex"), "\\documentclass{article}\n").unwrap();
        pkg
    }

    #[test]
    fn install_from_local_path_places_package_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let src = seed_package(tmp.path(), "acme/invoice", "1.0.0");
        let root = tmp.path().join("templates");

        let installed = install_from_local_path(&src, &root, false).unwrap();
        assert_eq!(installed.identifier.to_string(), "acme/invoice");
        assert_eq!(installed.version.to_string(), "1.0.0");
        assert_eq!(installed.dest_dir, root.join("acme").join("invoice"));
        assert!(installed.dest_dir.join("tex-template.toml").exists());
        assert!(installed.dest_dir.join("template.tex").exists());
    }

    #[test]
    fn install_refuses_overwrite_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        let src = seed_package(tmp.path(), "acme/invoice", "1.0.0");
        let root = tmp.path().join("templates");
        install_from_local_path(&src, &root, false).unwrap();
        let err = install_from_local_path(&src, &root, false).unwrap_err();
        assert!(matches!(err, TexError::InstallOverwrite { .. }));
    }

    #[test]
    fn install_allows_overwrite_with_force() {
        let tmp = tempfile::tempdir().unwrap();
        let src = seed_package(tmp.path(), "acme/invoice", "1.0.0");
        let root = tmp.path().join("templates");
        install_from_local_path(&src, &root, false).unwrap();
        install_from_local_path(&src, &root, true).unwrap();
    }

    #[test]
    fn install_rejects_bare_identifier() {
        let tmp = tempfile::tempdir().unwrap();
        // "invoice" (no namespace) is not allowed for third-party.
        let src = seed_package(tmp.path(), "invoice", "1.0.0");
        let root = tmp.path().join("templates");
        let err = install_from_local_path(&src, &root, false).unwrap_err();
        assert!(matches!(err, TexError::ManifestInvalidIdentifier { .. }));
    }

    #[test]
    fn install_rejects_entrypoint_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = tmp.path().join("pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("tex-template.toml"),
            "identifier = \"a/b\"\nversion = \"1.0.0\"\nentrypoint = \"../evil.tex\"\n",
        )
        .unwrap();
        let err = install_from_local_path(&pkg, &tmp.path().join("t"), false).unwrap_err();
        assert!(matches!(err, TexError::ManifestEntrypointEscape { .. }));
    }

    #[test]
    fn install_rejects_missing_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = tmp.path().join("empty");
        fs::create_dir_all(&pkg).unwrap();
        let err = install_from_local_path(&pkg, &tmp.path().join("t"), false).unwrap_err();
        assert!(matches!(err, TexError::ManifestNotFound { .. }));
    }

    #[test]
    fn list_installed_returns_empty_when_root_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let out = list_installed(&tmp.path().join("does-not-exist"));
        assert!(out.is_empty());
    }

    #[test]
    fn list_installed_enumerates_multiple_packages() {
        let tmp = tempfile::tempdir().unwrap();
        let src1 = seed_package(tmp.path(), "acme/invoice", "1.0.0");
        let src2 = {
            let p = tmp.path().join("pkg2");
            fs::create_dir_all(&p).unwrap();
            fs::write(
                p.join("tex-template.toml"),
                "identifier = \"my/report\"\nversion = \"0.2.0\"\nentrypoint = \"t.tex\"\n",
            )
            .unwrap();
            fs::write(p.join("t.tex"), "\\documentclass{article}").unwrap();
            p
        };
        let root = tmp.path().join("templates");
        install_from_local_path(&src1, &root, false).unwrap();
        install_from_local_path(&src2, &root, false).unwrap();
        let listed = list_installed(&root);
        assert_eq!(listed.len(), 2);
        let ids: Vec<String> = listed.iter().map(|p| p.identifier.to_string()).collect();
        assert!(ids.contains(&"acme/invoice".to_string()));
        assert!(ids.contains(&"my/report".to_string()));
    }

    #[test]
    fn uninstall_removes_package_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src = seed_package(tmp.path(), "acme/invoice", "1.0.0");
        let root = tmp.path().join("templates");
        install_from_local_path(&src, &root, false).unwrap();

        let id: Identifier = "acme/invoice".parse().unwrap();
        let removed = uninstall(&id, &root).unwrap();
        assert!(!removed.exists());
        // Empty namespace dir also pruned.
        assert!(!root.join("acme").exists());
    }

    #[test]
    fn uninstall_missing_package_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let id: Identifier = "acme/invoice".parse().unwrap();
        let err = uninstall(&id, &tmp.path().join("t")).unwrap_err();
        assert!(matches!(err, TexError::ExplicitTemplateMissing { .. }));
    }

    #[test]
    fn looks_like_git_url_positive_cases() {
        assert!(looks_like_git_url("https://github.com/x/y.git"));
        assert!(looks_like_git_url("git@github.com:x/y.git"));
        assert!(looks_like_git_url("ssh://git@github.com/x/y.git"));
        assert!(looks_like_git_url("/tmp/foo.git"));
    }

    #[test]
    fn looks_like_git_url_negative_cases() {
        assert!(!looks_like_git_url("/tmp/foo"));
        assert!(!looks_like_git_url("./local"));
        assert!(!looks_like_git_url("~/foo"));
    }
}
