# Implementation Plan: Compilação `.tex` → PDF (Compilador Plugável)

**Branch**: `004-compile-tectonic` | **Date**: 2026-07-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/004-compile-tectonic/spec.md`

## Summary

Entregar o subcomando `tex-cli compile <tex-file> [--output <path>]
[--engine <name>] [--keep-tex] [--keep-logs] [--force]` mais modo
interativo. **Fecha o pipeline JSON → PDF**: junto com spec 003
(render), o usuário pode agora ir de dados JSON estruturados a PDF
final sem sair da CLI.

Materializa o Princípio IV da constitution (compilador plugável)
via módulo dedicado `src/compiler.rs` que abstrai a invocação de
engines LaTeX externos. Enum fechado `SupportedEngine` com dispatch
`engine.binary_name()` + `engine.args_for(&tex_filename)` isola
todo hardcode do tectonic num único ponto de troca.

Abordagem técnica: `TempDir` (via `tempfile`) contém a compilação
inteira; o `.tex` fonte é copiado, o engine roda com cwd=temp, PDF
gerado é escrito atomicamente no destino via
`atomic::write_atomic(&pdf_path, &bytes, 0o644)` (helper compartilhado
da spec 003). `.tex` e `.log` são copiados pro `output_dir` só se
flags/config pedirem. TempDir é dropado no final, garantindo zero
artefato residual (SC-006).

Três novas variantes de `TexError` são introduzidas:
`CompileFailed { engine, tex_path, log_tail }` → exit `40`,
`EngineNotInstalled { engine }` → exit `41`,
`EngineNotSupported { engine, accepted }` → exit `42`.

Zero mudança em `Cargo.toml` — `std::process::Command`, `tempfile`,
`which` já cobrem todas as necessidades.

## Technical Context

**Language/Version**: Rust stable (edição 2021) — mesmo alvo das
specs anteriores.

**Primary Dependencies** (todas já ativas em `Cargo.toml`, zero
adições):

- `std::process::Command` — invocação do engine child process.
  Captura stdout/stderr como bytes; controle explícito de cwd,
  stdin, env se necessário.
- `tempfile` (já ativo) — `TempDir::new_in(...)?` cria dir
  temporário no mesmo filesystem do destino, dropado no scope
  end (garante SC-006).
- `which` (já ativo) — validação `which::which(engine_binary)`
  antes de spawn.
- `std::time::Instant` — cronometrar compilação (contrato de
  stdout final).
- `atomic::write_atomic` (spec 003) — grava o PDF final com mode
  0o644 atomicamente.
- `clap` v4 — novo subcomando `compile` com args e flags (incluindo
  `--keep-tex` / `--no-keep-tex` como `Option<bool>` via
  `#[arg(long)]` par de flags).
- `inquire` — modo interativo (Text pro path, Confirm pra
  keep_tex/keep_logs).
- `tracing` — spans `compile_start`, `engine_spawn`, `compile_done`
  com `#[instrument]`; log level derivado do `-v` já resolvido.
- `anyhow`, `thiserror` — fronteira de erro + variantes 40/41/42.

**Storage**: filesystem apenas.
- Input: `.tex` em path arbitrário.
- Temp: `TempDir` em `/tmp` (ou `$TMPDIR`); PDF gerado dentro dele
  como `<basename>.pdf`.
- Output: `<config.paths.output_dir>/<basename>.pdf` (default) ou
  path custom via `--output`. Opcionalmente `.tex` e `.log`
  copiados também.

**Assets embutidos**: nenhum novo.

**Testing**:

- **Unit tests** (`cargo test --lib`) no novo `src/compiler.rs`:
  - `supported_engine_from_str_accepts_canonical` — parse case-sensitive.
  - `supported_engine_from_str_rejects_unknown` — retorna
    `EngineNotSupported`.
  - `supported_engine_binary_name` — mapping enum → string.
  - `supported_engine_args_for_tectonic` — args esperados.
  - `supported_engine_args_for_latexmk` — args esperados.
  - `supported_engine_args_for_pdflatex_variants` — pdflatex/xelatex/lualatex.
  - `tail_lines` helper — retorna últimas N linhas de string
    multi-linha; borda: menos linhas que N.
  - `compile_request_resolves_default_output_path` — junta
    output_dir + basename + `.pdf`.
- **Integration tests** (`tests/`):
  - `tests/cli_compile.rs`:
    - `compile_produces_pdf_with_default_output_path` (requer
      tectonic — roda só no container).
    - `compile_with_output_flag_uses_custom_path`.
    - `compile_pdf_has_pdf_magic_bytes`.
    - `compile_stdout_mentions_path_and_duration`.
    - `compile_broken_tex_exits_40_with_log_tail`.
    - `compile_missing_tex_exits_1`.
    - `compile_missing_config_exits_10`.
    - `compile_engine_not_installed_exits_41` — usa `PATH=/tmp/empty`.
    - `compile_engine_not_supported_exits_42` — `--engine foo`.
    - `compile_engine_flag_overrides_config` — verifica via
      `config show --format json` que engine no config **não** mudou.
    - `compile_keep_tex_copies_source_to_output_dir`.
    - `compile_keep_logs_copies_log_to_output_dir`.
    - `compile_no_keep_tex_flags_leave_only_pdf`.
    - `compile_overwrite_without_force_non_tty_exits_15`.
    - `compile_force_overwrites_existing_pdf`.
    - `compile_leaves_no_artefacts_in_tmp` — snapshot `/tmp`
      antes/depois.
  - `tests/cli_banner.rs` **existente**: estender com
    `banner_appears_on_compile_stderr`.
- **Docker workflow**: container `tex-cli` já traz tectonic
  instalado (spec 001). Zero mudança de Dockerfile.

**Target Platform**: Linux x86_64 (mesmo das specs anteriores). Não
introduzimos nova dep de OS.

**Project Type**: CLI — extensão do binário `tex-cli`.

**Performance Goals**:

- SC-001: `compile` template simples (< 5 páginas) em < 15 s
  wall-clock via tectonic no container. Tectonic single-pass em
  documento comum tipicamente < 2 s + cold-start de fonts.
- Ciclo `cargo test --all` incremental permanece < 15 s (novo
  binário `cli_compile` mais alguns testes de integração que
  invocam tectonic — cada teste real leva ~1-2 s).
- Testes de integração que dependem de tectonic são marcados com
  `#[cfg(feature = "requires_tectonic")]` OU verificados via
  `which::which("tectonic")` no início e usam `println!("...
  skipped: tectonic not available") + return` fora do container.

**Constraints**:

- Escrita do PDF MUST ser atômica via `atomic::write_atomic` — mesmo
  padrão de render/templates.
- Nenhum artefato do TempDir MUST permanecer após comando
  (garantido por `TempDir` Drop, validado por teste).
- `--engine` MUST NOT alterar o config file.
- Banner sempre em stderr.
- Nenhuma nova dep fora da stack canônica.

**Scale/Scope**: single-user, single-process. Uma compilação = um
`.tex` → um PDF. Sem batch, sem paralelismo.

## Constitution Check

Gates avaliados contra
[`.specify/memory/constitution.md`](../../.specify/memory/constitution.md)
v1.0.0.

| Princípio | Gate | Status | Justificativa |
|-----------|------|--------|---------------|
| I. Escopo pessoal e não-substituição do LaTeX | Feature ajuda usuário individual a gerar documentos LaTeX mais rápido? Zero reimplementação de LaTeX? | ✅ PASS | Compila via engine externa — zero parsing/interpretação de LaTeX. |
| II. Separação Dados/Template/PDF | Feature respeita e materializa a separação? | ✅ PASS | Produz PDF a partir de `.tex`; não toca JSON nem templates. Consome apenas a camada `.tex` intermediária. |
| III. Interface Dupla CLI + Interativo | Cada op é acessível pelo CLI direto e pelo menu interativo? | ✅ PASS | `compile <tex>` scriptável com exit codes 40/41/42 específicos; `compile` sem args abre menu inquire; degrada gracefully em não-TTY. |
| IV. Compilador Plugável | Feature respeita/materializa o isolamento do compilador em `compiler.rs`? | ✅ PASS | **Esta é a spec que materializa o Princípio IV.** Enum `SupportedEngine` fechado, `binary_name()` + `args_for()`, todo hardcode do tectonic isolado no módulo. Trocar engine é adicionar variante ao enum. |
| V. Segurança na Manipulação de Arquivos | Escrita atômica, confirmação de overwrite, validação, mensagens humanas? | ✅ PASS | PDF via `atomic::write_atomic` (spec 003); `--force` ou TTY-confirm pra overwrite; mensagens de erro localizadas com tail do log; TempDir garante zero resíduo. |

**Stack fixada** (Restrições Técnicas da constitution): zero nova
dep. `tempfile`, `which`, `clap`, `inquire`, `tracing`, `anyhow`,
`thiserror` — tudo já ativo. `std::process::Command` é stdlib.
Zero violações restantes; Complexity Tracking permanece vazio.

## Project Structure

### Documentation (this feature)

```text
specs/004-compile-tectonic/
├── plan.md              # este arquivo
├── spec.md              # spec (checklist 16/16 PASS)
├── research.md          # Phase 0 output (decisões técnicas D-01..)
├── data-model.md        # Phase 1 output — SupportedEngine, CompileOutcome, TexError ext
├── quickstart.md        # Phase 1 output — como rodar/testar
├── contracts/
│   └── cli.md           # contrato observável do `compile`
├── checklists/
│   └── requirements.md  # 16/16 PASS
└── tasks.md             # criado depois por /speckit-tasks
```

### Source Code (repository root)

Esta feature adiciona **um único módulo novo** (`src/compiler.rs`)
e estende `src/cli.rs`, `src/errors.rs`, `src/interactive.rs`,
`src/main.rs`, `src/lib.rs` sem tocar em `config.rs`,
`templates.rs`, `render.rs`, `atomic.rs`, `paths.rs`.

```text
src/
├── main.rs              # + dispatch pra Commands::Compile(args)
├── cli.rs               # + Commands::Compile, CompileArgs,
│                        #   handle_compile + handle_compile_menu +
│                        #   compile_and_print (single choke point)
├── config.rs            # inalterado (Config::load reaproveitado)
├── errors.rs            # + CompileFailed (40), EngineNotInstalled (41),
│                        #   EngineNotSupported (42)
├── interactive.rs       # + prompt_tex_source(),
│                        #   confirm_keep_tex(), confirm_keep_logs()
├── paths.rs             # inalterado
├── atomic.rs            # inalterado (reutilizado)
├── render.rs            # inalterado
├── templates.rs         # inalterado
├── lib.rs               # + pub mod compiler;
└── compiler.rs          # NOVO — SupportedEngine enum, resolve_engine,
                         #   validate_binary, run_engine, tail_lines,
                         #   compile_and_write, CompileOutcome

Cargo.toml               # inalterado — zero nova dep

tests/
├── cli_banner.rs        # + banner_appears_on_compile_stderr
├── cli_compile.rs       # NOVO — ~16 testes de integração
└── (demais tests/ inalterados)
```

**Structure Decision**: Single project layout (mesmo das três specs
anteriores). Módulo `compiler.rs` fica no crate binário/lib existente
seguindo o padrão da estrutura interna esperada declarada pela
constitution. Testes de integração via `assert_cmd` + `assert_fs`,
mesmo pattern.

## Complexity Tracking

Sem violações da constitution. Zero nova dep. Deixa em branco por
transparência.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| —         | —          | —                                    |
