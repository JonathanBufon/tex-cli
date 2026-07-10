use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;

const BIN: &str = "tex-cli";
const BANNER_ASSET: &str = include_str!("../assets/banner.txt");

fn config_path(home: &TempDir) -> PathBuf {
    home.path().join(".config").join("tex").join("config.toml")
}

fn write_valid_config(home: &TempDir) {
    let cfg = config_path(home);
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
    let body = r#"[paths]
templates_dir = "/tmp/templates"
output_dir = "/tmp/output"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#;
    std::fs::write(&cfg, body).unwrap();
    let mut perms = std::fs::metadata(&cfg).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&cfg, perms).unwrap();
}

fn base_cmd(home: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin(BIN).expect("binary built");
    cmd.env_clear();
    cmd.env("HOME", home.path());
    cmd.env("XDG_CONFIG_HOME", home.path().join(".config"));
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd
}

/// A distinctive substring from the banner asset — first non-empty
/// line, which contains block-drawing glyphs unlikely to appear in
/// any normal payload.
fn banner_marker() -> String {
    BANNER_ASSET
        .lines()
        .find(|l| !l.trim().is_empty())
        .expect("banner asset should have at least one non-empty line")
        .to_string()
}

fn assert_banner_in_stderr_only(stdout: &str, stderr: &str) {
    let marker = banner_marker();
    assert!(
        stderr.contains(&marker),
        "banner marker `{marker}` should be present in stderr, got: {stderr:?}"
    );
    assert!(
        !stdout.contains(&marker),
        "banner marker `{marker}` should NOT appear in stdout, got: {stdout:?}"
    );
}

#[test]
fn banner_appears_on_init_stderr() {
    let home = TempDir::new().unwrap();
    let templates = home.path().join("t");
    let output = home.path().join("o");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&output).unwrap();

    let out = base_cmd(&home)
        .arg("init")
        .args([
            "--templates-dir",
            templates.to_str().unwrap(),
            "--output-dir",
            output.to_str().unwrap(),
            "--engine",
            "tectonic",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_appears_on_config_show_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let out = base_cmd(&home).args(["config", "show"]).output().unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_appears_on_config_set_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let out = base_cmd(&home)
        .args(["config", "set", "compiler.keep_tex", "false"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_never_appears_on_stdout_for_config_show_json() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let out = base_cmd(&home)
        .args(["config", "show", "--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    // Every non-empty banner line must be absent from stdout so
    // `config show --format=json | jq` works cleanly.
    for line in BANNER_ASSET.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            !stdout.contains(line),
            "banner line leaked into stdout: `{line}` — full stdout: {stdout:?}"
        );
    }
}

#[test]
fn banner_appears_on_templates_list_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = home.path().join(".config").join("tex").join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(templates_dir.join("x.tex"), "% x\n").unwrap();
    rewrite_config_pointing_at(&home, &templates_dir);

    let out = base_cmd(&home)
        .args(["templates", "list"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_appears_on_templates_show_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = home.path().join(".config").join("tex").join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(templates_dir.join("x.tex"), "% x\n").unwrap();
    rewrite_config_pointing_at(&home, &templates_dir);

    let out = base_cmd(&home)
        .args(["templates", "show", "x"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_appears_on_templates_add_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = home.path().join(".config").join("tex").join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    rewrite_config_pointing_at(&home, &templates_dir);

    let source = home.path().join("src.tex");
    std::fs::write(&source, "% src\n").unwrap();

    let out = base_cmd(&home)
        .args(["templates", "add"])
        .arg(source.to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_appears_on_templates_remove_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = home.path().join(".config").join("tex").join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::write(templates_dir.join("x.tex"), "% x\n").unwrap();
    rewrite_config_pointing_at(&home, &templates_dir);

    let out = base_cmd(&home)
        .args(["templates", "remove", "x", "--force"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

fn rewrite_config_pointing_at(home: &TempDir, templates_dir: &std::path::Path) {
    let cfg = config_path(home);
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "/tmp/output"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#,
        templates_dir.display()
    );
    std::fs::write(&cfg, body).unwrap();
    let mut perms = std::fs::metadata(&cfg).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&cfg, perms).unwrap();
}

#[test]
fn banner_appears_on_render_stderr() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = home.path().join(".config").join("tex").join("templates");
    let output_dir = home.path().join(".config").join("tex").join("output");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(templates_dir.join("x.tex"), "{{ n }}\n").unwrap();
    rewrite_config_pointing_both(&home, &templates_dir, &output_dir);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{"n":"X"}"#).unwrap();

    let out = base_cmd(&home)
        .args(["render", "x"])
        .arg(data.to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_banner_in_stderr_only(&stdout, &stderr);
}

#[test]
fn banner_never_leaks_on_render_dry_run() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);
    let templates_dir = home.path().join(".config").join("tex").join("templates");
    let output_dir = home.path().join(".config").join("tex").join("output");
    std::fs::create_dir_all(&templates_dir).unwrap();
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(templates_dir.join("x.tex"), "hello {{ n }}\n").unwrap();
    rewrite_config_pointing_both(&home, &templates_dir, &output_dir);

    let data = home.path().join("d.json");
    std::fs::write(&data, br#"{"n":"World"}"#).unwrap();

    let out = base_cmd(&home)
        .args(["render", "x"])
        .arg(data.to_str().unwrap())
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in BANNER_ASSET.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            !stdout.contains(line),
            "banner line leaked into dry-run stdout: `{line}`"
        );
    }
    // But the rendered content IS on stdout:
    assert!(stdout.contains("hello World"));
}

fn rewrite_config_pointing_both(
    home: &TempDir,
    templates_dir: &std::path::Path,
    output_dir: &std::path::Path,
) {
    let cfg = config_path(home);
    let body = format!(
        r#"[paths]
templates_dir = "{}"
output_dir = "{}"

[compiler]
engine = "tectonic"
keep_tex = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
"#,
        templates_dir.display(),
        output_dir.display()
    );
    std::fs::write(&cfg, body).unwrap();
    let mut perms = std::fs::metadata(&cfg).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&cfg, perms).unwrap();
}

#[test]
fn banner_content_matches_asset() {
    let home = TempDir::new().unwrap();
    write_valid_config(&home);

    let out = base_cmd(&home).args(["config", "show"]).output().unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in BANNER_ASSET.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            stderr.contains(line),
            "banner line missing from stderr: `{line}`"
        );
    }
}
