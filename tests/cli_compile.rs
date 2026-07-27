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
    assert_eq!(
        mode & 0o777,
        0o644,
        "expected 0o644, got {:o}",
        mode & 0o777
    );
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
            predicate::str::contains("PDF generated at")
                .and(predicate::str::contains("Compile took"))
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
            predicate::str::contains("Failed to compile").and(predicate::str::contains("broken")),
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
        .stdout(predicate::str::contains("overwritten"));

    let bytes = std::fs::read(&pdf).unwrap();
    assert!(
        bytes.starts_with(b"%PDF-"),
        "should be a real PDF after --force"
    );
}

#[test]
fn compile_engine_flag_overrides_config_and_config_stays_unchanged() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    let cfg_before = std::fs::read_to_string(config_path(&home)).unwrap();

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .args(["--engine", "tectonic"])
        .assert()
        .success();

    let cfg_after = std::fs::read_to_string(config_path(&home)).unwrap();
    assert_eq!(
        cfg_before, cfg_after,
        "config file must not change when --engine is provided"
    );

    // Also validate content parseable via JSON:
    let json = Command::cargo_bin(BIN)
        .unwrap()
        .env_clear()
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join(".config"))
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .args(["config", "show", "--format", "json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(json.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["compiler"]["engine"], "tectonic");
}

#[test]
fn compile_engine_not_supported_exits_42() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .args(["--engine", "foo"])
        .assert()
        .failure()
        .code(42)
        .stderr(
            predicate::str::contains("Engine 'foo' is not supported")
                .and(predicate::str::contains("tectonic"))
                .and(predicate::str::contains("latexmk"))
                .and(predicate::str::contains("pdflatex"))
                .and(predicate::str::contains("xelatex"))
                .and(predicate::str::contains("lualatex")),
        );

    assert!(
        !output.join("artigo.pdf").exists(),
        "no PDF should be written when engine is not supported"
    );
}

#[test]
fn compile_engine_not_installed_exits_41() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    // Force PATH to a nonexistent dir so `which::which("tectonic")` fails.
    cmd.env("PATH", "/tmp/empty-path-for-test");
    cmd.arg("compile").arg(tex.to_str().unwrap());

    cmd.assert().failure().code(41).stderr(
        predicate::str::contains("Engine 'tectonic' is not installed on PATH")
            .and(predicate::str::contains("--engine")),
    );
}

fn write_config_with_keep_defaults(
    home: &TempDir,
    output_dir: &Path,
    keep_tex: bool,
    keep_logs: bool,
) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::create_dir_all(output_dir).unwrap();
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "{}"

[compiler]
engine = "tectonic"
keep_tex = {}
keep_logs = {}

[behavior]
ask_output_path_every_time = false
"#,
        home.path().join("t").display(),
        output_dir.display(),
        keep_tex,
        keep_logs
    );
    std::fs::write(&cfg, body).unwrap();
}

#[test]
fn compile_keep_tex_copies_source_to_output_dir() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--keep-tex")
        .assert()
        .success();

    let dest_tex = output.join("artigo.tex");
    assert!(dest_tex.exists(), "copied .tex should exist");
    assert_eq!(
        std::fs::read(&dest_tex).unwrap(),
        std::fs::read(&tex).unwrap()
    );
}

#[test]
fn compile_keep_logs_copies_log_to_output_dir() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--keep-logs")
        .assert()
        .success();

    let log = output.join("artigo.log");
    assert!(log.exists(), "copied .log should exist");
    let bytes = std::fs::read(&log).unwrap();
    assert!(!bytes.is_empty(), "log should be non-empty");
}

#[test]
fn compile_no_keep_tex_flags_leave_only_pdf() {
    // Config has keep_tex=true and keep_logs=true, but --no-* flags win.
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config_with_keep_defaults(&home, &output, true, true);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--no-keep-tex")
        .arg("--no-keep-logs")
        .assert()
        .success();

    assert!(output.join("artigo.pdf").exists());
    assert!(
        !output.join("artigo.tex").exists(),
        "--no-keep-tex should NOT copy .tex"
    );
    assert!(
        !output.join("artigo.log").exists(),
        "--no-keep-logs should NOT copy .log"
    );
}

#[test]
fn compile_conflicting_keep_tex_flags_exit_2() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--keep-tex")
        .arg("--no-keep-tex")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn compile_conflicting_keep_logs_flags_exit_2() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--keep-logs")
        .arg("--no-keep-logs")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn compile_flags_override_config_defaults() {
    // Config keep_tex=true; --no-keep-tex wins → no .tex copy.
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config_with_keep_defaults(&home, &output, true, false);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .arg("--no-keep-tex")
        .arg("--keep-logs")
        .assert()
        .success();

    assert!(!output.join("artigo.tex").exists());
    assert!(output.join("artigo.log").exists());
}

#[test]
fn compile_config_keep_tex_true_copies_without_flag() {
    // No CLI flag; config says keep_tex=true → .tex should be copied.
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config_with_keep_defaults(&home, &output, true, true);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .assert()
        .success();

    assert!(
        output.join("artigo.tex").exists(),
        "config keep_tex=true should copy .tex"
    );
    assert!(
        output.join("artigo.log").exists(),
        "config keep_logs=true should copy .log"
    );
}

#[test]
fn compile_engine_case_sensitive() {
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    compile_cmd(&home)
        .arg(tex.to_str().unwrap())
        .args(["--engine", "Tectonic"])
        .assert()
        .failure()
        .code(42);
}

#[test]
fn compile_leaves_no_artefacts_in_tmp() {
    // Isolate the compile process's TMPDIR to a dedicated dir so parallel
    // tests running their own TempDir in /tmp don't pollute this check.
    let home = TempDir::new().unwrap();
    let output = home.path().join("out");
    write_config(&home, &output);
    let tex = seed_tex(&home.path().join("src"), "artigo", MINIMAL_TEX);

    let isolated_tmp = home.path().join("isolated-tmp");
    std::fs::create_dir_all(&isolated_tmp).unwrap();

    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.env("TMPDIR", &isolated_tmp);
    cmd.arg("compile").arg(tex.to_str().unwrap());
    cmd.assert().success();

    // After compile drops its TempDir, our isolated TMPDIR should be empty.
    let residue: Vec<_> = std::fs::read_dir(&isolated_tmp)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();
    assert!(
        residue.is_empty(),
        "TempDir Drop should clean up isolated TMPDIR; leaked: {residue:?}"
    );
}
