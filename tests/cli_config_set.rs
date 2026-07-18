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

fn set_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.args(["config", "set"]);
    cmd
}

fn parse_toml(cfg: &std::path::Path) -> toml::Value {
    toml::from_str(&std::fs::read_to_string(cfg).unwrap()).unwrap()
}

fn hash_file(cfg: &std::path::Path) -> Vec<u8> {
    std::fs::read(cfg).unwrap()
}

#[test]
fn set_boolean_flips_only_target_key() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let cfg = config_path(&home);
    let before = parse_toml(&cfg);

    set_cmd(&home)
        .args(["compiler.keep_tex", "false"])
        .assert()
        .success();

    let after = parse_toml(&cfg);
    assert_eq!(after["compiler"]["keep_tex"].as_bool(), Some(false));
    // Everything else preserved.
    assert_eq!(after["paths"], before["paths"]);
    assert_eq!(after["compiler"]["engine"], before["compiler"]["engine"]);
    assert_eq!(
        after["compiler"]["keep_logs"],
        before["compiler"]["keep_logs"]
    );
    assert_eq!(after["behavior"], before["behavior"]);
}

#[test]
fn set_path_expands_tilde() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let cfg = config_path(&home);

    set_cmd(&home)
        .args(["paths.templates_dir", "~/x"])
        .assert()
        .success();

    let after = parse_toml(&cfg);
    let path = after["paths"]["templates_dir"].as_str().unwrap();
    assert!(
        path.starts_with(home.path().to_str().unwrap()),
        "expected {} to start with {}",
        path,
        home.path().display()
    );
    assert!(path.ends_with("/x"), "expected {path} to end with /x");
    assert!(!path.contains('~'), "tilde must be expanded, got {path}");
}

#[test]
fn set_unknown_key_exits_12_and_lists_accepted() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let assert = set_cmd(&home).args(["foo.bar", "baz"]).assert();
    assert.failure().code(12).stderr(
        predicate::str::contains("foo.bar")
            .and(predicate::str::contains("paths.templates_dir"))
            .and(predicate::str::contains("paths.output_dir"))
            .and(predicate::str::contains("compiler.engine"))
            .and(predicate::str::contains("compiler.keep_tex"))
            .and(predicate::str::contains("compiler.keep_logs"))
            .and(predicate::str::contains(
                "behavior.ask_output_path_every_time",
            )),
    );
}

#[test]
fn set_invalid_bool_exits_13_and_leaves_file_intact() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let cfg = config_path(&home);
    let before = hash_file(&cfg);

    set_cmd(&home)
        .args(["compiler.keep_tex", "maybe"])
        .assert()
        .failure()
        .code(13)
        .stderr(predicate::str::contains("compiler.keep_tex"));

    let after = hash_file(&cfg);
    assert_eq!(before, after, "file must not change on invalid bool");
}

#[test]
fn set_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    // no config

    set_cmd(&home)
        .args(["compiler.keep_tex", "false"])
        .assert()
        .failure()
        .code(10);
}

#[test]
fn set_engine_non_tectonic_warns_but_persists() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let cfg = config_path(&home);

    set_cmd(&home)
        .args(["compiler.engine", "latexmk"])
        .assert()
        .success()
        .stderr(predicate::str::contains("latexmk").and(predicate::str::contains("not")));

    let after = parse_toml(&cfg);
    assert_eq!(after["compiler"]["engine"].as_str(), Some("latexmk"));
}

#[test]
fn set_path_nonexistent_dir_warns_but_persists() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let cfg = config_path(&home);
    let nonexistent = home.path().join("does").join("not").join("exist");

    set_cmd(&home)
        .args(["paths.templates_dir", nonexistent.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("does not exist"));

    let after = parse_toml(&cfg);
    assert_eq!(
        after["paths"]["templates_dir"].as_str(),
        Some(nonexistent.to_str().unwrap())
    );
}
