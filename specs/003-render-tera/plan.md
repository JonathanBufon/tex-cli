# Implementation Plan: Renderização JSON → `.tex` via Tera

**Branch**: `003-render-tera` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-render-tera/spec.md`

## Summary

Entregar um subcomando novo `tex-cli render <template> <data-source>
[--output <path>] [--dry-run] [--force]` mais modo interativo
(`tex-cli render` sem args). Materializa a ponte central do pipeline
JSON → LaTeX declarada no Princípio II da constitution:

```text
JSON (dados) + template `.tex` (spec 002) → `.tex` renderizado (esta spec)
```

Abordagem técnica: novo módulo `src/render.rs` com uma função pura
`render_template(template_src, json_value) -> Result<String, TexError>`
que encapsula a chamada ao tera (`Tera::one_off` com autoescape=false).
O handler `handle_render` em `src/cli.rs` orquestra: `Config::load` →
`resolve_template` → `read_template` (spec 002) → leitura do JSON
(arg path ou stdin) → chamada a `render_template` → escrita atômica
com `0o644` reaproveitando o helper `write_atomic_0644` já em
`src/templates.rs` (extraído se necessário, ou compartilhado).

Duas novas variantes de `TexError` são introduzidas:
`TeraRenderError { template_name, detail }` → exit `30`, e
`InvalidJson { source, detail }` → exit `31`. Estendem o enum sem
quebrar callers existentes.

Zero mudanças em `src/config.rs` ou em módulos que já cobrem specs
001/002 — a feature se pluga no ponto de junção existente.

## Technical Context

**Language/Version**: Rust stable (edição 2021), mesmo alvo das specs
anteriores.

**Primary Dependencies**:

- `tera` (**nova ativação** — já listado na stack canônica da
  constitution): usado via `tera::Tera::one_off(template, context,
  autoescape)` para renderização one-shot sem registro de template
  no engine. Autoescape = false (`.tex` não requer escape HTML).
  `tera::Context::from_value(serde_json::Value)` faz a ponte
  natural com o JSON já parseado.
- `serde_json` (já ativo): parse do JSON de arg ou stdin.
- `clap` v4 (já ativo): novo subcomando `render` com args positionais
  + flags.
- `inquire` (já ativo): modo interativo (Select do template + Text
  do path do JSON + Confirm do dry-run).
- `tempfile` (já ativo): escrita atômica reaproveitando padrão da
  spec 002.
- `anyhow`, `thiserror`, `tracing` (já ativos): fronteira de erro,
  variantes novas de `TexError`, spans `render_start`/`render_done`
  com `#[instrument]`.

**Storage**: sistema de arquivos. Input: template em
`config.paths.templates_dir/<nome>.tex` (leitura via
`read_template` da spec 002) + JSON em path arbitrário ou stdin.
Output: `config.paths.output_dir/<nome>.tex` (default) ou path
customizado via `--output`, ou stdout (com `--dry-run`).

**Assets embutidos**: nenhum novo. `assets/banner.txt` continua o
único asset.

**Testing**:

- **Unit tests** (`cargo test --lib`) no novo `src/render.rs`:
  - `render_template_basic`: template com `{{ nome }}` + JSON
    `{"nome":"X"}` → produz string esperada.
  - `render_template_missing_var_returns_tera_error`: template com
    variável ausente no JSON → `TexError::TeraRenderError`.
  - `render_template_uses_default_filter`: `{{ x | default(value="Y") }}`
    com JSON sem `x` → renderiza "Y".
  - `render_template_nested_access`: `{{ obj.chave }}` funciona.
  - `render_template_no_autoescape_for_latex`: template com `&` /
    `%` no lugar do valor → não é HTML-escapado para `&amp;` /
    `%25`.
- **Integration tests** (`tests/`):
  - `tests/cli_render.rs`:
    - `render_writes_file_with_default_output_path`.
    - `render_with_output_flag_uses_custom_path`.
    - `render_dry_run_prints_to_stdout_and_no_file`.
    - `render_dry_run_and_output_flag_dry_run_wins`.
    - `render_missing_template_exits_20`.
    - `render_missing_json_file_exits_1` (I/O genérico) ou
      `_exits_31` (dependendo de kind).
    - `render_malformed_json_exits_31`.
    - `render_json_top_level_array_exits_31`.
    - `render_tera_error_exits_30_with_location`.
    - `render_stdin_json_dash_arg`.
    - `render_overwrite_without_force_non_tty_exits_15`.
    - `render_force_overwrites_existing`.
    - `render_missing_config_exits_10`.
  - `tests/cli_banner.rs` **existente**: estender com um caso
    `banner_appears_on_render_stderr` + `banner_never_appears_on_stdout_for_render_dry_run`
    (protege o pipe pra `less`/`jq`).
- **Docker workflow**: container `tex-cli` já cobre tudo — zero
  mudança no Dockerfile.

**Target Platform**: Linux x86_64 (mesmo das specs anteriores). Não
introduzimos dependência de OS-specific além do que já foi
justificado.

**Project Type**: CLI — extensão do binário `tex-cli`.

**Performance Goals**:

- SC-001: render de template 5-10 KB com 10 vars < 100 ms
  wall-clock. Realista: tera é in-memory, o dominante é I/O de disco
  que a spec 002 mediu em < 5 ms para arquivos dessa ordem.
- `render --dry-run` para o mesmo template: < 50 ms (sem I/O de
  gravação, só leitura + render + write no pipe stdout).
- Ciclo `cargo test --all` incremental permanece < 3 s (o overhead
  do novo binário de teste `cli_render` é pequeno).

**Constraints**:

- Autoescape do tera MUST ser `false` (`.tex` não é HTML).
- JSON MUST ser objeto top-level; array/literal → erro dedicado.
- Escrita do `.tex` MUST ser atômica (tmpfile + persist) com
  `0644` — mesmo padrão do `templates add` (spec 002).
- Sem `--force` e destino existente + não-TTY → exit 15 (padrão da
  spec 002).
- Banner continua invariante em stderr (herda spec 001, protegido
  por FR-021 quando `--dry-run` pipeia stdout).
- `--dry-run` prevalece sobre `--output` (documentado no spec, edge
  cases).
- Nenhuma nova dep fora da stack canônica sem justificativa em
  research.

**Scale/Scope**: single-user, single-process. Um render = um
template + um JSON = um `.tex`. Sem batch, sem paralelismo, sem
cache.

## Constitution Check

Gates avaliados contra
[`.specify/memory/constitution.md`](../../.specify/memory/constitution.md)
v1.0.0.

| Princípio | Gate | Status | Justificativa |
|-----------|------|--------|---------------|
| I. Escopo pessoal e não-substituição do LaTeX | Feature ajuda usuário individual a gerar documentos LaTeX mais rápido? Zero reimplementação de LaTeX? | ✅ PASS | Render é substituição de placeholders textuais via engine externa (`tera`) — não parseia LaTeX, não compila, não interpreta macros. |
| II. Separação Dados/Template/PDF | Feature respeita e materializa a separação? | ✅ PASS | **Esta é a spec que materializa a separação**: consome JSON + template, produz `.tex` derivado. Zero mistura de responsabilidades. |
| III. Interface Dupla CLI + Interativo | Cada op é acessível pelo CLI direto (scriptável, exit codes) **e** pelo menu interativo? | ✅ PASS | `render` com args diretos é 100% scriptável; `render` sem args abre menu inquire; degrada gracefully em não-TTY (exit 1 com mensagem útil). |
| IV. Compilador Plugável | Feature respeita o isolamento do compilador em `compiler.rs`? | ✅ PASS | Não sabe da existência do compilador. Produz `.tex` — o resto é responsabilidade da spec 004. |
| V. Segurança na Manipulação de Arquivos | Escrita atômica, confirmação de overwrite, validação, mensagens humanas? | ✅ PASS | Reaproveita `write_atomic_0644` da spec 002 (tmpfile + persist); confirmação de overwrite via `--force` ou TTY; JSON validado antes de qualquer I/O de escrita; erros com localização (linha/coluna) em pt-BR. |

**Stack fixada** (Restrições Técnicas): `tera` já está na tabela
canônica da constitution — não é adição, é ativação. Zero
violações restantes. `serde_json`, `clap`, `inquire`, `tempfile`,
`anyhow`, `thiserror`, `tracing` já estão ativos. Complexity
Tracking permanece vazio.

## Project Structure

### Documentation (this feature)

```text
specs/003-render-tera/
├── plan.md              # este arquivo
├── spec.md              # spec (checklist 16/16 PASS)
├── research.md          # Phase 0 output (decisões técnicas D-01..)
├── data-model.md        # Phase 1 output — RenderOutcome, extensão TexError
├── quickstart.md        # Phase 1 output — como rodar/testar
├── contracts/
│   └── cli.md           # contrato observável do `render`
├── checklists/
│   └── requirements.md  # 16/16 PASS
└── tasks.md             # criado depois por /speckit-tasks
```

### Source Code (repository root)

Esta feature adiciona **um único módulo novo** (`src/render.rs`) e
estende `src/cli.rs`, `src/errors.rs`, `src/interactive.rs`,
`src/main.rs` sem tocar em `config.rs`, `templates.rs` (só se
extrairmos `write_atomic_0644` — decisão em research D-03),
`paths.rs`.

```text
src/
├── main.rs              # + dispatch pra Commands::Render(args)
├── cli.rs               # + Commands::Render, RenderArgs,
│                        #   handle_render + handle_render_menu
├── config.rs            # inalterado
├── errors.rs            # + TeraRenderError, InvalidJson (30/31)
├── interactive.rs       # + render_menu(), prompt_json_source(),
│                        #   confirm_dry_run(), reuso de
│                        #   prompt_template_name (spec 002)
├── paths.rs             # inalterado
├── lib.rs               # + pub mod render;
├── render.rs            # NOVO — render_template, load_json (arg/stdin),
│                        #   RenderOutcome
└── templates.rs         # inalterado, ou export do write_atomic_0644
                         #   se D-03 escolher extrair pra src/atomic.rs

Cargo.toml               # + tera = "1" (dependência já na constitution)

tests/
├── cli_banner.rs        # + banner_appears_on_render_stderr,
│                        #   + banner_never_leaks_on_render_dry_run
├── cli_render.rs        # NOVO — 12 testes de integração cobrindo
│                        #   FR-001..FR-023
└── (demais tests/ da spec 001/002 inalterados)
```

**Structure Decision**: Single project layout (mesmo das duas specs
anteriores). O novo módulo `render.rs` fica no crate binário/lib
existente. Testes de integração seguem o padrão `assert_cmd` +
`assert_fs`. Nenhuma reorganização do layout declarado pela
constitution.

## Complexity Tracking

Sem violações da constitution. `tera` é ativação de dep já canônica.
Se D-03 do research decidir extrair `write_atomic_0644` pra um
módulo compartilhado (`src/atomic.rs`), isso será documentado como
**refactor benigno**, não como violação — a lógica compartilhada é
o mesmo pattern que a spec 002 já provou.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| —         | —          | —                                    |
