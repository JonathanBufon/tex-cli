use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
