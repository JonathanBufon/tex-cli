use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_valid_config(home: &TempDir) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let body = r#"[paths]
templates_dir = "/tmp/templates"
output_dir = "/tmp/output"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#;
    std::fs::write(&cfg, body).unwrap();
    let mut perms = std::fs::metadata(&cfg).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&cfg, perms).unwrap();
}

fn show_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.args(["config", "show"]);
    cmd
}

#[test]
fn show_human_prints_all_sections() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    show_cmd(&home).assert().success().stdout(
        predicate::str::contains("paths")
            .and(predicate::str::contains("compiler"))
            .and(predicate::str::contains("behavior")),
    );
}

#[test]
fn show_format_json_is_valid_and_pipeable() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let output = show_cmd(&home).args(["--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is valid JSON");
    assert_eq!(parsed["compiler"]["engine"], "tectonic");
}

#[test]
fn show_format_toml_roundtrips() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let output = show_cmd(&home).args(["--format", "toml"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: toml::Value = toml::from_str(&stdout).expect("stdout is valid TOML");
    assert_eq!(parsed["compiler"]["engine"].as_str(), Some("tectonic"));
    assert_eq!(
        parsed["paths"]["templates_dir"].as_str(),
        Some("/tmp/templates")
    );
}

#[test]
fn show_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    // no config written

    show_cmd(&home)
        .assert()
        .failure()
        .code(10)
        .stderr(predicate::str::contains("tex-cli init"));
}

#[test]
fn show_corrupted_config_exits_11() {
    let home = TempDir::new().unwrap();
    let cfg = config_path(&home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::write(&cfg, "this is not valid toml === [[[").unwrap();

    show_cmd(&home)
        .assert()
        .failure()
        .code(11)
        .stderr(predicate::str::contains("invalid"));
}

#[test]
fn show_does_not_modify_mtime() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let cfg = config_path(&home);
    let before = std::fs::metadata(&cfg).unwrap().modified().unwrap();

    // Sleep a bit to make sure any accidental mtime change would be observable.
    std::thread::sleep(std::time::Duration::from_millis(50));

    show_cmd(&home).assert().success();

    let after = std::fs::metadata(&cfg).unwrap().modified().unwrap();
    assert_eq!(before, after, "config show must not modify mtime");
}
