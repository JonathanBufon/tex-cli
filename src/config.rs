use std::fmt;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

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

pub fn render_humano(c: &Config) -> String {
    let mut out = String::new();

    out.push_str("[paths]\n");
    out.push_str(&format!(
        "  templates_dir              = {}\n",
        c.paths.templates_dir.display()
    ));
    out.push_str(&format!(
        "  output_dir                 = {}\n",
        c.paths.output_dir.display()
    ));

    out.push_str("\n[compiler]\n");
    out.push_str(&format!(
        "  engine                     = {}\n",
        c.compiler.engine
    ));
    out.push_str(&format!(
        "  keep_tex                   = {}\n",
        c.compiler.keep_tex
    ));
    out.push_str(&format!(
        "  keep_logs                  = {}\n",
        c.compiler.keep_logs
    ));

    out.push_str("\n[behavior]\n");
    out.push_str(&format!(
        "  ask_output_path_every_time = {}\n",
        c.behavior.ask_output_path_every_time
    ));

    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    PathsTemplatesDir,
    PathsOutputDir,
    CompilerEngine,
    CompilerKeepTex,
    CompilerKeepLogs,
    BehaviorAskOutputPathEveryTime,
}

impl ConfigKey {
    pub const ALL: &'static [ConfigKey] = &[
        ConfigKey::PathsTemplatesDir,
        ConfigKey::PathsOutputDir,
        ConfigKey::CompilerEngine,
        ConfigKey::CompilerKeepTex,
        ConfigKey::CompilerKeepLogs,
        ConfigKey::BehaviorAskOutputPathEveryTime,
    ];

    pub fn canonical(&self) -> &'static str {
        match self {
            ConfigKey::PathsTemplatesDir => "paths.templates_dir",
            ConfigKey::PathsOutputDir => "paths.output_dir",
            ConfigKey::CompilerEngine => "compiler.engine",
            ConfigKey::CompilerKeepTex => "compiler.keep_tex",
            ConfigKey::CompilerKeepLogs => "compiler.keep_logs",
            ConfigKey::BehaviorAskOutputPathEveryTime => "behavior.ask_output_path_every_time",
        }
    }
}

impl fmt::Display for ConfigKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical())
    }
}

impl FromStr for ConfigKey {
    type Err = TexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for key in ConfigKey::ALL {
            if key.canonical() == s {
                return Ok(*key);
            }
        }
        Err(TexError::UnknownKey {
            key: s.to_string(),
            accepted: ConfigKey::ALL.iter().map(|k| k.canonical()).collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedChange {
    pub key: ConfigKey,
    pub normalized_value: String,
    pub warnings: Vec<String>,
}

fn parse_strict_bool(key: &ConfigKey, raw: &str) -> Result<bool, TexError> {
    match raw {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(TexError::InvalidBoolValue {
            key: key.to_string(),
            value: raw.to_string(),
        }),
    }
}

pub const DEFAULT_ENGINE: &str = "tectonic";
pub const DEFAULT_KEEP_TEX: bool = true;
pub const DEFAULT_KEEP_LOGS: bool = true;
pub const DEFAULT_ASK_OUTPUT_PATH_EVERY_TIME: bool = false;

impl Config {
    pub fn load(path: &Path) -> Result<Self, TexError> {
        let contents = fs::read_to_string(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => TexError::ConfigMissing,
            std::io::ErrorKind::PermissionDenied => TexError::PermissionDenied {
                path: path.to_path_buf(),
            },
            _ => TexError::Io(e),
        })?;

        toml::from_str::<Config>(&contents).map_err(|e| TexError::ConfigCorrupted {
            path: path.to_path_buf(),
            detail: e.message().to_string(),
        })
    }

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

    pub fn apply(
        &mut self,
        key: ConfigKey,
        raw_value: &str,
    ) -> Result<AppliedChange, TexError> {
        match key {
            ConfigKey::PathsTemplatesDir => {
                let expanded = crate::paths::expand_user_path(raw_value)?;
                let mut warnings = Vec::new();
                if !expanded.exists() {
                    warnings.push(format!(
                        "Aviso: '{}' não existe no momento.",
                        expanded.display()
                    ));
                }
                self.paths.templates_dir = expanded.clone();
                Ok(AppliedChange {
                    key,
                    normalized_value: expanded.display().to_string(),
                    warnings,
                })
            }
            ConfigKey::PathsOutputDir => {
                let expanded = crate::paths::expand_user_path(raw_value)?;
                let mut warnings = Vec::new();
                if !expanded.exists() {
                    warnings.push(format!(
                        "Aviso: '{}' não existe no momento.",
                        expanded.display()
                    ));
                }
                self.paths.output_dir = expanded.clone();
                Ok(AppliedChange {
                    key,
                    normalized_value: expanded.display().to_string(),
                    warnings,
                })
            }
            ConfigKey::CompilerEngine => {
                if raw_value.trim().is_empty() {
                    return Err(TexError::InvalidBoolValue {
                        key: key.to_string(),
                        value: raw_value.to_string(),
                    });
                }
                let mut warnings = Vec::new();
                if raw_value != "tectonic" {
                    warnings.push(format!(
                        "Aviso: engine '{raw_value}' ainda não é executada pelo Tex nesta versão. A preferência foi salva."
                    ));
                }
                if which::which(raw_value).is_err() {
                    warnings.push(format!("Aviso: '{raw_value}' não está no PATH."));
                }
                self.compiler.engine = raw_value.to_string();
                Ok(AppliedChange {
                    key,
                    normalized_value: raw_value.to_string(),
                    warnings,
                })
            }
            ConfigKey::CompilerKeepTex => {
                let parsed = parse_strict_bool(&key, raw_value)?;
                self.compiler.keep_tex = parsed;
                Ok(AppliedChange {
                    key,
                    normalized_value: parsed.to_string(),
                    warnings: Vec::new(),
                })
            }
            ConfigKey::CompilerKeepLogs => {
                let parsed = parse_strict_bool(&key, raw_value)?;
                self.compiler.keep_logs = parsed;
                Ok(AppliedChange {
                    key,
                    normalized_value: parsed.to_string(),
                    warnings: Vec::new(),
                })
            }
            ConfigKey::BehaviorAskOutputPathEveryTime => {
                let parsed = parse_strict_bool(&key, raw_value)?;
                self.behavior.ask_output_path_every_time = parsed;
                Ok(AppliedChange {
                    key,
                    normalized_value: parsed.to_string(),
                    warnings: Vec::new(),
                })
            }
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
    fn config_key_from_str_accepts_all_canonical() {
        for key in ConfigKey::ALL {
            let parsed: ConfigKey = key.canonical().parse().unwrap();
            assert_eq!(&parsed, key);
        }
    }

    #[test]
    fn config_key_from_str_rejects_unknown() {
        let err = "foo.bar".parse::<ConfigKey>().unwrap_err();
        match err {
            TexError::UnknownKey { key, accepted } => {
                assert_eq!(key, "foo.bar");
                assert_eq!(accepted.len(), 6);
            }
            other => panic!("expected UnknownKey, got {other:?}"),
        }
    }

    #[test]
    fn config_key_from_str_rejects_aliases_and_case_variants() {
        assert!("templates_dir".parse::<ConfigKey>().is_err());
        assert!("Paths.Templates_dir".parse::<ConfigKey>().is_err());
        assert!(" paths.templates_dir ".parse::<ConfigKey>().is_err());
        assert!("PATHS.TEMPLATES_DIR".parse::<ConfigKey>().is_err());
    }

    #[test]
    fn config_key_display_returns_canonical() {
        assert_eq!(ConfigKey::CompilerEngine.to_string(), "compiler.engine");
        assert_eq!(ConfigKey::PathsOutputDir.to_string(), "paths.output_dir");
    }

    #[test]
    fn config_key_all_has_six_entries() {
        assert_eq!(ConfigKey::ALL.len(), 6);
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
