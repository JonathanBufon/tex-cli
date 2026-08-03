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

fn seed_shell_escape_package(dir: &Path) -> PathBuf {
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
    std::fs::write(
        pkg.join("template.tex"),
        "\\documentclass{article}\n\
         \\immediate\\write18{echo pwned > /tmp/tex_cli_sandbox_test_pwned}\n\
         \\begin{document}attempt\\end{document}\n",
    )
    .unwrap();
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

/// FR-017: an installed third-party template attempting `\write18` must
/// fail the compile — either with the canonical `SandboxShellEscapeAttempted`
/// (exit 92) or a generic `CompileFailed` (exit 40) if tectonic's log tail
/// doesn't match our sandbox-signature scan. In both cases NO PDF must be
/// produced and the sentinel side-effect file must NOT exist.
#[test]
fn write18_in_installed_template_is_blocked() {
    if !requires_tectonic() {
        eprintln!("SKIP: tectonic not on PATH");
        return;
    }
    // Clean any leftover sentinel from a previous run.
    let _ = std::fs::remove_file("/tmp/tex_cli_sandbox_test_pwned");

    let home = TempDir::new().unwrap();
    let (_templates, output) = write_config(&home);
    let src = seed_shell_escape_package(home.path());
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

    // Exit code is either 92 (mapped SandboxShellEscapeAttempted) or 40
    // (generic CompileFailed). Both count as "sandbox held".
    let assert = base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .failure();
    let output_stderr = assert.get_output().stderr.clone();
    let stderr = String::from_utf8_lossy(&output_stderr);
    let code = assert.get_output().status.code();
    assert!(
        matches!(code, Some(92) | Some(40)),
        "unexpected exit code {code:?}, stderr={stderr}"
    );

    // Side-effect check: whatever exit code, the malicious side-effect
    // file must NOT have been created.
    assert!(
        !Path::new("/tmp/tex_cli_sandbox_test_pwned").exists(),
        "sandbox failed to contain \\write18 side-effect"
    );

    // And no PDF landed in output_dir.
    assert!(
        !output.join("badtemplate.pdf").exists(),
        "sandboxed compile still produced a PDF"
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
