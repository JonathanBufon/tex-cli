use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::TexError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub paths: PathsConfig,
    pub compiler: CompilerConfig,
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathsConfig {
    pub templates_dir: PathBuf,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerConfig {
    pub engine: String,
    pub keep_tex: bool,
    pub keep_logs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorConfig {
    pub ask_output_path_every_time: bool,
}

pub const DEFAULT_ENGINE: &str = "tectonic";
pub const DEFAULT_KEEP_TEX: bool = true;
pub const DEFAULT_KEEP_LOGS: bool = true;
pub const DEFAULT_ASK_OUTPUT_PATH_EVERY_TIME: bool = false;

impl Config {
    pub fn new_from_prompts(
        templates_dir: PathBuf,
        output_dir: PathBuf,
        engine: String,
    ) -> Self {
        Self {
            paths: PathsConfig {
                templates_dir,
                output_dir,
            },
            compiler: CompilerConfig {
                engine,
                keep_tex: DEFAULT_KEEP_TEX,
                keep_logs: DEFAULT_KEEP_LOGS,
            },
            behavior: BehaviorConfig {
                ask_output_path_every_time: DEFAULT_ASK_OUTPUT_PATH_EVERY_TIME,
            },
        }
    }

    pub fn save_atomic(&self, target: &Path) -> Result<(), TexError> {
        let parent = target.parent().ok_or_else(|| TexError::Io(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "target has no parent dir"),
        ))?;
        fs::create_dir_all(parent)?;

        let serialized = toml::to_string_pretty(self).map_err(|e| {
            TexError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
        })?;

        let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
                path: parent.to_path_buf(),
            },
            _ => TexError::Io(e),
        })?;
        tmp.as_file_mut().write_all(serialized.as_bytes())?;
        tmp.as_file_mut().sync_all()?;

        let mut perms = tmp.as_file().metadata()?.permissions();
        perms.set_mode(0o600);
        tmp.as_file().set_permissions(perms)?;

        tmp.persist(target).map_err(|e| match e.error.kind() {
            std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
                path: target.to_path_buf(),
            },
            _ => TexError::Io(e.error),
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> Config {
        Config {
            paths: PathsConfig {
                templates_dir: PathBuf::from("/home/u/tex/templates"),
                output_dir: PathBuf::from("/home/u/tex/output"),
            },
            compiler: CompilerConfig {
                engine: "tectonic".into(),
                keep_tex: true,
                keep_logs: true,
            },
            behavior: BehaviorConfig {
                ask_output_path_every_time: false,
            },
        }
    }

    #[test]
    fn roundtrip_config_toml() {
        let cfg = sample_config();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn deny_unknown_fields_rejects_extra_field() {
        let bad = r#"
[paths]
templates_dir = "/x"
output_dir = "/y"
extra = "nope"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#;
        let res = toml::from_str::<Config>(bad);
        assert!(res.is_err(), "extra field should be rejected");
    }

    #[test]
    fn save_atomic_creates_file_on_missing_path() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("nested").join("config.toml");
        let cfg = sample_config();

        cfg.save_atomic(&target).unwrap();

        assert!(target.exists());
        let back: Config = toml::from_str(&std::fs::read_to_string(&target).unwrap()).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn save_atomic_overwrites_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("config.toml");
        std::fs::write(&target, "old content").unwrap();

        let cfg = sample_config();
        cfg.save_atomic(&target).unwrap();

        let written = std::fs::read_to_string(&target).unwrap();
        assert!(written.contains("tectonic"));
        assert!(!written.contains("old content"));
    }

    #[test]
    fn save_atomic_sets_mode_0600() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("config.toml");
        let cfg = sample_config();

        cfg.save_atomic(&target).unwrap();

        let mode = std::fs::metadata(&target).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "expected 0o600, got {:o}", mode & 0o777);
    }

    #[test]
    fn deny_unknown_top_level_section_rejected() {
        let bad = r#"
[paths]
templates_dir = "/x"
output_dir = "/y"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false

[foo]
bar = "baz"
"#;
        let res = toml::from_str::<Config>(bad);
        assert!(res.is_err(), "unknown top-level section should be rejected");
    }
}
