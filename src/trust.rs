//! Per-(template, version) approval records for installed third-party templates.
//!
//! See spec 007 FR-016 and `specs/007-auto-pdf-pipeline/research.md` R3.

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::atomic;
use crate::discovery::Identifier;
use crate::errors::TexError;
use crate::templates::Version;

/// Serialized form of `~/.local/share/tex/trust.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFile {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub trust: Vec<TrustRecord>,
}

fn default_schema_version() -> u32 {
    1
}

/// One approval — `(identifier, version)` pair plus a Unix-seconds timestamp
/// for future audit tooling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRecord {
    /// Canonical `namespace/name`.
    pub identifier: String,
    /// Canonical semver string.
    pub version: String,
    /// Seconds since Unix epoch. (v1 keeps this as u64 to avoid pulling in a
    /// date/time crate outside the fixed stack; can migrate to RFC 3339 in
    /// schema_version 2 without breaking readers.)
    pub approved_at: u64,
}

impl Default for TrustFile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            trust: Vec::new(),
        }
    }
}

impl TrustFile {
    /// Read the trust file from `path`. Missing file → default (empty).
    pub fn load(path: &Path) -> Result<Self, TexError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).map_err(TexError::Io)?;
        toml::from_str::<TrustFile>(&raw).map_err(|e| TexError::TrustFileCorrupted {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })
    }

    /// Persist atomically (spec-005 atomic helper — no half-writes on crash).
    pub fn save(&self, path: &Path) -> Result<(), TexError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(TexError::Io)?;
        }
        let body = toml::to_string_pretty(self).map_err(|e| TexError::TrustFileCorrupted {
            path: path.to_path_buf(),
            detail: format!("serialize: {e}"),
        })?;
        atomic::write_atomic(path, body.as_bytes(), 0o600)
    }

    /// True iff a record exists for the exact `(identifier, version)` pair.
    pub fn is_trusted(&self, id: &Identifier, ver: &Version) -> bool {
        let id_s = id.to_string();
        let ver_s = ver.to_string();
        self.trust
            .iter()
            .any(|r| r.identifier == id_s && r.version == ver_s)
    }

    /// Append an approval (idempotent — no duplicate `(id, version)` entries).
    pub fn grant(&mut self, id: &Identifier, ver: &Version) {
        let id_s = id.to_string();
        let ver_s = ver.to_string();
        if self.is_trusted(id, ver) {
            return;
        }
        let approved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.trust.push(TrustRecord {
            identifier: id_s,
            version: ver_s,
            approved_at,
        });
    }

    /// Drop every record for a given identifier (any version).
    /// Used by `tex-cli template remove`.
    pub fn revoke_all(&mut self, id: &Identifier) {
        let id_s = id.to_string();
        self.trust.retain(|r| r.identifier != id_s);
    }

    /// Drop a single `(identifier, version)` record.
    pub fn revoke_version(&mut self, id: &Identifier, ver: &Version) {
        let id_s = id.to_string();
        let ver_s = ver.to_string();
        self.trust
            .retain(|r| !(r.identifier == id_s && r.version == ver_s));
    }
}

/// FR-016: prompt the user for approval of an installed third-party template.
///
/// * If the pair is already trusted → returns `Ok(())` without prompting.
/// * If the invocation is not attached to a TTY → returns
///   `TexError::TrustDenied` (never auto-approves silently — the caller
///   should tell the user to run `tex-cli template trust` first).
/// * If the user answers "no" at the `inquire::Confirm` → returns
///   `TexError::TrustDenied`.
/// * On approval, the record is granted and persisted to `trust_path`.
pub fn ensure_trusted(
    trust_path: &Path,
    id: &Identifier,
    ver: &Version,
) -> Result<(), TexError> {
    let mut file = TrustFile::load(trust_path)?;
    if file.is_trusted(id, ver) {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        return Err(TexError::TrustDenied {
            identifier: id.to_string(),
            version: ver.to_string(),
        });
    }
    let msg = format!(
        "Trust `{id}@{ver}`? This template will run inside the spec-007 sandbox \
         (no shell escape, no network, writes confined to the output directory)."
    );
    let approved = inquire::Confirm::new(&msg)
        .with_default(false)
        .prompt()
        .map_err(|e| match e {
            inquire::InquireError::OperationCanceled
            | inquire::InquireError::OperationInterrupted => TexError::UserAborted,
            other => TexError::Io(std::io::Error::other(other.to_string())),
        })?;
    if !approved {
        return Err(TexError::TrustDenied {
            identifier: id.to_string(),
            version: ver.to_string(),
        });
    }
    file.grant(id, ver);
    file.save(trust_path)?;
    Ok(())
}

/// Read-only convenience for `tex-cli template list --json` (spec 007 T030).
pub fn is_trusted(trust_path: &Path, id: &Identifier, ver: &Version) -> bool {
    match TrustFile::load(trust_path) {
        Ok(f) => f.is_trusted(id, ver),
        Err(_) => false,
    }
}

/// Testing helper — build a canonical trust-file path under a `HOME` dir.
pub fn trust_file_under(home: &Path) -> PathBuf {
    home.join(".local").join("share").join("tex").join("trust.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> Identifier {
        s.parse().unwrap()
    }
    fn v(s: &str) -> Version {
        s.parse().unwrap()
    }

    #[test]
    fn empty_file_defaults_to_schema_v1() {
        let f = TrustFile::default();
        assert_eq!(f.schema_version, 1);
        assert!(f.trust.is_empty());
    }

    #[test]
    fn is_trusted_matches_only_exact_identifier_and_version() {
        let mut f = TrustFile::default();
        f.grant(&id("acme/invoice"), &v("1.0.0"));
        assert!(f.is_trusted(&id("acme/invoice"), &v("1.0.0")));
        assert!(!f.is_trusted(&id("acme/invoice"), &v("1.0.1")));
        assert!(!f.is_trusted(&id("acme/other"), &v("1.0.0")));
    }

    #[test]
    fn grant_is_idempotent() {
        let mut f = TrustFile::default();
        f.grant(&id("acme/invoice"), &v("1.0.0"));
        f.grant(&id("acme/invoice"), &v("1.0.0"));
        assert_eq!(f.trust.len(), 1);
    }

    #[test]
    fn revoke_all_removes_every_version_of_id() {
        let mut f = TrustFile::default();
        f.grant(&id("acme/invoice"), &v("1.0.0"));
        f.grant(&id("acme/invoice"), &v("1.1.0"));
        f.grant(&id("other/x"), &v("0.1.0"));
        f.revoke_all(&id("acme/invoice"));
        assert!(!f.is_trusted(&id("acme/invoice"), &v("1.0.0")));
        assert!(!f.is_trusted(&id("acme/invoice"), &v("1.1.0")));
        assert!(f.is_trusted(&id("other/x"), &v("0.1.0")));
    }

    #[test]
    fn save_then_load_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("trust.toml");
        let mut f = TrustFile::default();
        f.grant(&id("acme/invoice"), &v("1.0.0"));
        f.save(&path).unwrap();
        let loaded = TrustFile::load(&path).unwrap();
        assert!(loaded.is_trusted(&id("acme/invoice"), &v("1.0.0")));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does-not-exist.toml");
        let f = TrustFile::load(&path).unwrap();
        assert!(f.trust.is_empty());
    }

    #[test]
    fn load_corrupted_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.toml");
        std::fs::write(&path, "this is [not = toml").unwrap();
        let err = TrustFile::load(&path).unwrap_err();
        assert!(matches!(err, TexError::TrustFileCorrupted { .. }));
    }
}
