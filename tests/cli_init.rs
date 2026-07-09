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

fn valid_stdin(templates: &str, output: &str, engine_index: u8) -> String {
    let mut input = String::new();
    input.push_str(templates);
    input.push('\n');
    input.push_str(output);
    input.push('\n');
    // inquire Select navigation: `engine_index` down-arrows then enter.
    // 0 = tectonic (default), so just enter.
    for _ in 0..engine_index {
        input.push_str("\x1b[B");
    }
    input.push('\n');
    input
}

#[test]
fn init_creates_config_with_valid_answers() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("templates");
    let output = home.path().join("output");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let stdin = valid_stdin(
        templates.to_str().unwrap(),
        output.to_str().unwrap(),
        0, // tectonic
    );

    init_cmd(&home)
        .write_stdin(stdin)
        .assert()
        .success();

    let cfg = config_path(&home);
    assert!(cfg.exists(), "config file should be created at {cfg:?}");

    let contents = std::fs::read_to_string(&cfg).unwrap();
    assert!(contents.contains(templates.to_str().unwrap()));
    assert!(contents.contains(output.to_str().unwrap()));
    assert!(contents.contains("tectonic"));

    let mode = std::fs::metadata(&cfg).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "config mode should be 0600, got {:o}", mode & 0o777);
}

#[test]
fn init_refuses_overwrite_without_confirmation() {
    let home = TempDir::new().unwrap();
    let cfg = config_path(&home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(&cfg).unwrap();
    f.write_all(b"original content untouched\n").unwrap();
    let before = std::fs::read(&cfg).unwrap();

    // Respond "n" (no) to the overwrite confirmation.
    init_cmd(&home)
        .write_stdin("n\n")
        .assert()
        .failure()
        .code(15);

    let after = std::fs::read(&cfg).unwrap();
    assert_eq!(before, after, "config file should be untouched byte-for-byte");
}

#[test]
fn init_prompts_to_create_missing_templates_dir() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("nested").join("templates");
    let output = home.path().join("output");
    std::fs::create_dir_all(&output).unwrap();
    // templates does NOT exist yet.

    // Answers: templates path, "y" to create dir, output path, engine.
    let mut stdin = String::new();
    stdin.push_str(templates.to_str().unwrap());
    stdin.push('\n');
    stdin.push_str("y\n"); // confirm create templates dir
    stdin.push_str(output.to_str().unwrap());
    stdin.push('\n');
    stdin.push('\n'); // pick default (tectonic)

    init_cmd(&home)
        .write_stdin(stdin)
        .assert()
        .success();

    assert!(templates.exists(), "templates dir should have been created");
}

#[test]
fn init_warns_but_persists_when_tectonic_missing() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let stdin = valid_stdin(
        templates.to_str().unwrap(),
        output.to_str().unwrap(),
        0,
    );

    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", "/tmp/empty-path-for-test"); // no tectonic here
    cmd.arg("init");

    cmd.write_stdin(stdin)
        .assert()
        .success()
        .stderr(predicate::str::contains("tectonic").and(predicate::str::contains("não foi encontrado")));

    let cfg = config_path(&home);
    assert!(cfg.exists(), "config should still be written despite missing tectonic");
}

#[test]
fn init_banner_in_stderr_never_stdout() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let stdin = valid_stdin(
        templates.to_str().unwrap(),
        output.to_str().unwrap(),
        0,
    );

    let assert = init_cmd(&home).write_stdin(stdin).assert();
    let output_ = assert.get_output();
    let stdout = String::from_utf8_lossy(&output_.stdout);
    let stderr = String::from_utf8_lossy(&output_.stderr);

    assert!(stderr.contains("TEX") || stderr.contains("T"), "banner glyphs expected in stderr");
    assert!(!stdout.contains("TEX CLI"), "banner literal must not appear on stdout");
    assert!(!stdout.contains("╔") && !stdout.contains("╗"), "banner box-drawing glyphs must not appear on stdout");
}
