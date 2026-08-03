//! Integration tests for spec 007 T017 — trust prompt behaviour.
//!
//! All tests run in non-TTY (assert_cmd) context, so the trust prompt
//! short-circuits to `TexError::TrustDenied` (exit code 70) rather than
//! blocking on `inquire::Confirm`. This is the correct scripted-invocation
//! behaviour per contracts/cli-build-pipeline.md exit code 6/70.

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

fn seed_package(dir: &Path, identifier: &str, version: &str) -> PathBuf {
    let pkg = dir.join(format!("pkg-{}", version));
    std::fs::create_dir_all(&pkg).unwrap();
    let manifest = format!(
        r#"identifier = "{identifier}"
version = "{version}"
entrypoint = "template.tex"
"#
    );
    std::fs::write(pkg.join("tex-template.toml"), manifest).unwrap();
    std::fs::write(
        pkg.join("template.tex"),
        "\\documentclass{article}\n\\begin{document}{{ hello }}\\end{document}\n",
    )
    .unwrap();
    pkg
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, body).unwrap();
    path
}

/// First compile against an installed, un-trusted template in a non-TTY
/// invocation → exit 70 (TrustDenied) with a message pointing at the
/// `tex-cli template trust` command.
#[test]
fn build_against_installed_without_trust_exits_70() {
    let home = TempDir::new().unwrap();
    write_config(&home);
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"template":"acme/invoice"},"hello":"hi"}"#,
    );

    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .code(70)
        .stderr(predicate::str::contains("acme/invoice"))
        .stderr(predicate::str::contains("1.0.0"));
}

/// After `template remove`, the trust records for that identifier are dropped
/// — a re-install of the same version re-prompts. We assert via the exit
/// code: pre-remove trust file grew, post-remove it doesn't contain the id.
#[test]
fn template_remove_drops_trust_records() {
    let home = TempDir::new().unwrap();
    write_config(&home);
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    // Prime a trust record manually — writing directly to trust.toml.
    let trust_path = home.path().join(".local/share/tex/trust.toml");
    std::fs::write(
        &trust_path,
        r#"schema_version = 1

[[trust]]
identifier = "acme/invoice"
version = "1.0.0"
approved_at = 0
"#,
    )
    .unwrap();
    assert!(std::fs::read_to_string(&trust_path)
        .unwrap()
        .contains("acme/invoice"));

    base_cmd(&home)
        .args(["template", "remove", "acme/invoice", "--yes"])
        .assert()
        .success();

    // Trust file should no longer contain the identifier.
    let after = std::fs::read_to_string(&trust_path).unwrap();
    assert!(
        !after.contains("acme/invoice"),
        "trust.toml still lists acme/invoice after remove:\n{after}"
    );
}

/// Version bump — install 1.1.0 on top of an already-trusted 1.0.0. A
/// non-TTY compile against the new version must exit 70 (the trust for
/// 1.0.0 does NOT carry to 1.1.0).
#[test]
fn version_bump_forces_reprompt_exits_70() {
    let home = TempDir::new().unwrap();
    write_config(&home);

    // Install 1.0.0 and grant trust manually.
    let src_1 = seed_package(home.path(), "acme/invoice", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src_1.to_str().unwrap()])
        .assert()
        .success();

    let trust_path = home.path().join(".local/share/tex/trust.toml");
    std::fs::create_dir_all(trust_path.parent().unwrap()).unwrap();
    std::fs::write(
        &trust_path,
        r#"schema_version = 1

[[trust]]
identifier = "acme/invoice"
version = "1.0.0"
approved_at = 0
"#,
    )
    .unwrap();

    // Bump to 1.1.0 via --force reinstall.
    let src_2 = seed_package(home.path(), "acme/invoice", "1.1.0");
    base_cmd(&home)
        .args(["template", "install", src_2.to_str().unwrap(), "--force"])
        .assert()
        .success();

    // Now the installed version is 1.1.0 but only 1.0.0 is trusted —
    // resolve picks 1.1.0, trust check fails.
    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"template":"acme/invoice"},"hello":"hi"}"#,
    );
    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .code(70)
        .stderr(predicate::str::contains("1.1.0"));
}

/// `tex-cli template trust <id> --version <v> --revoke` non-interactively
/// removes the specific version's record.
#[test]
fn template_trust_revoke_specific_version() {
    let home = TempDir::new().unwrap();
    write_config(&home);
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    let trust_path = home.path().join(".local/share/tex/trust.toml");
    std::fs::create_dir_all(trust_path.parent().unwrap()).unwrap();
    std::fs::write(
        &trust_path,
        r#"schema_version = 1

[[trust]]
identifier = "acme/invoice"
version = "1.0.0"
approved_at = 0
"#,
    )
    .unwrap();

    base_cmd(&home)
        .args([
            "template",
            "trust",
            "acme/invoice",
            "--version",
            "1.0.0",
            "--revoke",
        ])
        .assert()
        .success();

    let after = std::fs::read_to_string(&trust_path).unwrap();
    assert!(!after.contains("1.0.0"), "revoke did not drop the record");
}
