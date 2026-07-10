# Implementation Plan: Pipeline JSON → PDF (`build`)

**Branch**: `005-build-pipeline` | **Date**: 2026-07-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/005-build-pipeline/spec.md`

## Summary

Entregar `tex-cli build <template> <data-source> [--output <path>]
[--engine <name>] [--keep-tex] [--keep-logs] [--force]` mais modo
interativo. **Fecha o pipeline JSON → PDF end-to-end** orquestrando
render (spec 003) + compile (spec 004) em uma única invocação.

Abordagem técnica: novo módulo `src/build.rs` **puramente
orquestrador** — não introduz lógica de render/compile, só combina
os helpers existentes. Contrato:

1. `read_template` → bytes.
2. `load_json_source` → `serde_json::Value` (com validação).
3. `render_template` → `String` (com validação tera).
4. Se `keep_tex=true`: `atomic::write_atomic(<output_dir>/<name>.tex,
   rendered, 0o644)` **antes** do compile (FR-015 — permite debug
   se compile falhar).
5. Escreve `rendered` em `<temp>/<name>.tex` dentro de um TempDir.
6. `compile_and_write(cfg, <temp>/<name>.tex, engine, output_pdf,
   keep_tex, keep_logs, force, verbose)`.
7. Retorna `BuildOutcome` com durations de cada etapa.

Zero nova dep, zero nova variante de `TexError`. Toda a lógica de
erro já existe nas specs 001-004 e é propagada com `?`.

## Technical Context

**Language/Version**: Rust stable (edição 2021) — mesmo alvo das
specs anteriores.

**Primary Dependencies** (todas já ativas em `Cargo.toml`, **zero
adições**):

- `serde_json` — via `render::load_json_source` (já feito).
- `tera` — via `render::render_template` (já feito).
- `tempfile` — via `TempDir` compartilhado com o compile.
- `clap` v4 — novo subcomando `build`.
- `inquire` — modo interativo (reusa `prompt_template_name`,
  `prompt_json_source`, `confirm_keep_tex`, `confirm_keep_logs`).
- `tracing` — spans `build_start`, `render_phase`, `compile_phase`,
  `build_done` com `#[instrument]`.
- `std::time::Instant` — cronometrar cada etapa.

**Storage**: nenhum novo padrão.
- Input: template em `paths.templates_dir/<name>.tex`, JSON em path
  arbitrário ou stdin.
- Intermediate: `.tex` renderizado em TempDir OU
  `<output_dir>/<name>.tex` se `keep_tex=true`.
- Output: `<output_dir>/<name>.pdf` (default) ou path custom via
  `--output`. Opcionalmente `.log` copiado.

**Assets embutidos**: nenhum novo.

**Testing**:

- **Unit tests** (`cargo test --lib`) no novo `src/build.rs`:
  - `resolve_output_pdf_default` — path default = output_dir/<name>.pdf.
  - `resolve_output_pdf_custom` — flag override.
  - `resolve_intermediate_tex_path` — helper resolve o path do .tex
    intermediário quando keep_tex=true.
- **Integration tests** (`tests/`):
  - `tests/cli_build.rs`:
    - `build_produces_pdf_with_default_output_path` (MVP).
    - `build_pdf_has_pdf_magic_bytes`.
    - `build_stdout_mentions_path_and_total_duration`.
    - `build_with_output_flag_uses_custom_path`.
    - `build_missing_template_exits_20`.
    - `build_json_top_level_array_exits_31`.
    - `build_malformed_json_exits_31`.
    - `build_tera_error_exits_30`.
    - `build_engine_not_supported_exits_42`.
    - `build_engine_flag_overrides_config_and_config_stays_unchanged`.
    - `build_broken_tex_after_render_exits_40_with_log_tail` — JSON
      válido + template que produz `.tex` com `\undefinedcommand`.
    - `build_keep_tex_preserves_intermediate_after_compile_fail` —
      combina compile-fail + `--keep-tex` para provar FR-015.
    - `build_no_keep_tex_leaves_only_pdf`.
    - `build_stdin_json_dash_arg` (US3).
    - `build_missing_config_exits_10`.
    - `build_overwrite_without_force_non_tty_exits_15`.
    - `build_force_overwrites_existing_pdf`.
    - `build_leaves_no_artefacts_in_tmp` — mesmo pattern SC-006.
  - `tests/cli_banner.rs` **existente**: estender com
    `banner_appears_on_build_stderr`.

**Target Platform**: Linux x86_64 (mesmo das specs anteriores).

**Project Type**: CLI — extensão do binário `tex-cli`.

**Performance Goals**:

- SC-001: `build` template simples (<5 páginas) via tectonic **<
  20 s** wall-clock no container. Realista: render ~2 ms + compile
  ~2 s + I/O de write ~10 ms → < 3 s típico; folga cobre cold-start
  de tectonic.
- SC-002: pipeline `build` economiza **> 30%** vs. `render` +
  `compile` separados. Justificativa: um `render` + `compile`
  separados exigem dois processos, dois carregamentos de config,
  dois setups de tracing. `build` faz um só. Estimativa: ~40-50 ms
  economizados (cargo run overhead) por invocação.

**Constraints**:

- Escrita atômica do PDF via `atomic::write_atomic` (helper spec 003).
- `--keep-tex` escreve `.tex` intermediário **antes** de compile
  (FR-015).
- Zero artefato no filesystem em qualquer erro (SC-006).
- `--engine` não altera config (SC-005 herdado).
- Banner em stderr, nunca stdout.
- Zero nova dep fora da stack canônica.

**Scale/Scope**: single-user, single-process. Um `build` = um
template + um JSON = um PDF.

## Constitution Check

Gates avaliados contra
[`.specify/memory/constitution.md`](../../.specify/memory/constitution.md)
v1.0.0.

| Princípio | Gate | Status | Justificativa |
|-----------|------|--------|---------------|
| I. Escopo pessoal e não-substituição do LaTeX | Feature ajuda usuário individual a gerar documentos LaTeX mais rápido? Zero reimplementação de LaTeX? | ✅ PASS | Comando de mais alto nível compõe as peças já implementadas — não reimplementa nada. |
| II. Separação Dados/Template/PDF | Feature respeita a separação? | ✅ PASS | **Este comando é o "combinador" das três camadas** (JSON + Template + PDF) — mas cada camada continua isolada em seu módulo (`render`, `compile`, `templates`). `build.rs` apenas orquestra. |
| III. Interface Dupla CLI + Interativo | Cada op é acessível pelo CLI direto e pelo menu interativo? | ✅ PASS | `build <tex>` scriptável com exit codes específicos; `build` sem args abre menu inquire; degrada gracefully em não-TTY. |
| IV. Compilador Plugável | Feature respeita/preserva o isolamento do compilador? | ✅ PASS | `build.rs` não toca `src/compiler.rs` — só chama `resolve_engine` / `validate_binary` / `compile_and_write` como API pública. Engine plugável 100% preservado. |
| V. Segurança na Manipulação de Arquivos | Escrita atômica, confirmação de overwrite, validação, mensagens humanas? | ✅ PASS | PDF via `atomic::write_atomic` (herdado); `--force` ou TTY-confirm; erros específicos por etapa em pt-BR; TempDir garante zero resíduo; `.tex` intermediário preservado em falha para debug. |

**Stack fixada**: zero nova dep. `serde_json`, `tera`, `tempfile`,
`clap`, `inquire`, `tracing`, `anyhow`, `thiserror` — tudo já ativo.
Zero violações; Complexity Tracking vazio.

## Project Structure

### Documentation (this feature)

```text
specs/005-build-pipeline/
├── plan.md              # este arquivo
├── spec.md              # spec (checklist 16/16 PASS)
├── research.md          # Phase 0 output (decisões técnicas D-01..)
├── data-model.md        # Phase 1 output — BuildOutcome, orquestração
├── quickstart.md        # Phase 1 output — como rodar/testar
├── contracts/
│   └── cli.md           # contrato observável do `build`
├── checklists/
│   └── requirements.md  # 16/16 PASS
└── tasks.md             # criado depois por /speckit-tasks
```

### Source Code (repository root)

Esta feature adiciona **um único módulo novo** (`src/build.rs`) —
puro orquestrador — e estende `src/cli.rs`, `src/interactive.rs` (só
se novo prompt for necessário — provavelmente não), `src/lib.rs`,
`src/main.rs` sem tocar em `config.rs`, `templates.rs`, `render.rs`,
`compiler.rs`, `errors.rs`, `atomic.rs`, `paths.rs`.

**Zero mudança em `TexError`** — todos os erros do `build` são
propagados dos módulos filhos.

```text
src/
├── main.rs              # + dispatch pra Commands::Build(args)
├── cli.rs               # + Commands::Build, BuildArgs,
│                        #   handle_build + handle_build_menu +
│                        #   print_build_outcome (single choke point)
├── config.rs            # inalterado
├── errors.rs            # inalterado (zero nova variante)
├── interactive.rs       # inalterado (reutiliza prompts existentes
│                        #   das specs 003/004)
├── paths.rs             # inalterado
├── atomic.rs            # inalterado (reutilizado)
├── render.rs            # inalterado (funções públicas reutilizadas)
├── compiler.rs          # inalterado (funções públicas reutilizadas)
├── templates.rs         # inalterado (funções públicas reutilizadas)
├── lib.rs               # + pub mod build;
└── build.rs             # NOVO — build_pipeline, BuildOutcome

Cargo.toml               # inalterado — zero nova dep

tests/
├── cli_banner.rs        # + banner_appears_on_build_stderr
├── cli_build.rs         # NOVO — ~19 testes de integração
└── (demais tests/ inalterados)
```

**Structure Decision**: Single project layout (mesmo das quatro
specs anteriores). Módulo `build.rs` fica no crate binário/lib
existente. Testes de integração via `assert_cmd` + `assert_fs`.
Nenhuma reorganização.

## Complexity Tracking

Sem violações da constitution. Zero nova dep, zero nova variante
de erro. Módulo `build.rs` é orquestração pura — o commit de
implementação será dominado por wiring (clap args + handler +
tests), não lógica nova.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| —         | —          | —                                    |
