//! T017 — English-denylist smoke test.
//!
//! Captures the `--help` output of the top-level CLI plus every
//! subcommand and asserts no Portuguese words from a curated denylist
//! appear. This locks in the SC-002 acceptance metric of spec 006.

use assert_cmd::Command;

const BIN: &str = "tex-cli";

const DENYLIST: &[&str] = &[
    // High-frequency function words that would leak from mistranslations.
    "Nenhum",
    "Sobrescrever",
    "Sobrescreve",
    "Renderiza",
    "renderizar",
    "Renderizado",
    "Compilação",
    "Compilando",
    "Compilando",
    "Diretório",
    "diretório",
    "Manter",
    "arquivo",
    "caminho",
    "usuário",
    "padrão",
    "cancelada",
    "Últimas",
    "Cria",
    "Copia",
    "Descarta",
    "Gerencia",
    "Aviso",
    "Inspeciona",
    "Aumenta",
    "Altera",
    "Adiciona",
    "Lista",
    // Verify the humano default value was removed.
    "humano",
    "Humano",
];

fn help_output(args: &[&str]) -> String {
    let out = Command::cargo_bin(BIN)
        .expect("binary built")
        .args(args)
        .output()
        .expect("run help");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    format!("{stdout}\n{stderr}")
}

fn assert_no_portuguese(context: &str, text: &str) {
    for word in DENYLIST {
        assert!(
            !text.contains(word),
            "Portuguese word '{word}' found in '{context}' output. \
             Full output:\n{text}"
        );
    }
}

#[test]
fn help_top_level_has_no_portuguese() {
    let out = help_output(&["--help"]);
    assert_no_portuguese("tex-cli --help", &out);
}

#[test]
fn help_init_has_no_portuguese() {
    let out = help_output(&["init", "--help"]);
    assert_no_portuguese("tex-cli init --help", &out);
}

#[test]
fn help_config_show_has_no_portuguese() {
    let out = help_output(&["config", "show", "--help"]);
    assert_no_portuguese("tex-cli config show --help", &out);
}

#[test]
fn help_config_set_has_no_portuguese() {
    let out = help_output(&["config", "set", "--help"]);
    assert_no_portuguese("tex-cli config set --help", &out);
}

#[test]
fn help_templates_has_no_portuguese() {
    let out = help_output(&["templates", "--help"]);
    assert_no_portuguese("tex-cli templates --help", &out);
}

#[test]
fn help_templates_list_has_no_portuguese() {
    let out = help_output(&["templates", "list", "--help"]);
    assert_no_portuguese("tex-cli templates list --help", &out);
}

#[test]
fn help_templates_show_has_no_portuguese() {
    let out = help_output(&["templates", "show", "--help"]);
    assert_no_portuguese("tex-cli templates show --help", &out);
}

#[test]
fn help_templates_add_has_no_portuguese() {
    let out = help_output(&["templates", "add", "--help"]);
    assert_no_portuguese("tex-cli templates add --help", &out);
}

#[test]
fn help_templates_remove_has_no_portuguese() {
    let out = help_output(&["templates", "remove", "--help"]);
    assert_no_portuguese("tex-cli templates remove --help", &out);
}

#[test]
fn help_render_has_no_portuguese() {
    let out = help_output(&["render", "--help"]);
    assert_no_portuguese("tex-cli render --help", &out);
}

#[test]
fn help_compile_has_no_portuguese() {
    let out = help_output(&["compile", "--help"]);
    assert_no_portuguese("tex-cli compile --help", &out);
}

#[test]
fn help_build_has_no_portuguese() {
    let out = help_output(&["build", "--help"]);
    assert_no_portuguese("tex-cli build --help", &out);
}
