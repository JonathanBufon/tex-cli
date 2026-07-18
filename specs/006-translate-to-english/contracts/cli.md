# Phase 1 — CLI Contract: Translate All User-Facing Strings to English

**Feature**: `006-translate-to-english`
**Date**: 2026-07-18

Documents the **new English-language user-visible surface** after
this feature ships. This is the source of truth used by both the
implementation and the tests to know what to write and what to
assert.

Every entry follows the same shape: **Before (PT)** → **After (EN)**.

Exit codes and structural positions are preserved throughout — only
the natural-language tokens change.

---

## 1. `TexError` display messages (`src/errors.rs`)

### `ConfigMissing` — exit 10

- **Before**: `Nenhum config encontrado. Rode 'tex-cli init' primeiro.`
- **After**: `No config found. Run 'tex-cli init' first.`

### `ConfigCorrupted { path, detail }` — exit 11

- **Before**: `Config em {path} está inválido: {detail}. Rode 'tex-cli init' novamente para recriar.`
- **After**: `Config at {path} is invalid: {detail}. Run 'tex-cli init' again to recreate.`

### `UnknownKey { key, accepted }` — exit 12

- **Before**: `Chave desconhecida: '{key}'. Chaves aceitas:\n{list}`
- **After**: `Unknown key: '{key}'. Accepted keys:\n{list}`

### `InvalidBoolValue { key, value }` — exit 13

- **Before**: `Valor inválido para chave booleana '{key}': '{value}'. Aceito: true, false.`
- **After**: `Invalid value for boolean key '{key}': '{value}'. Accepted: true, false.`

### `PermissionDenied { path }` — exit 14

- **Before**: `Permissão negada ao acessar {path}.`
- **After**: `Permission denied while accessing {path}.`

### `UserAborted` — exit 15

- **Before**: `Operação cancelada pelo usuário.`
- **After**: `Operation cancelled by user.`

### `HomeDirUnavailable` — exit 1

- **Before**: `Não foi possível resolver o diretório home do usuário.`
- **After**: `Could not resolve the user's home directory.`

### `TemplateNotFound { name, templates_dir }` — exit 20

- **Before**: `Template '{name}' não existe em {templates_dir}.`
- **After**: `Template '{name}' not found in {templates_dir}.`

### `InvalidUtf8 { source_path, detail }` — exit 21

- **Before**: `Arquivo '{path}' não é texto UTF-8 válido: {detail}.`
- **After**: `File '{path}' is not valid UTF-8 text: {detail}.`

### `TemplatesDirMissing { templates_dir }` — exit 22

- **Before**: `Diretório de templates '{path}' não existe. Rode 'tex-cli config set paths.templates_dir <path>' ou crie o diretório.`
- **After**: `Templates directory '{path}' not found. Run 'tex-cli config set paths.templates_dir <path>' or create the directory.`

### `TeraRenderError { template_name, detail }` — exit 30

- **Before**: `Falha ao renderizar template '{template_name}': {detail}`
- **After**: `Failed to render template '{template_name}': {detail}`

### `InvalidJson { source_name, detail }` — exit 31

- **Before**: `JSON inválido em {source_name}: {detail}`
- **After**: `Invalid JSON in {source_name}: {detail}`

### `CompileFailed { engine, tex_path, log_tail }` — exit 40

- **Before**: `Falha ao compilar '{path}': engine '{engine}' retornou erro.\nÚltimas linhas do log:\n{log_tail}\n`
- **After**: `Failed to compile '{path}': engine '{engine}' returned an error.\nLast lines of the log:\n{log_tail}\n`

### `EngineNotInstalled { engine }` — exit 41

- **Before**: `Engine '{engine}' não está instalado no PATH. Instale-o antes ou use --engine <outro>.`
- **After**: `Engine '{engine}' is not installed on PATH. Install it or use --engine <other>.`

### `EngineNotSupported { engine, accepted }` — exit 42

- **Before**: `Engine '{engine}' não é suportado. Aceitos:\n{list}`
- **After**: `Engine '{engine}' is not supported. Accepted:\n{list}`

### `Io(source)` — exit 1

- **Before**: `Erro de I/O: {source}`
- **After**: `I/O error: {source}`

---

## 2. `--help` doc comments (`src/cli.rs`)

Each `///` comment on a subcommand, arg struct, or flag becomes the
`--help` text. Every one is translated. Sample below; the
implementation will sweep the full file exhaustively (see
`plan.md`).

### Top-level `Cli`

- **Before**: (varies) — CLI top-level doc.
- **After**: `Convert JSON data into LaTeX documents and compile them to PDF.`

### Subcommand descriptions

| Subcommand   | Before                                                                   | After                                                                          |
|--------------|--------------------------------------------------------------------------|--------------------------------------------------------------------------------|
| `init`       | `Cria o config em ~/.config/tex/config.toml (interativo ou via flags).`   | `Create the config at ~/.config/tex/config.toml (interactive or via flags).`   |
| `config`     | `Inspeciona ou altera o config atual.`                                   | `Inspect or update the current config.`                                        |
| `templates`  | `Gerencia templates LaTeX em `paths.templates_dir`.`                     | `Manage LaTeX templates in `paths.templates_dir`.`                             |
| `render`     | `Renderiza um template com dados JSON, produzindo um `.tex`.`            | `Render a template with JSON data, producing a `.tex`.`                        |
| `compile`    | `Compila um `.tex` para PDF usando o engine configurado.`               | `Compile a `.tex` to PDF using the configured engine.`                         |
| `build`      | `Pipeline JSON → PDF: renderiza template + compila em uma invocação.` | `JSON → PDF pipeline: render template + compile in one invocation.`         |

### Flag descriptions (sample — all translated)

| Flag              | Before                                                                                                                             | After                                                                                                                          |
|-------------------|------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------|
| `--output`        | `Caminho custom do PDF final. Default: `paths.output_dir/<template>.pdf`.`                                                        | `Custom path for the final PDF. Default: `paths.output_dir/<template>.pdf`.`                                                  |
| `--engine`        | `Engine LaTeX (`tectonic`, `latexmk`, ...). Sobrescreve `compiler.engine` do config só nesta invocação.`                        | `LaTeX engine (`tectonic`, `latexmk`, ...). Overrides `compiler.engine` for this invocation only.`                          |
| `--keep-tex`      | `Copia o `.tex` intermediário para `output_dir` ANTES do compile.`                                                                | `Copy the intermediate `.tex` to `output_dir` BEFORE compile.`                                                                |
| `--no-keep-tex`   | `Descarta o `.tex` intermediário mesmo se config diz true.`                                                                        | `Discard the intermediate `.tex` even if config says true.`                                                                    |
| `--keep-logs`     | `Copia o `.log` do engine para `output_dir` após compile.`                                                                        | `Copy the engine's `.log` to `output_dir` after compile.`                                                                     |
| `--no-keep-logs`  | `Descarta o `.log` mesmo se config diz true.`                                                                                     | `Discard the `.log` even if config says true.`                                                                                 |
| `--force`         | `Sobrescreve PDF existente sem pedir confirmação.`                                                                                 | `Overwrite an existing PDF without confirming.`                                                                                |
| `--create-dirs`   | `Cria diretórios ausentes sem pedir confirmação.`                                                                                  | `Create missing directories without confirming.`                                                                               |
| `-t/--templates-dir` | `Diretório de templates LaTeX (pula o prompt correspondente).`                                                                  | `LaTeX templates directory (skips the corresponding prompt).`                                                                  |
| `-o/--output-dir` | `Diretório padrão de saída dos PDFs (pula o prompt correspondente).`                                                              | `Default output directory for PDFs (skips the corresponding prompt).`                                                          |
| `-e/--engine` (init) | `Nome do engine LaTeX (`tectonic`, `latexmk`, ...).`                                                                            | `Name of the LaTeX engine (`tectonic`, `latexmk`, ...).`                                                                       |
| `--dry-run`       | `Imprime o renderizado em stdout, não grava arquivo. Ignora --output.`                                                            | `Print the rendered output to stdout instead of writing a file. Ignores --output.`                                             |
| `--name` (add)    | `Nome final do template (default = basename do arquivo, sem `.tex`).`                                                             | `Final template name (default = file basename without `.tex`).`                                                                |

**Note**: This table is representative, not exhaustive. `src/cli.rs`
has ~50 doc comments in total, all following the same translation
pattern. Every single one is swept during implementation.

---

## 3. `--format` default value

- **Before**: `#[arg(long, default_value = "humano")]` on
  `templates list` and `config show`.
- **After**: `#[arg(long, default_value = "human")]`.

**Consequence**:

- `tex-cli templates list` (no `--format`) → human output. Unchanged
  behavior.
- `tex-cli templates list --format human` → human output. New alias
  (identical to default).
- `tex-cli templates list --format humano` → **clap parse error, exit
  code 2**, stderr lists accepted values.

---

## 4. Interactive prompts (`src/interactive.rs`)

| Prompt kind | Before                                                | After                                                     |
|-------------|-------------------------------------------------------|-----------------------------------------------------------|
| Text        | `Diretório de templates LaTeX:`                       | `LaTeX templates directory:`                              |
| Text        | `Diretório padrão de saída dos PDFs:`                 | `Default output directory for PDFs:`                      |
| Select      | `Compilador LaTeX:`                                   | `LaTeX engine:`                                           |
| Confirm     | `Sobrescrever config em {path}?`                      | `Overwrite config at {path}?`                             |
| Confirm     | `Criar diretório {path}?`                             | `Create directory {path}?`                                |
| Confirm     | `Sobrescrever template '{name}'?`                     | `Overwrite template '{name}'?`                            |
| Confirm     | `Remover template '{name}'?`                          | `Remove template '{name}'?`                               |
| Confirm     | `Sobrescrever {path}?`                                | `Overwrite {path}?`                                       |
| Text        | `Caminho do .tex a compilar:`                         | `Path to the .tex to compile:`                            |
| Confirm     | `Manter cópia do .tex no output_dir?`                 | `Keep a copy of the .tex in output_dir?`                  |
| Confirm     | `Manter cópia do .log no output_dir?`                 | `Keep a copy of the .log in output_dir?`                  |
| Select      | `O que fazer com os templates?`                       | `What would you like to do with templates?`               |
| Select      | `Nome do template:`                                   | `Template name:`                                          |
| Text        | `Caminho do arquivo .tex:`                            | `Path to the .tex file:`                                  |
| Text        | `Caminho do arquivo JSON (- para stdin):`             | `Path to the JSON file (- for stdin):`                    |
| Confirm     | `Modo dry-run (imprime em stdout, não grava)?`        | `Dry-run mode (prints to stdout, does not write)?`        |

---

## 5. Stdout success messages

### `build` (in `src/cli.rs`, `handle_build`)

- **Before**: `PDF gerado em {path}. Pipeline (render + compile) levou {duration:.1}s.`
- **After**: `PDF generated at {path}. Pipeline (render + compile) took {duration:.1}s.`

### `compile` (in `src/cli.rs`, `handle_compile`)

- **Before**: `PDF gerado em {path}. Compilação levou {duration:.1}s.`
- **After**: `PDF generated at {path}. Compile took {duration:.1}s.`

### `render` (in `src/cli.rs`, `handle_render`)

- **Before**: `.tex gerado em {path}.` (or similar; verified during implementation)
- **After**: `.tex written to {path}.`

---

## 6. Stderr diagnostics (`src/compiler.rs`)

### Post-compile no-PDF fallback (line ~167)

- **Before**: `Engine retornou sucesso mas nenhum PDF foi produzido.`
- **After**: `Engine returned success but no PDF was produced.`

---

## 7. What is **not** changing

- **Exit codes**: every one of `1, 2, 10, 11, 12, 13, 14, 15, 20, 21,
  22, 30, 31, 40, 41, 42` maps to the same `TexError` variant as
  before.
- **Structural positions**: paths, durations, engine names, template
  names, config keys — all appear in the same positions.
- **Config schema**: `~/.config/tex/config.toml` untouched.
- **JSON payload schema**: user data structure untouched.
- **Template syntax**: Tera syntax and autoescape=false unchanged.
- **File permissions**: `0644` on written PDFs/`.tex`/`templates` and
  `0600` on config, unchanged.
- **Atomic-write behavior**: unchanged.
- **Docker image**: unchanged.
- **Dependencies**: unchanged.

---

## Verification points

Every entry in this contract must be exercised by at least one test
after implementation. See [`quickstart.md`](../quickstart.md) for
the running-order + [`spec.md` § Success Criteria](../spec.md) for
the acceptance metric.
