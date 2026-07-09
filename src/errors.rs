use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TexError {
    #[error("Nenhum config encontrado. Rode 'tex-cli init' primeiro.")]
    ConfigMissing,

    #[error(
        "Config em {path} está inválido: {detail}. Rode 'tex-cli init' novamente para recriar."
    )]
    ConfigCorrupted { path: PathBuf, detail: String },

    #[error(
        "Chave desconhecida: '{key}'. Chaves aceitas:\n{}",
        format_accepted(accepted)
    )]
    UnknownKey {
        key: String,
        accepted: Vec<&'static str>,
    },

    #[error("Valor inválido para chave booleana '{key}': '{value}'. Aceito: true, false.")]
    InvalidBoolValue { key: String, value: String },

    #[error("Permissão negada ao acessar {path}.")]
    PermissionDenied { path: PathBuf },

    #[error("Operação cancelada pelo usuário.")]
    UserAborted,

    #[error("Não foi possível resolver o diretório home do usuário.")]
    HomeDirUnavailable,

    #[error("Binário do compilador '{engine}' não foi encontrado no PATH.")]
    EngineBinaryMissing { engine: String },

    #[error("Template '{name}' não existe em {}.", templates_dir.display())]
    TemplateNotFound {
        name: String,
        templates_dir: PathBuf,
    },

    #[error("Arquivo '{}' não é texto UTF-8 válido: {detail}.", source_path.display())]
    InvalidUtf8 {
        source_path: PathBuf,
        detail: String,
    },

    #[error(
        "Diretório de templates '{}' não existe. \
         Rode 'tex-cli config set paths.templates_dir <path>' \
         ou crie o diretório.",
        templates_dir.display()
    )]
    TemplatesDirMissing { templates_dir: PathBuf },

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
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
            TexError::Io(_) => 1,
            TexError::HomeDirUnavailable => 1,
            TexError::EngineBinaryMissing { .. } => 1,
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
    fn display_messages_are_in_portuguese() {
        assert!(format!("{}", TexError::ConfigMissing).contains("config"));
        assert!(format!("{}", TexError::UserAborted).contains("cancelada"));
    }
}
