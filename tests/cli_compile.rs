use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

const MINIMAL_TEX: &str = r#"\documentclass{article}
\begin{document}
Hello, world.
\end{document}
"#;

const BROKEN_TEX: &str = r#"\documentclass{article}
\begin{document}
\undefinedcommand{oops}
"#;

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_config(home: &TempDir, output_dir: &Path) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::create_dir_all(output_dir).unwrap();
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
        home.path().join("t").display(),
        output_dir.display()
    );
    std::fs::write(&cfg, body).unwrap();
}

fn compile_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.arg("compile");
    cmd
}

fn seed_tex(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.tex"));
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn compile_produces_pdf_with_default_output_path() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .success();

    let pdf = output.join("artigo.pdf");
    assert!(pdf.exists(), "PDF should exist at {pdf:?}");
}

#[test]
fn compile_pdf_has_pdf_magic_bytes() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .success();

    let bytes = std::fs::read(output.join("artigo.pdf")).unwrap();
    assert!(
        bytes.starts_with(b"%PDF-"),
        "expected PDF magic bytes, got {:?}",
        &bytes[..bytes.len().min(8)]
    );
}

#[test]
fn compile_pdf_has_mode_0644() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .success();

    let mode = std::fs::metadata(output.join("artigo.pdf"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644, "expected 0o644, got {:o}", mode & 0o777);
}

#[test]
fn compile_stdout_mentions_path_and_duration() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("PDF gerado em")
                .and(predicate::str::contains("Compilação levou"))
                .and(predicate::str::contains("s.")),
        );
}

#[test]
fn compile_missing_tex_exits_1() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);

    compile_cmd(&home)
        .arg("/tmp/definitely-nao-existe-xyz.tex")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn compile_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .failure()
        .code(10);
}

#[test]
fn compile_broken_tex_exits_40_with_log_tail() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "broken", BROKEN_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .failure()
        .code(40)
        .stderr(
            predicate::str::contains("Falha ao compilar")
                .and(predicate::str::contains("broken")),
        );

    assert!(
        !output.join("broken.pdf").exists(),
        "no PDF should be written on compile failure"
    );
}

#[test]
fn compile_overwrite_without_force_non_tty_exits_15() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    let pdf = output.join("artigo.pdf");
    std::fs::write(&pdf, b"original PDF").unwrap();

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .failure()
        .code(15);

    assert_eq!(std::fs::read(&pdf).unwrap(), b"original PDF");
}

#[test]
fn compile_force_overwrites_existing_pdf() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    let pdf = output.join("artigo.pdf");
    std::fs::write(&pdf, b"original").unwrap();

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("sobrescrito"));

    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"), "should be a real PDF after --force");
}

#[test]
fn compile_leaves_no_artefacts_in_tmp() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    let before: std::collections::HashSet<_> = std::fs::read_dir("/tmp")
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .success();

    let after: std::collections::HashSet<_> = std::fs::read_dir("/tmp")
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();

    let new_entries: Vec<_> = after.difference(&before).collect();
    // Filter out anything that isn't a `.tmp*` — TempDir uses that prefix.
    let leaked_tempdirs: Vec<_> = new_entries
        .iter()
        .filter(|n| {
            n.to_string_lossy().starts_with(".tmp")
                || n.to_string_lossy().contains("tex-cli")
        })
        .collect();
    assert!(
        leaked_tempdirs.is_empty(),
        "TempDir should be cleaned up; leaked: {:?}",
        leaked_tempdirs
    );
}
