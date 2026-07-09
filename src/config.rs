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
