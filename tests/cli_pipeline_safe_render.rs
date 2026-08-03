//! Integration tests for spec 007 US3 (T034):
//! * FR-005 universal escaping of LaTeX specials in JSON string values.
//! * A-06 em-dash / en-dash substitution logged via tracing (no silent drop).
//! * FR-006 template-engine ↔ LaTeX collision detected at load time and
//!   surfaced with a targeted diagnostic.
//!
//! Uses `tex-cli render --dry-run` — writes rendered LaTeX to stdout without
//! invoking the compiler, so these tests do NOT require tectonic.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn base_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("XDG_DATA_HOME", home.path().join(".local/share"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd
}

fn write_config(home: &TempDir) -> (PathBuf, PathBuf) {
    let templates = home.path().join("t");
    let output = home.path().join("out");
    let cfg_path = home.path().join(".config").join("tex").join("config.toml");
    std::fs::create_dir_all(cfg_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "{}"

[compiler]
engine = "tectonic"
keep_tex = false
keep_logs = false

[behavior]
ask_output_path_every_time = false
"#,
        templates.display(),
        output.display()
    );
    std::fs::write(&cfg_path, body).unwrap();
    (templates, output)
}

fn seed_template(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(format!("{name}.tex")), body).unwrap();
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(format!("{name}.json"));
    std::fs::write(&p, body).unwrap();
    p
}

/// FR-005: every LaTeX special in a JSON string value is escaped before
/// tera sees it. The rendered `.tex` should contain the escaped forms,
/// never the raw specials in a position that would fail to compile.
#[test]
fn render_escapes_all_ten_latex_specials_in_json_value() {
    let home = TempDir::new().unwrap();
    let (templates, _output) = write_config(&home);
    seed_template(&templates, "tpl", "X{{ x }}Y");
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"x":"& % $ # _ { } \\ ~ ^"}"#,
    );

    base_cmd(&home)
        .args([
            "render",
            "tpl",
            data.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r"\&"))
        .stdout(predicate::str::contains(r"\%"))
        .stdout(predicate::str::contains(r"\$"))
        .stdout(predicate::str::contains(r"\#"))
        .stdout(predicate::str::contains(r"\_"))
        .stdout(predicate::str::contains(r"\{"))
        .stdout(predicate::str::contains(r"\}"))
        .stdout(predicate::str::contains(r"\textbackslash{}"))
        .stdout(predicate::str::contains(r"\textasciitilde{}"))
        .stdout(predicate::str::contains(r"\textasciicircum{}"));
}

/// A-06: em-dash and en-dash are substituted with LaTeX shorthand AND
/// reported via `tracing::warn!` to stderr (with `-vv` for DEBUG level).
#[test]
fn render_substitutes_em_dash_and_reports_via_tracing() {
    let home = TempDir::new().unwrap();
    let (templates, _output) = write_config(&home);
    seed_template(&templates, "tpl", "{{ text }}");
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"text":"before—after"}"#,
    );

    let assert = base_cmd(&home)
        .args([
            "-vv",
            "render",
            "tpl",
            data.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(
        stdout.contains("before---after"),
        "em-dash not substituted; stdout={stdout}"
    );
    // Substitution reported via tracing::warn! (goes to stderr).
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("substituted") || stderr.contains("unrepresentable"),
        "substitution not reported to stderr; stderr={stderr}"
    );
}

/// A-06 en-dash: same treatment as em-dash — substituted with `--`.
#[test]
fn render_substitutes_en_dash() {
    let home = TempDir::new().unwrap();
    let (templates, _output) = write_config(&home);
    seed_template(&templates, "tpl", "{{ text }}");
    let data = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"text":"pages 10–20"}"#,
    );

    base_cmd(&home)
        .args([
            "render",
            "tpl",
            data.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pages 10--20"));
}

/// FR-006 / SC-004: a template whose LaTeX contains `\macro{#N}` collides
/// with Tera's comment opener `{#` and fails at load time with a message
/// naming the exact character sequence, NOT an opaque Tera parse error.
#[test]
fn template_engine_collision_detected_at_load_time_with_line_and_col() {
    let home = TempDir::new().unwrap();
    let (templates, _output) = write_config(&home);
    seed_template(
        &templates,
        "bad",
        "\\documentclass{article}\n\\begin{document}\n\\MakeUppercase{#1}\n\\end{document}\n",
    );
    let data = seed_json(&home.path().join("d"), "d", r#"{"x":"whatever"}"#);

    base_cmd(&home)
        .args([
            "render",
            "bad",
            data.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("collision"))
        .stderr(predicate::str::contains("line 3"));
}

/// Ensure a "normal" template (no collision, no specials in JSON) still
/// renders unmodified — regression guard for the escape/collision path.
#[test]
fn plain_template_and_plain_json_renders_verbatim() {
    let home = TempDir::new().unwrap();
    let (templates, _output) = write_config(&home);
    seed_template(&templates, "tpl", "Hello {{ name }}!");
    let data = seed_json(&home.path().join("d"), "d", r#"{"name":"world"}"#);

    base_cmd(&home)
        .args([
            "render",
            "tpl",
            data.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello world!"));
}
