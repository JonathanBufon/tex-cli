//! Integration test for spec 007 T046 — FR-009 / SC-007 byte-identical PDFs
//! on repeat compile.
//!
//! Requires tectonic on PATH; skips when absent (Docker-gated).

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;

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

fn seed_template(dir: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{name}.tex")), body).unwrap();
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(format!("{name}.json"));
    std::fs::write(&p, body).unwrap();
    p
}

fn seed_installed_package(home: &TempDir, identifier: &str, version: &str) -> PathBuf {
    let pkg = home.path().join(format!("pkg-{version}"));
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("tex-template.toml"),
        format!("identifier = \"{identifier}\"\nversion = \"{version}\"\nentrypoint = \"t.tex\"\n"),
    )
    .unwrap();
    std::fs::write(
        pkg.join("t.tex"),
        "\\documentclass{article}\\begin{document}{{ hello }}\\end{document}",
    )
    .unwrap();
    pkg
}

fn prime_trust(home: &TempDir, id: &str, ver: &str) {
    let trust_path = home.path().join(".local/share/tex/trust.toml");
    std::fs::create_dir_all(trust_path.parent().unwrap()).unwrap();
    std::fs::write(
        &trust_path,
        format!(
            "schema_version = 1\n\n[[trust]]\nidentifier = \"{id}\"\nversion = \"{ver}\"\napproved_at = 0\n"
        ),
    )
    .unwrap();
}

/// FR-009 / SC-007: repeat compile of the same JSON + template produces
/// byte-identical PDFs (built-in path).
#[test]
fn repeat_compile_of_builtin_is_byte_identical() {
    if !requires_tectonic() {
        eprintln!("SKIP: tectonic not on PATH");
        return;
    }
    let home = TempDir::new().unwrap();
    let (templates, output) = write_config(&home);
    seed_template(
        &templates,
        "artigo",
        "\\documentclass{article}\\begin{document}{{ titulo }}\\end{document}",
    );
    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"type":"artigo"},"titulo":"Deterministic"}"#,
    );

    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .success();
    let pdf1 = std::fs::read(output.join("artigo.pdf")).unwrap();

    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap(), "--force"])
        .assert()
        .success();
    let pdf2 = std::fs::read(output.join("artigo.pdf")).unwrap();

    assert_eq!(
        pdf1.len(),
        pdf2.len(),
        "repeat builtin compile produced different-sized PDFs"
    );
    assert_eq!(
        pdf1, pdf2,
        "repeat builtin compile produced byte-different PDFs \
         (FR-009 / SC-007 regression)"
    );
}

/// FR-009 / SC-007: same guarantee for an installed third-party template
/// at a fixed version.
#[test]
fn repeat_compile_of_installed_at_fixed_version_is_byte_identical() {
    if !requires_tectonic() {
        eprintln!("SKIP: tectonic not on PATH");
        return;
    }
    let home = TempDir::new().unwrap();
    let (_templates, output) = write_config(&home);
    let src = seed_installed_package(&home, "det/pkg", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();
    prime_trust(&home, "det/pkg", "1.0.0");

    let json = seed_json(
        &home.path().join("d"),
        "d",
        r#"{"document":{"template":"det/pkg"},"hello":"same"}"#,
    );

    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap()])
        .assert()
        .success();
    let pdf1 = std::fs::read(output.join("pkg.pdf")).unwrap();

    base_cmd(&home)
        .args(["build", "--json", json.to_str().unwrap(), "--force"])
        .assert()
        .success();
    let pdf2 = std::fs::read(output.join("pkg.pdf")).unwrap();

    assert_eq!(
        pdf1, pdf2,
        "repeat installed-template compile produced byte-different PDFs \
         (FR-009 / SC-007 regression)"
    );
}
