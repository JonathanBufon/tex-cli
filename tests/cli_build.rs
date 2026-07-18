use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

const BIN: &str = "tex-cli";

const MINIMAL_TEMPLATE: &str = r#"\documentclass{article}
\begin{document}
{{ titulo }}
\end{document}
"#;

const BROKEN_TEMPLATE: &str = r#"\documentclass{article}
\begin{document}
{{ titulo }}
\undefinedcommand{oops}
"#;

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_config(home: &TempDir, templates_dir: &Path, output_dir: &Path) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::create_dir_all(templates_dir).unwrap();
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
        templates_dir.display(),
        output_dir.display()
    );
    std::fs::write(&cfg, body).unwrap();
}

fn build_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.arg("build");
    cmd
}

fn seed_template(templates_dir: &Path, name: &str, body: &str) {
    std::fs::create_dir_all(templates_dir).unwrap();
    let path = templates_dir.join(format!("{name}.tex"));
    std::fs::write(&path, body).unwrap();
}

fn seed_json(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn build_produces_pdf_with_default_output_path() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"Meu Artigo"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success();

    let pdf = output.join("artigo.pdf");
    assert!(pdf.exists(), "PDF should exist at {pdf:?}");
}

#[test]
fn build_pdf_has_pdf_magic_bytes() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success();

    let bytes = std::fs::read(output.join("artigo.pdf")).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn build_pdf_has_mode_0644() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success();

    let mode = std::fs::metadata(output.join("artigo.pdf"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644);
}

#[test]
fn build_stdout_mentions_path_and_total_duration() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("PDF generated at")
                .and(predicate::str::contains(
                    "Pipeline (render + compile) took",
                ))
                .and(predicate::str::contains("s.")),
        );
}

#[test]
fn build_with_output_flag_uses_custom_path() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let custom = home.path().join("custom.pdf");
    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--output"])
        .arg(custom.to_str().unwrap())
        .assert()
        .success();

    assert!(custom.exists());
    assert!(!output.join("artigo.pdf").exists());
}

#[test]
fn build_missing_template_exits_20() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{}"#);

    build_cmd(&home)
        .args(["nao-existe", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(20);
}

#[test]
fn build_malformed_json_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "bad", "not json {");

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(31)
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn build_json_top_level_array_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "arr", r#"["x"]"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(31);
}

#[test]
fn build_tera_error_exits_30() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    // JSON does not contain the {{ titulo }} variable.
    let data = seed_json(&home.path().join("d"), "empty", r#"{}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(30);
}

#[test]
fn build_broken_tex_after_render_exits_40_with_log_tail() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "broken", BROKEN_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["broken", data.to_str().unwrap()])
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
fn build_missing_config_exits_10() {
    let home = TempDir::new().unwrap();
    let data = seed_json(&home.path().join("d"), "d", r#"{}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(10);
}

#[test]
fn build_overwrite_without_force_non_tty_exits_15() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let pdf = output.join("artigo.pdf");
    std::fs::write(&pdf, b"original PDF").unwrap();

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap()])
        .assert()
        .failure()
        .code(15);

    assert_eq!(std::fs::read(&pdf).unwrap(), b"original PDF");
}

#[test]
fn build_force_overwrites_existing_pdf() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let pdf = output.join("artigo.pdf");
    std::fs::write(&pdf, b"original").unwrap();

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("overwritten"));

    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}

// --- US2: --keep-tex / --keep-logs -------------------------------------

fn write_config_with_keep_defaults(
    home: &TempDir,
    templates_dir: &Path,
    output_dir: &Path,
    keep_tex: bool,
    keep_logs: bool,
) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    std::fs::create_dir_all(templates_dir).unwrap();
    std::fs::create_dir_all(output_dir).unwrap();
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "{}"

[compiler]
engine = "tectonic"
keep_tex = {keep_tex}
keep_logs = {keep_logs}

[behavior]
ask_output_path_every_time = false
"#,
        templates_dir.display(),
        output_dir.display(),
    );
    std::fs::write(&cfg, body).unwrap();
}

#[test]
fn build_keep_tex_writes_intermediate_to_output_dir() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"Meu Artigo"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--keep-tex"])
        .assert()
        .success();

    let intermediate = output.join("artigo.tex");
    assert!(intermediate.exists(), "intermediate .tex should exist");
    let body = std::fs::read_to_string(&intermediate).unwrap();
    assert!(body.contains("Meu Artigo"));
    assert!(body.contains("\\documentclass{article}"));
}

#[test]
fn build_keep_logs_writes_log_to_output_dir() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--keep-logs"])
        .assert()
        .success();

    assert!(output.join("artigo.log").exists(), ".log should be copied");
}

#[test]
fn build_keep_tex_preserves_intermediate_after_compile_fail() {
    // Materializes SC-004 and FR-15: --keep-tex writes the .tex BEFORE
    // compile, so compile failure leaves the intermediate on disk for debug.
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "broken", BROKEN_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["broken", data.to_str().unwrap(), "--keep-tex"])
        .assert()
        .failure()
        .code(40);

    let intermediate = output.join("broken.tex");
    assert!(
        intermediate.exists(),
        "FR-15: intermediate .tex must remain after compile failure"
    );
    assert!(
        !output.join("broken.pdf").exists(),
        "no PDF should be written when compile fails"
    );
    let body = std::fs::read_to_string(&intermediate).unwrap();
    assert!(body.contains("\\undefinedcommand"));
}

#[test]
fn build_no_keep_tex_leaves_only_pdf() {
    // Config keep_tex=true but --no-keep-tex should suppress the .tex copy.
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config_with_keep_defaults(&home, &templates, &output, true, false);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--no-keep-tex"])
        .assert()
        .success();

    assert!(output.join("artigo.pdf").exists());
    assert!(!output.join("artigo.tex").exists());
}

#[test]
fn build_conflicting_keep_tex_flags_exit_2() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args([
            "artigo",
            data.to_str().unwrap(),
            "--keep-tex",
            "--no-keep-tex",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn build_conflicting_keep_logs_flags_exit_2() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args([
            "artigo",
            data.to_str().unwrap(),
            "--keep-logs",
            "--no-keep-logs",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn build_flags_override_config_defaults() {
    // Config keep_tex=true, keep_logs=true; flags disable both.
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config_with_keep_defaults(&home, &templates, &output, true, true);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args([
            "artigo",
            data.to_str().unwrap(),
            "--no-keep-tex",
            "--no-keep-logs",
        ])
        .assert()
        .success();

    assert!(output.join("artigo.pdf").exists());
    assert!(!output.join("artigo.tex").exists());
    assert!(!output.join("artigo.log").exists());
}

// --- US3: JSON via stdin ------------------------------------------------

#[test]
fn build_stdin_json_dash_arg() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);

    build_cmd(&home)
        .args(["artigo", "-"])
        .write_stdin(r#"{"titulo":"Do stdin"}"#)
        .assert()
        .success();

    let pdf = output.join("artigo.pdf");
    assert!(pdf.exists());
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn build_stdin_empty_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);

    build_cmd(&home)
        .args(["artigo", "-"])
        .write_stdin("")
        .assert()
        .failure()
        .code(31);
}

#[test]
fn build_stdin_malformed_exits_31() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);

    build_cmd(&home)
        .args(["artigo", "-"])
        .write_stdin("not json {")
        .assert()
        .failure()
        .code(31);
}

// --- Polish (T020): engine override, engine 41/42, tmpdir cleanliness ---

#[test]
fn build_engine_flag_overrides_config_and_config_stays_unchanged() {
    // SC-005: --engine override does not mutate config.
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let cfg_before = std::fs::read_to_string(config_path(&home)).unwrap();

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--engine", "tectonic"])
        .assert()
        .success();

    let cfg_after = std::fs::read_to_string(config_path(&home)).unwrap();
    assert_eq!(
        cfg_before, cfg_after,
        "config file must remain byte-identical"
    );
}

#[test]
fn build_engine_not_supported_exits_42() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    build_cmd(&home)
        .args(["artigo", data.to_str().unwrap(), "--engine", "foo"])
        .assert()
        .failure()
        .code(42)
        .stderr(
            predicate::str::contains("Engine 'foo' is not supported")
                .and(predicate::str::contains("tectonic")),
        );

    assert!(!output.join("artigo.pdf").exists());
}

#[test]
fn build_engine_not_installed_exits_41() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", "/tmp/empty-path-for-build-test");
    cmd.arg("build").arg("artigo").arg(data.to_str().unwrap());

    cmd.assert()
        .failure()
        .code(41)
        .stderr(predicate::str::contains(
            "Engine 'tectonic' is not installed on PATH",
        ));
}

#[test]
fn build_leaves_no_artefacts_in_tmp() {
    // SC-006: TempDir Drop must clean up after build, even on success.
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);
    let data = seed_json(&home.path().join("d"), "d", r#"{"titulo":"X"}"#);

    let isolated_tmp = home.path().join("isolated-tmp");
    std::fs::create_dir_all(&isolated_tmp).unwrap();

    let mut cmd = Command::cargo_bin(BIN).unwrap();
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.env("TMPDIR", &isolated_tmp);
    cmd.arg("build").arg("artigo").arg(data.to_str().unwrap());
    cmd.assert().success();

    let residue: Vec<_> = std::fs::read_dir(&isolated_tmp)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();
    assert!(
        residue.is_empty(),
        "TempDir Drop should clean up isolated TMPDIR; leaked: {residue:?}"
    );
}

#[test]
fn build_menu_non_tty_exits_1() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("out");
    seed_template(&templates, "artigo", MINIMAL_TEMPLATE);
    write_config(&home, &templates, &output);

    build_cmd(&home)
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("terminal"));
}
