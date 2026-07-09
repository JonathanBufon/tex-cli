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
