//! Integration tests for spec 007 T016 — `tex-cli template install`.
//!
//! Local-path install exercises manifest validation, destination layout,
//! and `--force` overwrite. Git-URL install requires `git` on PATH; those
//! tests are annotated but do not attempt a real remote clone.

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

fn seed_package(dir: &Path, identifier: &str, version: &str) -> PathBuf {
    let pkg = dir.join("pkg");
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

#[test]
fn install_from_local_path_succeeds_and_prints_dest() {
    let home = TempDir::new().unwrap();
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");

    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("installed acme/invoice@1.0.0"))
        .stdout(predicate::str::contains("acme/invoice"));

    // Layout on disk matches R5.
    let dest = home
        .path()
        .join(".local/share/tex/templates/acme/invoice");
    assert!(dest.join("tex-template.toml").exists());
    assert!(dest.join("template.tex").exists());
}

#[test]
fn install_refuses_overwrite_without_force_exit_53() {
    let home = TempDir::new().unwrap();
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");

    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .code(53)
        .stderr(predicate::str::contains("--force"));
}

#[test]
fn install_with_force_overwrites() {
    let home = TempDir::new().unwrap();
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");

    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap(), "--force"])
        .assert()
        .success();
}

#[test]
fn install_rejects_bare_identifier_exit_63() {
    let home = TempDir::new().unwrap();
    // Bare "invoice" — no namespace — fails FR-018 install-time rejection.
    let src = seed_package(home.path(), "invoice", "1.0.0");

    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .code(63)
        .stderr(predicate::str::contains("namespace/name"));
}

#[test]
fn install_rejects_missing_manifest_exit_60() {
    let home = TempDir::new().unwrap();
    let empty = home.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();

    base_cmd(&home)
        .args(["template", "install", empty.to_str().unwrap()])
        .assert()
        .code(60)
        .stderr(predicate::str::contains("tex-template.toml"));
}

#[test]
fn install_rejects_invalid_version_exit_64() {
    let home = TempDir::new().unwrap();
    let pkg = home.path().join("bad-ver");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("tex-template.toml"),
        "identifier = \"a/b\"\nversion = \"not-semver\"\nentrypoint = \"template.tex\"\n",
    )
    .unwrap();
    std::fs::write(pkg.join("template.tex"), "\\documentclass{article}").unwrap();

    base_cmd(&home)
        .args(["template", "install", pkg.to_str().unwrap()])
        .assert()
        .code(64);
}

#[test]
fn install_rejects_entrypoint_not_tex_exit_67() {
    let home = TempDir::new().unwrap();
    let pkg = home.path().join("wrong-ext");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("tex-template.toml"),
        "identifier = \"a/b\"\nversion = \"1.0.0\"\nentrypoint = \"template.txt\"\n",
    )
    .unwrap();
    std::fs::write(pkg.join("template.txt"), "not tex").unwrap();

    base_cmd(&home)
        .args(["template", "install", pkg.to_str().unwrap()])
        .assert()
        .code(67)
        .stderr(predicate::str::contains(".tex"));
}

#[test]
fn install_rejects_entrypoint_escape_exit_65() {
    let home = TempDir::new().unwrap();
    let pkg = home.path().join("escape");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("tex-template.toml"),
        "identifier = \"a/b\"\nversion = \"1.0.0\"\nentrypoint = \"../evil.tex\"\n",
    )
    .unwrap();

    base_cmd(&home)
        .args(["template", "install", pkg.to_str().unwrap()])
        .assert()
        .code(65);
}

#[test]
fn install_then_list_shows_the_package() {
    let home = TempDir::new().unwrap();
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    base_cmd(&home)
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("acme/invoice"))
        .stdout(predicate::str::contains("1.0.0"));
}

#[test]
fn install_then_remove_deletes_the_package_dir() {
    let home = TempDir::new().unwrap();
    let src = seed_package(home.path(), "acme/invoice", "1.0.0");
    base_cmd(&home)
        .args(["template", "install", src.to_str().unwrap()])
        .assert()
        .success();

    base_cmd(&home)
        .args(["template", "remove", "acme/invoice", "--yes"])
        .assert()
        .success();

    let dest = home
        .path()
        .join(".local/share/tex/templates/acme/invoice");
    assert!(!dest.exists());
}
