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

// resolve() lands in Phase 3 (T010) and Phase 4 (T025).

#[cfg(test)]
mod tests {
    use super::*;

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
}
