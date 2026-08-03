//! Integration tests for spec 007 T018 — sandbox enforcement (FR-017).
//!
//! Sandbox verification requires an actual compile step, which needs
//! `tectonic` on PATH. Tests that need tectonic are marked with a
//! `requires_tectonic()` guard and skip when the binary is absent
//! (letting them pass locally without a full LaTeX toolchain and run
//! for real inside the project Docker container).

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

fn requires_tectonic() -> bool {
    which::which("tectonic").is_ok()
}

fn base_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("XDG_DATA_HOME", home.path().join(".local/share"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd
}

/// Initialise the config via `tex-cli init` so the Tectonic bundle-cache
/// warm-up runs. The sandboxed third-party compile uses `--only-cached`
/// and would fail with `SandboxBundleMissing` on a cold isolated cache —
/// which would mask the actual `\write18` rejection this suite is meant
/// to verify.
fn write_config(home: &TempDir) -> (PathBuf, PathBuf) {
    let templates = home.path().join("t");
    let output = home.path().join("out");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();
    base_cmd(home)
        .args([
            "init",
            "--templates-dir",
            templates.to_str().unwrap(),
            "--output-dir",
            output.to_str().unwrap(),
            "--engine",
            "tectonic",
            "--create-dirs",
        ])
        .assert()
        .success();
    (templates, output)
}

fn seed_shell_escape_package(dir: &Path, sentinel: &Path) -> PathBuf {
    let pkg = dir.join("pkg-write18");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("tex-template.toml"),
        r#"identifier = "acme/badtemplate"
version = "1.0.0"
entrypoint = "template.tex"
"#,
    )
    .unwrap();
    // Template attempts shell-escape via \write18 — must be blocked by FR-017.
    let template_body = format!(
        "\\documentclass{{article}}\n\
         \\immediate\\write18{{echo pwned > {}}}\n\
         \\begin{{document}}attempt\\end{{document}}\n",
        sentinel.display()
    );
    std::fs::write(pkg.join("template.tex"), template_body).unwrap();
    pkg
}

fn prime_trust(home: &TempDir, id: &str, ver: &str) {
    let trust_path = home.path().join(".local/share/tex/trust.toml");
    std::fs::create_dir_all(trust_path.parent().unwrap()).unwrap();
    let body = format!(
        r#"schema_version = 1

[[trust]]
identifier = "{id}"
version = "{ver}"
approved_at = 0
"#
    );
    std::fs::write(&trust_path, body).unwrap();
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, body).unwrap();
    path
}

/// FR-017: an installed third-party template attempting `\write18` MUST
/// have that shell-escape neutralised — the sentinel side-effect file
/// must NOT exist after the compile. Whether tectonic hard-fails the
/// compile (with `--untrusted`) or silently discards the `\write18`
/// (its default behaviour) is implementation detail — the actual
/// security guarantee is "the shell command did not execute".
#[test]
fn write18_in_installed_template_is_blocked() {
    if !requires_tectonic() {
        eprintln!("SKIP: tectonic not on PATH");
        return;
    }
    // Sentinel path is unique to this test run so parallel/repeat runs
    // never cross-contaminate.
    let sentinel = std::env::temp_dir().join(format!(
        "tex_cli_sandbox_test_pwned_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&sentinel);

    let home = TempDir::new().unwrap();
    let (_templates, _output) = write_config(&home);
    let src = seed_shell_escape_package(home.path(), &sentinel);
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();
    prime_trust(&home, "acme/badtemplate", "1.0.0");

    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"template":"acme/badtemplate"}}"#,
    );

    // Run the compile — do NOT assert on exit code. Under `--untrusted`
    // tectonic may hard-fail the compile; under the default it silently
    // discards \write18 and produces a PDF. Both are acceptable outcomes
    // per FR-017's "must be blocked" — the falsifiable check is the
    // sentinel file.
    let _ = base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert();

    assert!(
        !sentinel.exists(),
        "FR-017 regression: sandboxed template's \\write18 side-effect \
         file was created at {}",
        sentinel.display()
    );
}

/// Built-in templates keep their default (unrestricted) compile path —
/// existing spec-005 tests (cli_build.rs) already validate that flow;
/// this smoke test is a belt-and-suspenders assertion that the sandbox
/// flag threading did not accidentally down-grade the built-in path.
#[test]
fn builtin_compile_still_works_without_sandbox() {
    if !requires_tectonic() {
        eprintln!("SKIP: tectonic not on PATH");
        return;
    }
    let home = TempDir::new().unwrap();
    let (templates, output) = write_config(&home);
    std::fs::write(
        templates.join("artigo.tex"),
        "\\documentclass{article}\\begin{document}{{ titulo }}\\end{document}",
    )
    .unwrap();
    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"type":"artigo"},"titulo":"Sanity"}"#,
    );
    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("artigo"));
    assert!(output.join("artigo.pdf").exists());
}
