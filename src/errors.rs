use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TexError {
    #[error("No config found. Run 'tex-cli init' first.")]
    ConfigMissing,

    #[error("Config at {path} is invalid: {detail}. Run 'tex-cli init' again to recreate.")]
    ConfigCorrupted { path: PathBuf, detail: String },

    #[error("Unknown key: '{key}'. Accepted keys:\n{}", format_accepted(accepted))]
    UnknownKey {
        key: String,
        accepted: Vec<&'static str>,
    },

    #[error("Invalid value for boolean key '{key}': '{value}'. Accepted: true, false.")]
    InvalidBoolValue { key: String, value: String },

    #[error("Permission denied while accessing {path}.")]
    PermissionDenied { path: PathBuf },

    #[error("Operation cancelled by user.")]
    UserAborted,

    #[error("Could not resolve the user's home directory.")]
    HomeDirUnavailable,

    #[error("Template '{name}' not found in {}.", templates_dir.display())]
    TemplateNotFound {
        name: String,
        templates_dir: PathBuf,
    },

    #[error("File '{}' is not valid UTF-8 text: {detail}.", source_path.display())]
    InvalidUtf8 {
        source_path: PathBuf,
        detail: String,
    },

    #[error(
        "Templates directory '{}' not found. \
         Run 'tex-cli config set paths.templates_dir <path>' \
         or create the directory.",
        templates_dir.display()
    )]
    TemplatesDirMissing { templates_dir: PathBuf },

    #[error("Failed to render template '{template_name}': {detail}")]
    TeraRenderError {
        template_name: String,
        detail: String,
    },

    #[error("Invalid JSON in {source_name}: {detail}")]
    InvalidJson { source_name: String, detail: String },

    #[error(
        "Failed to compile '{}': engine '{engine}' returned an error.\n\
         Last lines of the log:\n{log_tail}\n",
        tex_path.display()
    )]
    CompileFailed {
        engine: String,
        tex_path: PathBuf,
        log_tail: String,
    },

    #[error(
        "Engine '{engine}' is not installed on PATH. \
         Install it or use --engine <other>."
    )]
    EngineNotInstalled { engine: String },

    #[error(
        "Engine '{engine}' is not supported. Accepted:\n{}",
        format_accepted(accepted)
    )]
    EngineNotSupported {
        engine: String,
        accepted: Vec<&'static str>,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // ---- Install (spec 007, range 50-59) ----
    #[error(
        "'git' binary not found on PATH. \
         Install git (Debian: apt install git; macOS: brew install git) \
         and re-run."
    )]
    GitBinaryMissing,

    #[error("git clone of '{url}' failed: {status}")]
    GitCloneFailed { url: String, status: String },

    #[error(
        "installed template layout mismatch: manifest declares '{manifest_id}' \
         but the destination path is '{}'. Refusing to install.",
        dest_path.display()
    )]
    InstallPathMismatch {
        manifest_id: String,
        dest_path: PathBuf,
    },

    #[error(
        "'{identifier}@{version}' is already installed at '{}'. \
         Re-run with --force to overwrite.",
        path.display()
    )]
    InstallOverwrite {
        identifier: String,
        version: String,
        path: PathBuf,
    },

    // ---- Manifest (spec 007, range 60-69) ----
    #[error("no template manifest found at '{}'.", path.display())]
    ManifestNotFound { path: PathBuf },

    #[error("template manifest at '{}' is not valid TOML: {detail}", path.display())]
    ManifestParse { path: PathBuf, detail: String },

    #[error("template manifest at '{}' is missing required field '{field}'.", path.display())]
    ManifestMissingField { path: PathBuf, field: &'static str },

    #[error(
        "invalid template identifier: '{value}'. \
         Third-party templates require the form 'namespace/name' \
         where each segment starts with a-z0-9."
    )]
    ManifestInvalidIdentifier { value: String },

    #[error(
        "invalid template version: '{value}'. \
         Expected semver 'MAJOR.MINOR.PATCH' with optional '-<pre>' and '+<build>'."
    )]
    ManifestInvalidVersion { value: String },

    #[error(
        "template entrypoint '{}' resolves outside the package root — refusing to install.",
        entrypoint.display()
    )]
    ManifestEntrypointEscape { entrypoint: PathBuf },

    #[error("template entrypoint '{}' does not exist in the package.", entrypoint.display())]
    ManifestEntrypointMissing { entrypoint: PathBuf },

    #[error("template entrypoint '{}' must end in '.tex'.", entrypoint.display())]
    ManifestEntrypointNotTex { entrypoint: PathBuf },

    // ---- Trust (spec 007, range 70-79) ----
    #[error("trust denied by user for '{identifier}@{version}'; no compile performed.")]
    TrustDenied { identifier: String, version: String },

    #[error(
        "trust file at '{}' is corrupted: {detail}. \
         Delete it to reset (all installed templates will need re-approval).",
        path.display()
    )]
    TrustFileCorrupted { path: PathBuf, detail: String },

    // ---- Resolve (spec 007, range 80-89) ----
    #[error(
        "JSON has no 'document.type' or 'document.template' field; cannot resolve a template."
    )]
    MissingTypeField,

    #[error(
        "explicit 'document.template = \"{requested}\"' does not match any installed \
         or built-in template."
    )]
    ExplicitTemplateMissing { requested: String },

    #[error(
        "'document.type = \"{requested}\"' matched no template.\n{}",
        format_candidates("Near-matches", candidates)
    )]
    NoTemplateMatch {
        requested: String,
        candidates: Vec<String>,
    },

    #[error(
        "'document.type = \"{requested}\"' matched multiple installed templates:\n{}\n\
         Set 'document.template' in the JSON to disambiguate.",
        format_candidates("Candidates", candidates)
    )]
    AmbiguousMatch {
        requested: String,
        candidates: Vec<String>,
    },

    // ---- Sandbox (spec 007, range 90-99) ----
    #[error(
        "template '{identifier}' attempted to write outside the output directory ('{}'). \
         Compile aborted; no PDF produced.",
        path.display()
    )]
    SandboxFilesystemEscape { path: PathBuf, identifier: String },

    #[error(
        "Tectonic bundle cache is empty — third-party template compiles require a warm cache. \
         Run 'tex-cli init' or compile a built-in template once to populate the cache."
    )]
    SandboxBundleMissing,

    #[error(
        "template '{identifier}' attempted a restricted operation: \
         shell escape (\\write18) — disabled for third-party templates. \
         Compile aborted; no PDF produced."
    )]
    SandboxShellEscapeAttempted { identifier: String },
}

impl TexError {
    pub fn exit_code(&self) -> i32 {
        match self {
            TexError::ConfigMissing => 10,
            TexError::ConfigCorrupted { .. } => 11,
            TexError::UnknownKey { .. } => 12,
            TexError::InvalidBoolValue { .. } => 13,
            TexError::PermissionDenied { .. } => 14,
            TexError::UserAborted => 15,
            TexError::TemplateNotFound { .. } => 20,
            TexError::InvalidUtf8 { .. } => 21,
            TexError::TemplatesDirMissing { .. } => 22,
            TexError::TeraRenderError { .. } => 30,
            TexError::InvalidJson { .. } => 31,
            TexError::CompileFailed { .. } => 40,
            TexError::EngineNotInstalled { .. } => 41,
            TexError::EngineNotSupported { .. } => 42,
            TexError::Io(_) => 1,
            TexError::HomeDirUnavailable => 1,

            // Install (50-59)
            TexError::GitBinaryMissing => 50,
            TexError::GitCloneFailed { .. } => 51,
            TexError::InstallPathMismatch { .. } => 52,
            TexError::InstallOverwrite { .. } => 53,

            // Manifest (60-69)
            TexError::ManifestNotFound { .. } => 60,
            TexError::ManifestParse { .. } => 61,
            TexError::ManifestMissingField { .. } => 62,
            TexError::ManifestInvalidIdentifier { .. } => 63,
            TexError::ManifestInvalidVersion { .. } => 64,
            TexError::ManifestEntrypointEscape { .. } => 65,
            TexError::ManifestEntrypointMissing { .. } => 66,
            TexError::ManifestEntrypointNotTex { .. } => 67,

            // Trust (70-79)
            TexError::TrustDenied { .. } => 70,
            TexError::TrustFileCorrupted { .. } => 71,

            // Resolve (80-89)
            TexError::MissingTypeField => 80,
            TexError::ExplicitTemplateMissing { .. } => 81,
            TexError::NoTemplateMatch { .. } => 82,
            TexError::AmbiguousMatch { .. } => 83,

            // Sandbox (90-99)
            TexError::SandboxFilesystemEscape { .. } => 90,
            TexError::SandboxBundleMissing => 91,
            TexError::SandboxShellEscapeAttempted { .. } => 92,
        }
    }
}

fn format_accepted(accepted: &[&'static str]) -> String {
    accepted
        .iter()
        .map(|k| format!("  - {k}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_candidates(label: &str, candidates: &[String]) -> String {
    if candidates.is_empty() {
        format!("{label}: (none)")
    } else {
        let list = candidates
            .iter()
            .map(|c| format!("  - {c}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{label}:\n{list}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_contract() {
        assert_eq!(TexError::ConfigMissing.exit_code(), 10);
        assert_eq!(
            TexError::ConfigCorrupted {
                path: PathBuf::from("/x"),
                detail: "bad".into(),
            }
            .exit_code(),
            11
        );
        assert_eq!(
            TexError::UnknownKey {
                key: "foo".into(),
                accepted: vec!["a", "b"],
            }
            .exit_code(),
            12
        );
        assert_eq!(
            TexError::InvalidBoolValue {
                key: "compiler.keep_tex".into(),
                value: "maybe".into(),
            }
            .exit_code(),
            13
        );
        assert_eq!(
            TexError::PermissionDenied {
                path: PathBuf::from("/x"),
            }
            .exit_code(),
            14
        );
        assert_eq!(TexError::UserAborted.exit_code(), 15);
        assert_eq!(
            TexError::TemplateNotFound {
                name: "artigo".into(),
                templates_dir: PathBuf::from("/tpl"),
            }
            .exit_code(),
            20
        );
        assert_eq!(
            TexError::InvalidUtf8 {
                source_path: PathBuf::from("/x.tex"),
                detail: "bad byte".into(),
            }
            .exit_code(),
            21
        );
        assert_eq!(
            TexError::TemplatesDirMissing {
                templates_dir: PathBuf::from("/tpl"),
            }
            .exit_code(),
            22
        );
        assert_eq!(
            TexError::TeraRenderError {
                template_name: "artigo".into(),
                detail: "Variable `nome` not found".into(),
            }
            .exit_code(),
            30
        );
        assert_eq!(
            TexError::InvalidJson {
                source_name: "stdin".into(),
                detail: "expected value at line 1".into(),
            }
            .exit_code(),
            31
        );
        assert_eq!(
            TexError::CompileFailed {
                engine: "tectonic".into(),
                tex_path: PathBuf::from("/tmp/x.tex"),
                log_tail: "! Undefined control sequence.".into(),
            }
            .exit_code(),
            40
        );
        assert_eq!(
            TexError::EngineNotInstalled {
                engine: "tectonic".into(),
            }
            .exit_code(),
            41
        );
        assert_eq!(
            TexError::EngineNotSupported {
                engine: "foo".into(),
                accepted: vec!["tectonic", "latexmk"],
            }
            .exit_code(),
            42
        );
    }

    #[test]
    fn compile_failed_display_includes_engine_and_log_tail() {
        let e = TexError::CompileFailed {
            engine: "tectonic".into(),
            tex_path: PathBuf::from("/tmp/artigo.tex"),
            log_tail: "! Undefined control sequence.\nl.3 \\undefinedcommand".into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("/tmp/artigo.tex"));
        assert!(msg.contains("tectonic"));
        assert!(msg.contains("Undefined control sequence"));
        assert!(msg.contains("Last lines of the log"));
    }

    #[test]
    fn engine_not_installed_display_orients_user() {
        let e = TexError::EngineNotInstalled {
            engine: "latexmk".into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("latexmk"));
        assert!(msg.contains("PATH"));
        assert!(msg.contains("--engine"));
    }

    #[test]
    fn engine_not_supported_display_lists_accepted() {
        let e = TexError::EngineNotSupported {
            engine: "foo".into(),
            accepted: vec!["tectonic", "latexmk", "pdflatex", "xelatex", "lualatex"],
        };
        let msg = format!("{e}");
        assert!(msg.contains("foo"));
        assert!(msg.contains("tectonic"));
        assert!(msg.contains("latexmk"));
        assert!(msg.contains("lualatex"));
    }

    #[test]
    fn tera_render_error_display_mentions_template() {
        let e = TexError::TeraRenderError {
            template_name: "artigo".into(),
            detail: "Variable `nome` not found in context".into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("artigo"));
        assert!(msg.contains("render"));
        assert!(msg.contains("Variable"));
    }

    #[test]
    fn invalid_json_display_mentions_source() {
        let e = TexError::InvalidJson {
            source_name: "/tmp/bad.json".into(),
            detail: "expected value at line 3 column 5".into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("/tmp/bad.json"));
        assert!(msg.contains("Invalid JSON"));
        assert!(msg.contains("line 3"));
    }

    #[test]
    fn template_not_found_display_mentions_dir() {
        let e = TexError::TemplateNotFound {
            name: "artigo".into(),
            templates_dir: PathBuf::from("/home/u/tex/templates"),
        };
        let msg = format!("{e}");
        assert!(msg.contains("artigo"));
        assert!(msg.contains("/home/u/tex/templates"));
    }

    #[test]
    fn invalid_utf8_display_mentions_source() {
        let e = TexError::InvalidUtf8 {
            source_path: PathBuf::from("/tmp/binario.tex"),
            detail: "invalid utf-8 sequence".into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("/tmp/binario.tex"));
        assert!(msg.contains("UTF-8"));
    }

    #[test]
    fn templates_dir_missing_display_orients_user() {
        let e = TexError::TemplatesDirMissing {
            templates_dir: PathBuf::from("/nope"),
        };
        let msg = format!("{e}");
        assert!(msg.contains("/nope"));
        assert!(msg.contains("config set"));
    }

    #[test]
    fn unknown_key_display_lists_accepted() {
        let e = TexError::UnknownKey {
            key: "foo".into(),
            accepted: vec!["paths.templates_dir", "compiler.engine"],
        };
        let msg = format!("{e}");
        assert!(msg.contains("paths.templates_dir"));
        assert!(msg.contains("compiler.engine"));
        assert!(msg.contains("foo"));
    }

    #[test]
    fn display_messages_are_in_english() {
        assert!(format!("{}", TexError::ConfigMissing).contains("config"));
        assert!(format!("{}", TexError::UserAborted).contains("cancelled"));
    }
}
