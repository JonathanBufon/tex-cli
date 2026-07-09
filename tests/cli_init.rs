use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn init_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.arg("init");
    cmd
}

fn init_args(
    cmd: &mut Command,
    templates: &std::path::Path,
    output: &std::path::Path,
    engine: &str,
) {
    cmd.args([
        "--templates-dir",
        templates.to_str().unwrap(),
        "--output-dir",
        output.to_str().unwrap(),
        "--engine",
        engine,
    ]);
}

#[test]
fn init_creates_config_with_valid_answers() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("templates");
    let output = home.path().join("output");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let mut cmd = init_cmd(&home);
    init_args(&mut cmd, &templates, &output, "tectonic");
    cmd.assert().success();

    let cfg = config_path(&home);
    assert!(cfg.exists(), "config file should be created at {cfg:?}");

    let contents = std::fs::read_to_string(&cfg).unwrap();
    assert!(contents.contains(templates.to_str().unwrap()));
    assert!(contents.contains(output.to_str().unwrap()));
    assert!(contents.contains("tectonic"));

    let mode = std::fs::metadata(&cfg).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "config mode should be 0600, got {:o}",
        mode & 0o777
    );
}

#[test]
fn init_refuses_overwrite_without_force() {
    let home = TempDir::new().unwrap();
    let cfg = config_path(&home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&cfg).unwrap();
    f.write_all(b"original content untouched\n").unwrap();
    let before = std::fs::read(&cfg).unwrap();

    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    // No `--force` flag on a non-TTY invocation → inquire fails to
    // produce a confirm, which maps to a runtime error. The file must
    // stay untouched either way.
    let mut cmd = init_cmd(&home);
    init_args(&mut cmd, &templates, &output, "tectonic");
    cmd.assert().failure();

    let after = std::fs::read(&cfg).unwrap();
    assert_eq!(
        before, after,
        "config file should be untouched byte-for-byte"
    );
}

#[test]
fn init_force_overwrites_existing_config() {
    let home = TempDir::new().unwrap();
    let cfg = config_path(&home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::write(&cfg, "original content").unwrap();

    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let mut cmd = init_cmd(&home);
    init_args(&mut cmd, &templates, &output, "tectonic");
    cmd.arg("--force").assert().success();

    let contents = std::fs::read_to_string(&cfg).unwrap();
    assert!(contents.contains("tectonic"));
    assert!(!contents.contains("original content"));
}

#[test]
fn init_create_dirs_creates_missing_templates_dir() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("nested").join("templates");
    let output = home.path().join("output");
    std::fs::create_dir_all(&output).unwrap();

    let mut cmd = init_cmd(&home);
    init_args(&mut cmd, &templates, &output, "tectonic");
    cmd.arg("--create-dirs").assert().success();

    assert!(templates.exists(), "templates dir should have been created");
    assert!(config_path(&home).exists());
}

#[test]
fn init_warns_but_persists_when_tectonic_missing() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", "/tmp/empty-path-for-test"); // no tectonic here
    cmd.arg("init");
    init_args(&mut cmd, &templates, &output, "tectonic");

    cmd.assert().success().stderr(
        predicate::str::contains("tectonic").and(predicate::str::contains("não foi encontrado")),
    );

    let cfg = config_path(&home);
    assert!(
        cfg.exists(),
        "config should still be written despite missing tectonic"
    );
}

#[test]
fn init_banner_in_stderr_never_stdout() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let mut cmd = init_cmd(&home);
    init_args(&mut cmd, &templates, &output, "tectonic");
    let assert = cmd.assert();
    let output_ = assert.get_output();
    let stdout = String::from_utf8_lossy(&output_.stdout);
    let stderr = String::from_utf8_lossy(&output_.stderr);

    assert!(
        stderr.contains("╗") || stderr.contains("█"),
        "banner glyphs expected in stderr"
    );
    assert!(
        !stdout.contains("╔") && !stdout.contains("╗"),
        "banner box-drawing glyphs must not appear on stdout"
    );
    assert!(
        !stdout.contains("████"),
        "banner block glyphs must not appear on stdout"
    );
}
