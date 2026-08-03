//! Resolution of a JSON document to a concrete template — built-in first, then
//! installed third-party (per FR-018).
//!
//! See `specs/007-auto-pdf-pipeline/contracts/json-document-fields.md` and
//! `specs/007-auto-pdf-pipeline/data-model.md`.

use std::fmt;
use std::str::FromStr;

/// A template identifier.
///
/// Grammar:
/// * built-in (bare): `^[a-z0-9][a-z0-9._-]*$`
/// * third-party (namespaced): `^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9._-]*$`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub namespace: Option<String>,
    pub name: String,
}

impl Identifier {
    pub fn builtin(name: impl Into<String>) -> Result<Self, IdentifierParseError> {
        let name = name.into();
        if !is_valid_name(&name) {
            return Err(IdentifierParseError { value: name });
        }
        Ok(Self {
            namespace: None,
            name,
        })
    }

    pub fn is_builtin(&self) -> bool {
        self.namespace.is_none()
    }

    pub fn is_third_party(&self) -> bool {
        self.namespace.is_some()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.namespace {
            Some(ns) => write!(f, "{}/{}", ns, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl FromStr for Identifier {
    type Err = IdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdentifierParseError {
                value: s.to_string(),
            });
        }
        match s.split_once('/') {
            Some((ns, name)) => {
                if !is_valid_namespace(ns) || !is_valid_name(name) {
                    Err(IdentifierParseError {
                        value: s.to_string(),
                    })
                } else {
                    Ok(Self {
                        namespace: Some(ns.to_string()),
                        name: name.to_string(),
                    })
                }
            }
            None => {
                if !is_valid_name(s) {
                    Err(IdentifierParseError {
                        value: s.to_string(),
                    })
                } else {
                    Ok(Self {
                        namespace: None,
                        name: s.to_string(),
                    })
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierParseError {
    pub value: String,
}

fn is_valid_namespace(s: &str) -> bool {
    // ^[a-z0-9][a-z0-9-]*$
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_valid_name(s: &str) -> bool {
    // ^[a-z0-9][a-z0-9._-]*$
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

use crate::errors::TexError;
use crate::templates::Version;

/// Inputs to `resolve()`: enumerated built-in identifiers and enumerated
/// installed third-party identifiers (installed is empty in US1; populated
/// by Phase 4).
pub struct ResolveInputs<'a> {
    pub builtins: &'a [Identifier],
    pub installed: &'a [InstalledCandidate<'a>],
}

/// One installed template candidate — pairs an identifier with its version
/// so the resolver can return both together.
pub struct InstalledCandidate<'a> {
    pub identifier: &'a Identifier,
    pub version: &'a Version,
}

/// The template resolved from a JSON document.
#[derive(Debug, Clone)]
pub struct ResolvedTemplate {
    pub identifier: Identifier,
    /// `None` for built-in; `Some` for an installed third-party at a
    /// specific installed version.
    pub version: Option<Version>,
}

/// Resolve which template to use for a JSON document per FR-018.
///
/// Order:
/// 1. Explicit `document.template` (if present) — either built-in or installed.
/// 2. `document.type` — matched against built-in first, then installed.
/// 3. On no match or ambiguity, an actionable `TexError` variant.
///
/// See `specs/007-auto-pdf-pipeline/contracts/json-document-fields.md`.
pub fn resolve(
    json: &serde_json::Value,
    inputs: ResolveInputs<'_>,
) -> Result<ResolvedTemplate, TexError> {
    let doc = json.get("document");

    // 1. Explicit override.
    if let Some(raw) = doc.and_then(|d| d.get("template")).and_then(|t| t.as_str()) {
        let requested: Identifier = raw.parse().map_err(|_| TexError::ExplicitTemplateMissing {
            requested: raw.to_string(),
        })?;
        return resolve_explicit(&requested, raw, &inputs);
    }

    // 2. Implicit via document.type.
    let doc_type_raw = doc
        .and_then(|d| d.get("type"))
        .and_then(|t| t.as_str())
        .ok_or(TexError::MissingTypeField)?;

    resolve_by_type(doc_type_raw, &inputs)
}

fn resolve_explicit(
    requested: &Identifier,
    raw: &str,
    inputs: &ResolveInputs<'_>,
) -> Result<ResolvedTemplate, TexError> {
    if requested.is_builtin() {
        if inputs.builtins.iter().any(|b| b == requested) {
            return Ok(ResolvedTemplate {
                identifier: requested.clone(),
                version: None,
            });
        }
        return Err(TexError::ExplicitTemplateMissing {
            requested: raw.to_string(),
        });
    }
    // Third-party — match against installed.
    let hit = inputs.installed.iter().find(|c| c.identifier == requested);
    match hit {
        Some(c) => Ok(ResolvedTemplate {
            identifier: c.identifier.clone(),
            version: Some(c.version.clone()),
        }),
        None => Err(TexError::ExplicitTemplateMissing {
            requested: raw.to_string(),
        }),
    }
}

fn resolve_by_type(
    doc_type: &str,
    inputs: &ResolveInputs<'_>,
) -> Result<ResolvedTemplate, TexError> {
    // Built-in first — bare name match.
    if let Some(builtin) = inputs
        .builtins
        .iter()
        .find(|b| b.is_builtin() && b.name == doc_type)
    {
        return Ok(ResolvedTemplate {
            identifier: builtin.clone(),
            version: None,
        });
    }

    // Installed second — either full `namespace/name` == doc_type
    // or bare `name` (after slash) == doc_type.
    let mut matches: Vec<&InstalledCandidate<'_>> = inputs
        .installed
        .iter()
        .filter(|c| c.identifier.name == doc_type || c.identifier.to_string() == doc_type)
        .collect();

    match matches.len() {
        0 => Err(TexError::NoTemplateMatch {
            requested: doc_type.to_string(),
            candidates: near_matches(doc_type, inputs),
        }),
        1 => {
            let c = matches.pop().unwrap();
            Ok(ResolvedTemplate {
                identifier: c.identifier.clone(),
                version: Some(c.version.clone()),
            })
        }
        _ => Err(TexError::AmbiguousMatch {
            requested: doc_type.to_string(),
            candidates: matches.iter().map(|c| c.identifier.to_string()).collect(),
        }),
    }
}

fn near_matches(_requested: &str, inputs: &ResolveInputs<'_>) -> Vec<String> {
    // v1: return every known identifier as a candidate list. A prefix-similarity
    // filter is a plan-phase enhancement, not a correctness requirement.
    let mut out: Vec<String> = inputs.builtins.iter().map(|b| b.to_string()).collect();
    out.extend(inputs.installed.iter().map(|c| c.identifier.to_string()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bare_name_parses_as_builtin() {
        let id: Identifier = "resume".parse().unwrap();
        assert!(id.is_builtin());
        assert_eq!(id.name, "resume");
        assert_eq!(id.namespace, None);
        assert_eq!(id.to_string(), "resume");
    }

    #[test]
    fn namespaced_parses_as_third_party() {
        let id: Identifier = "acme/invoice".parse().unwrap();
        assert!(id.is_third_party());
        assert_eq!(id.namespace.as_deref(), Some("acme"));
        assert_eq!(id.name, "invoice");
        assert_eq!(id.to_string(), "acme/invoice");
    }

    #[test]
    fn hyphens_and_digits_allowed() {
        assert!("artigo-basico".parse::<Identifier>().is_ok());
        assert!("acme2/v1-invoice".parse::<Identifier>().is_ok());
    }

    #[test]
    fn dots_and_underscores_allowed_in_name_only() {
        assert!("my_report.v2".parse::<Identifier>().is_ok());
        assert!("ns/my_report.v2".parse::<Identifier>().is_ok());
        // Namespace disallows dots/underscores.
        assert!("my.org/name".parse::<Identifier>().is_err());
        assert!("my_org/name".parse::<Identifier>().is_err());
    }

    #[test]
    fn uppercase_rejected() {
        assert!("Resume".parse::<Identifier>().is_err());
        assert!("Acme/invoice".parse::<Identifier>().is_err());
    }

    #[test]
    fn leading_special_char_rejected() {
        assert!("-resume".parse::<Identifier>().is_err());
        assert!(".foo".parse::<Identifier>().is_err());
        assert!("_foo".parse::<Identifier>().is_err());
    }

    #[test]
    fn empty_string_rejected() {
        assert!("".parse::<Identifier>().is_err());
    }

    #[test]
    fn empty_namespace_or_name_rejected() {
        assert!("/name".parse::<Identifier>().is_err());
        assert!("ns/".parse::<Identifier>().is_err());
    }

    #[test]
    fn double_slash_rejected() {
        // "a/b/c" — split_once returns ("a", "b/c") — "b/c" fails name regex.
        assert!("a/b/c".parse::<Identifier>().is_err());
    }

    #[test]
    fn builtin_constructor_validates() {
        assert!(Identifier::builtin("resume").is_ok());
        assert!(Identifier::builtin("Resume").is_err());
    }

    // ---- resolve() tests (US1 — no installed templates yet) ----

    fn builtins(names: &[&str]) -> Vec<Identifier> {
        names
            .iter()
            .map(|n| Identifier::builtin(*n).unwrap())
            .collect()
    }

    fn no_installed<'a>() -> Vec<InstalledCandidate<'a>> {
        Vec::new()
    }

    #[test]
    fn resolve_matches_builtin_by_document_type() {
        let bs = builtins(&["resume", "artigo-basico"]);
        let installed = no_installed();
        let json = json!({ "document": { "type": "resume" } });
        let inputs = ResolveInputs {
            builtins: &bs,
            installed: &installed,
        };
        let out = resolve(&json, inputs).unwrap();
        assert_eq!(out.identifier.name, "resume");
        assert!(out.identifier.is_builtin());
        assert!(out.version.is_none());
    }

    #[test]
    fn resolve_honours_explicit_document_template_for_builtin() {
        let bs = builtins(&["resume", "artigo-basico"]);
        let installed = no_installed();
        // document.type says one thing, document.template overrides.
        let json = json!({ "document": { "type": "resume", "template": "artigo-basico" } });
        let inputs = ResolveInputs {
            builtins: &bs,
            installed: &installed,
        };
        let out = resolve(&json, inputs).unwrap();
        assert_eq!(out.identifier.name, "artigo-basico");
    }

    #[test]
    fn resolve_fails_when_no_document_field() {
        let bs = builtins(&["resume"]);
        let installed = no_installed();
        let json = json!({ "sections": [] });
        let inputs = ResolveInputs {
            builtins: &bs,
            installed: &installed,
        };
        let err = resolve(&json, inputs).unwrap_err();
        assert!(matches!(err, TexError::MissingTypeField));
    }

    #[test]
    fn resolve_fails_when_type_matches_nothing() {
        let bs = builtins(&["resume"]);
        let installed = no_installed();
        let json = json!({ "document": { "type": "invoice" } });
        let inputs = ResolveInputs {
            builtins: &bs,
            installed: &installed,
        };
        let err = resolve(&json, inputs).unwrap_err();
        match err {
            TexError::NoTemplateMatch {
                requested,
                candidates,
            } => {
                assert_eq!(requested, "invoice");
                assert!(candidates.contains(&"resume".to_string()));
            }
            other => panic!("expected NoTemplateMatch, got {other:?}"),
        }
    }

    #[test]
    fn resolve_fails_when_explicit_builtin_missing() {
        let bs = builtins(&["resume"]);
        let installed = no_installed();
        let json = json!({ "document": { "template": "unknown-template" } });
        let inputs = ResolveInputs {
            builtins: &bs,
            installed: &installed,
        };
        let err = resolve(&json, inputs).unwrap_err();
        match err {
            TexError::ExplicitTemplateMissing { requested } => {
                assert_eq!(requested, "unknown-template");
            }
            other => panic!("expected ExplicitTemplateMissing, got {other:?}"),
        }
    }

    #[test]
    fn resolve_fails_when_explicit_third_party_absent_from_installed() {
        // US1: no installed templates yet. Any namespaced explicit ID misses.
        let bs = builtins(&["resume"]);
        let installed = no_installed();
        let json = json!({ "document": { "template": "acme/invoice" } });
        let inputs = ResolveInputs {
            builtins: &bs,
            installed: &installed,
        };
        let err = resolve(&json, inputs).unwrap_err();
        assert!(matches!(err, TexError::ExplicitTemplateMissing { .. }));
    }
}
