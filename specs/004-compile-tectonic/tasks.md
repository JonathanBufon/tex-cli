---
description: "Task list for feature 004-compile-tectonic"
---

# Tasks: Compilação `.tex` → PDF (Compilador Plugável)

**Input**: Design documents from `/specs/004-compile-tectonic/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: Included — plan.md e contracts/cli.md declaram contratos observáveis explícitos (banner, PDF magic bytes, exit codes 40/41/42, atomicidade, config-imutabilidade do --engine, cleanup zero-resíduo) que exigem verificação automatizada.

**Organization**: Tasks agrupadas por user story para permitir implementação e teste independentes. MVP = User Story 1 (`compile <tex>` grava PDF no output_dir) — entrega valor sozinho, fecha o pipeline JSON → PDF junto com spec 003.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1..US4)
- File paths são relativos à raiz do repo.

## Path Conventions

- Rust code: `src/`
- Integration tests: `tests/`
- Docker: `docker/` (inalterado desta spec)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Verificar baseline após spec 003 mergeada; nenhuma dep nova ativada.

- [X] T001 Rodar `docker run --rm -v $(pwd):/src -w /src tex-cli cargo check --all` no container Docker sancionado para confirmar que a baseline das specs 001-003 continua compilando limpo, e que zero nova dep será necessária nesta spec. Também rodar `docker run --rm ... tex-cli tectonic --version` (via `sh -c`) para confirmar que o engine default está instalado no container.

**Checkpoint**: `cargo check --all` passa. Tectonic disponível no container.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extensões estruturais que TODAS as user stories dependem — enum de erro, novo módulo, esqueleto de subcomando.

- [X] T002 Estender `src/errors.rs`: adicionar variantes `TexError::CompileFailed { engine: String, tex_path: PathBuf, log_tail: String }`, `TexError::EngineNotInstalled { engine: String }`, `TexError::EngineNotSupported { engine: String, accepted: Vec<&'static str> }` conforme data-model.md e research D-08. Atualizar `TexError::exit_code()` mapeando 40, 41, 42 respectivamente. Estender o `#[cfg(test)] mod tests` para cobrir os três novos códigos + Displays (`CompileFailed` menciona template, engine e log_tail; `EngineNotInstalled` orienta instalação; `EngineNotSupported` lista aceitos usando o helper `format_accepted` existente).
- [X] T003 [P] Criar `src/compiler.rs` (stub vazio com header comment apontando pra T005+ onde os bodies chegam) e adicionar `pub mod compiler;` em `src/lib.rs`.
- [X] T004 Estender `src/cli.rs`: adicionar variante `Commands::Compile(CompileArgs)` ao enum `Commands`. Definir `pub struct CompileArgs { tex_file: Option<PathBuf>, #[arg(short = 'o', long)] output: Option<PathBuf>, #[arg(short = 'e', long)] engine: Option<String>, #[arg(long, conflicts_with = "no_keep_tex")] keep_tex: bool, #[arg(long = "no-keep-tex", conflicts_with = "keep_tex")] no_keep_tex: bool, #[arg(long, conflicts_with = "no_keep_logs")] keep_logs: bool, #[arg(long = "no-keep-logs", conflicts_with = "keep_logs")] no_keep_logs: bool, #[arg(long)] force: bool }`. Handler `handle_compile(args: CompileArgs)` retorna `Err(anyhow!("não implementado"))`. Registrar dispatch em `src/main.rs`: `Commands::Compile(args) => handle_compile(args)`.

**Checkpoint**: `cargo build` compila. `tex-cli compile /tmp/x.tex` retorna "não implementado". Banner em stderr. Flags conflitantes (`--keep-tex --no-keep-tex`) já falham com exit 2 (clap). Todos os 140+ testes das specs anteriores continuam verdes.

---

## Phase 3: User Story 1 - Compilar `.tex` para PDF (Priority: P1) 🎯 MVP

**Goal**: Usuário roda `tex-cli compile /tmp/artigo.tex` e obtém `<output_dir>/artigo.pdf` válido, com escrita atômica mode 0o644.

**Independent Test**: Fixture com `.tex` válido → rodar `compile /tmp/artigo.tex` → validar PDF criado com magic bytes `%PDF-`, mode 0o644, exit 0, stdout menciona path e tempo.

### Tests for User Story 1 (write first, ensure they FAIL before implementation) ⚠️

- [X] T005 [P] [US1] Criar `tests/cli_compile.rs` com testes de integração usando `assert_cmd` + `assert_fs`:
  - `compile_produces_pdf_with_default_output_path`: fixture `.tex` mínimo válido, `HOME` isolado, config com output_dir apontando pra tempdir → PDF criado em `<output_dir>/<basename>.pdf`.
  - `compile_pdf_has_pdf_magic_bytes`: `std::fs::read(&pdf)[0..4] == b"%PDF"`.
  - `compile_pdf_has_mode_0644`: `metadata.permissions().mode() & 0o777 == 0o644`.
  - `compile_stdout_mentions_path_and_duration`: stdout regex `PDF gerado em .*\. Compilação levou \d+(\.\d+)?s\.`.
  - `compile_missing_tex_exits_1`: path inexistente → exit 1, stderr menciona path.
  - `compile_missing_config_exits_10`.
  - `compile_broken_tex_exits_40_with_log_tail`: `.tex` com `\undefinedcommand{oops}` sem `\end{document}` → exit 40, stderr contém "Falha ao compilar" + trecho do log.
  - `compile_overwrite_without_force_non_tty_exits_15`: PDF já existe, sem `--force`, sem TTY → exit 15, PDF original intacto (comparar bytes).
  - `compile_force_overwrites_existing_pdf`: `--force` → sobrescreve, stdout menciona "sobrescrito".
  - `compile_leaves_no_artefacts_in_tmp`: snapshot de `/tmp` antes/depois, filtra arquivos com prefixo `.tmp*` gerados por `TempDir` — nenhum deve permanecer.
- [X] T006 [P] [US1] Unit tests inline em `src/compiler.rs` (`#[cfg(test)] mod tests`) cobrindo:
  - `supported_engine_from_str_accepts_canonical`: parse case-sensitive de todas as 5 variantes.
  - `supported_engine_from_str_rejects_unknown`: "foo" → `EngineNotSupported`, `"Tectonic"` (case wrong) → rejeitado.
  - `supported_engine_as_str_matches_binary_name`: `Tectonic.as_str() == Tectonic.binary_name() == "tectonic"` etc.
  - `supported_engine_args_for_tectonic`: `args_for("artigo.tex")` inclui `"--outdir=."`, `"--keep-logs"`, `"--keep-intermediates"`, `"artigo.tex"`.
  - `supported_engine_args_for_latexmk`: inclui `"-pdf"`, `"-interaction=nonstopmode"`, `"-halt-on-error"`.
  - `supported_engine_args_for_pdflatex_variants`: pdflatex/xelatex/lualatex retornam pattern `-interaction=nonstopmode -halt-on-error <tex>`.
  - `tail_lines_returns_last_n_lines`: string com 10 linhas + `tail_lines(s, 3)` retorna as 3 últimas.
  - `tail_lines_handles_fewer_than_n_lines`: string com 2 linhas + `tail_lines(s, 5)` retorna todas as 2.
  - `tail_lines_empty_string_returns_empty`.
  - `all_has_five_entries`: `SupportedEngine::ALL.len() == 5`.

### Implementation for User Story 1

- [X] T007 [P] [US1] Implementar `pub enum SupportedEngine` + `impl` (ALL, as_str, binary_name, args_for) em `src/compiler.rs` conforme data-model.md.
- [X] T008 [P] [US1] Implementar `impl FromStr for SupportedEngine` com match exato + `TexError::EngineNotSupported { engine, accepted: SupportedEngine::ALL.iter().map(|e| e.as_str()).collect() }` em falha. Adicionar `impl Display` delegando a `as_str()`.
- [X] T009 [P] [US1] Implementar `pub fn tail_lines(s: &str, n: usize) -> String` em `src/compiler.rs`: split por `\n`, `.rev().take(n).rev()`, `join("\n")`. Handles empty string. Cobrir por unit tests T006.
- [ ] T010 [US1] Implementar `pub struct CompileOutcome { pdf_path: PathBuf, duration: Duration, bytes_written: u64, overwrote_existing: bool, kept_tex: bool, kept_logs: bool }` em `src/compiler.rs` conforme data-model.md.
- [X] T011 [US1] Implementar `pub fn resolve_engine(cli_flag: Option<&str>, config_engine: &str) -> Result<SupportedEngine, TexError>` em `src/compiler.rs`: se `cli_flag.is_some()` parseia via `FromStr`; senão parseia `config_engine`. Ambas as falhas retornam `EngineNotSupported`. Depende de T008.
- [X] T012 [US1] Implementar `pub fn validate_binary(engine: SupportedEngine) -> Result<PathBuf, TexError>` em `src/compiler.rs`: `which::which(engine.binary_name()).map_err(|_| TexError::EngineNotInstalled { engine: engine.to_string() })`. Retorna path do binário.
- [X] T013 [US1] Implementar `pub fn run_engine(engine: SupportedEngine, cwd: &Path, tex_filename: &str, verbose: u8) -> Result<(), (i32, String)>` em `src/compiler.rs`: (1) constrói `Command::new(engine.binary_name()).current_dir(cwd).args(engine.args_for(tex_filename))`; (2) se `verbose >= 2`: `Stdio::inherit()` pra stdout/stderr; senão captura via `.output()`; (3) verifica `status.success()`; (4) verifica `<cwd>/<basename>.pdf` existe (D-10); (5) em erro, lê `<cwd>/<basename>.log` (últimas 30 linhas via `tail_lines`) ou fallback pro stderr; retorna `Err((exit_code, log_tail))`. Handler mapeia isso pra `CompileFailed`. Depende de T007, T009, T012.
- [X] T014 [US1] Implementar `pub fn compile_and_write(cfg: &Config, tex_path: &Path, engine: SupportedEngine, output_pdf: &Path, keep_tex: bool, keep_logs: bool, force: bool, verbose: u8) -> Result<CompileOutcome, TexError>` em `src/compiler.rs` conforme data-model.md fluxo. Usa `atomic::write_atomic(&output_pdf, &pdf_bytes, 0o644)` para gravar o PDF, e para copiar `.tex`/`.log` quando keep flags = true. `TempDir::new()?` no início; cai fora de scope no final. Depende de T007..T013.
- [X] T015 [US1] Implementar `handle_compile(args: CompileArgs)` em `src/cli.rs`:
  1. Load config (exit 10/11).
  2. Se `args.tex_file.is_none()` → delega para `handle_compile_menu(&cfg)` (implementado em Phase 6 US4; por enquanto retorna `anyhow!("modo interativo será implementado na US4")`).
  3. Expandir `args.tex_file` via `paths::expand_user_path` se relativo; verificar existência → exit 1 se não.
  4. Resolver `engine` via `compiler::resolve_engine(args.engine.as_deref(), &cfg.compiler.engine)`.
  5. Resolver `output_pdf`: `args.output` presente → `expand_user_path`; ausente → `cfg.paths.output_dir.join(format!("{basename}.pdf"))`.
  6. Resolver `keep_tex`: `args.keep_tex` → true; `args.no_keep_tex` → false; nenhum → `cfg.compiler.keep_tex`. Idem `keep_logs`.
  7. Overwrite handling: se `output_pdf.exists() && !args.force`: TTY → `confirm_compile_overwrite(&output_pdf)?`; se não/não-TTY → `TexError::UserAborted` (exit 15).
  8. Chamar `compile_and_write(...)?`.
  9. Compor stdout:
     - `overwrote_existing = true` → `PDF gerado (sobrescrito) em {path}. Compilação levou {sec:.1}s.`
     - senão → `PDF gerado em {path}. Compilação levou {sec:.1}s.`
  10. Depende de T014.
- [X] T016 [US1] Estender `src/interactive.rs`: `pub fn confirm_compile_overwrite(path: &Path) -> Result<bool, TexError>` usando `inquire::Confirm::new("Sobrescrever <path>?").with_default(false).prompt().map_err(map_inquire_err)`.
- [X] T017 [US1] Rodar `tests/cli_compile.rs` no container. Todos os 10 testes de US1 devem passar. Ajustar mensagens até casarem com predicates. Rodar `cargo test --lib compiler` para confirmar unit tests inline verdes.

**Checkpoint**: MVP funcional. `tex-cli compile <tex>` produz PDF válido no output_dir. Container: `cargo test --test cli_compile` verde.

---

## Phase 4: User Story 2 - Sobrescrever engine só nesta invocação (Priority: P2)

**Goal**: `tex-cli compile <tex> --engine <name>` usa `<name>` só nesta invocação sem alterar o config.

**Independent Test**: Config com engine=tectonic → rodar `compile <tex> --engine latexmk` (assumindo latexmk disponível OU testar via `PATH=/tmp/empty` para exit 41). Verificar config imutável via `config show --format json`.

### Tests for User Story 2 ⚠️

- [ ] T018 [P] [US2] Estender `tests/cli_compile.rs`:
  - `compile_engine_flag_overrides_config_and_config_stays_unchanged`: config tectonic → rodar `compile <tex> --engine tectonic` (mesmo, testável sem outros engines) → sucesso. Ler config novamente e verificar `.compiler.engine == "tectonic"`. SC-005.
  - `compile_engine_not_supported_exits_42`: `--engine foo` → exit 42, stderr `Engine 'foo' não é suportado` + lista dos 5 aceitos.
  - `compile_engine_not_installed_exits_41`: `--engine tectonic` mas `PATH=/tmp/empty` → exit 41, stderr `Engine 'tectonic' não está instalado no PATH`.
  - `compile_engine_case_sensitive`: `--engine Tectonic` (case wrong) → exit 42 (não normalizamos).

### Implementation for User Story 2

- [ ] T019 [US2] `resolve_engine` + `validate_binary` já cobrem este caso via T011/T012. **Nenhuma nova implementação necessária.** Task é apenas validar via `tests/cli_compile.rs` que os 4 novos testes passam. Se algum falhar, ajustar mensagem em `EngineNotSupported`/`EngineNotInstalled` para casar com predicates.

**Checkpoint**: `--engine` funcional; config permanece imutável (SC-005).

---

## Phase 5: User Story 3 - Manter `.tex` e `.log` no output (Priority: P2)

**Goal**: `--keep-tex` / `--no-keep-tex` / `--keep-logs` / `--no-keep-logs` sobrescrevem config só nesta invocação. Flags conflitantes → exit 2.

**Independent Test**: Rodar `compile <tex> --keep-tex --keep-logs` → todos os três (`x.pdf`, `x.tex`, `x.log`) existem no output_dir. Rodar com `--no-keep-tex --no-keep-logs` → só `.pdf` fica.

### Tests for User Story 3 ⚠️

- [ ] T020 [P] [US3] Estender `tests/cli_compile.rs`:
  - `compile_keep_tex_copies_source_to_output_dir`: `--keep-tex` → `<output_dir>/<basename>.tex` existe com bytes iguais ao fonte.
  - `compile_keep_logs_copies_log_to_output_dir`: `--keep-logs` → `<output_dir>/<basename>.log` existe (não-vazio).
  - `compile_no_keep_tex_flags_leave_only_pdf`: `--no-keep-tex --no-keep-logs` (mesmo com config tendo keep_tex=true) → só `<basename>.pdf` no output; nenhum `.tex` ou `.log` do compile.
  - `compile_conflicting_keep_tex_flags_exit_2`: `--keep-tex --no-keep-tex` → exit 2 (clap enforce).
  - `compile_conflicting_keep_logs_flags_exit_2`.
  - `compile_flags_override_config_defaults`: config com keep_tex=true → `compile --no-keep-tex` sem manter .tex; config com keep_tex=false → `compile --keep-tex` mantém.

### Implementation for User Story 3

- [ ] T021 [US3] A lógica de resolver `keep_tex`/`keep_logs` já foi implementada em T015 e `compile_and_write` (T014) já grava os arquivos condicionalmente. Task valida via testes de T020. Se algum falhar, ajustar o handler.

**Checkpoint**: Flags de artefatos funcionam corretamente com precedência flag > config.

---

## Phase 6: User Story 4 - Modo interativo `compile` (Priority: P3)

**Goal**: `tex-cli compile` sem args abre menu (Text pro `.tex` path + 2 Confirm de keep flags).

**Independent Test**: TTY: menu completo. Sem TTY: exit 1.

### Implementation for User Story 4

- [ ] T022 [US4] Estender `src/interactive.rs`: `pub fn prompt_tex_source() -> Result<PathBuf, TexError>` — `inquire::Text::new("Caminho do .tex a compilar:").prompt()` mapeado ao TexError::UserAborted em cancel. Reusa mesmo pattern do `prompt_json_source` da spec 003.
- [ ] T023 [US4] Estender `src/interactive.rs`: `pub fn confirm_keep_tex(default: bool) -> Result<bool, TexError>` — `inquire::Confirm::new("Manter cópia do .tex no output_dir?").with_default(default)`. Idem `pub fn confirm_keep_logs(default: bool)`. Recebem default do config pra prompt useful.
- [ ] T024 [US4] Implementar `handle_compile_menu(cfg: &Config)` em `src/cli.rs`:
  1. `if !stdin().is_terminal()` → `anyhow!("Menu interativo de compile requer terminal. Use tex-cli compile <tex-file>.")` (exit 1).
  2. `let tex_path = prompt_tex_source()?;`
  3. `let keep_tex = confirm_keep_tex(cfg.compiler.keep_tex)?;`
  4. `let keep_logs = confirm_keep_logs(cfg.compiler.keep_logs)?;`
  5. Constrói `CompileArgs { tex_file: Some(tex_path), output: None, engine: None, keep_tex, no_keep_tex: !keep_tex, keep_logs, no_keep_logs: !keep_logs, force: false }` e chama `handle_compile(args)`.
- [ ] T025 [US4] Atualizar `handle_compile` (T015) para delegar realmente a `handle_compile_menu(&cfg)` quando `args.tex_file.is_none()` (remover o placeholder do T015).
- [ ] T026 [US4] Smoke test manual do modo interativo (documentado no quickstart § 4 "Modo interativo", não automatizado).

**Checkpoint**: Menu funcional em TTY, degrada gracefully.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Verificações transversais, higiene, docs, validação de performance.

- [ ] T027 [P] Estender `tests/cli_banner.rs` (existente): `banner_appears_on_compile_stderr` — invoca `compile` com `.tex` válido, valida marker em stderr.
- [ ] T028 [P] Rodar `cargo fmt --all -- --check` e `cargo clippy --all-targets --all-features -- -D warnings` no container. Corrigir violações.
- [ ] T029 [P] Estender `README.md` da raiz: seção "Compilar `.tex` → PDF" com exemplos dos fluxos principais (default, --output, --engine override, --keep-*, --force, menu interativo). Atualizar tabela de exit codes com 40/41/42. Atualizar roadmap indicando spec 005 (build end-to-end) como próxima.
- [ ] T030 Rodar o quickstart.md do início ao fim dentro do container Docker: build → init → templates add → render → compile → verificar PDF válido. Documentar tempo total e outcomes no PR.
- [ ] T031 Validar SC-001 (`compile` template simples via tectonic < 15 s wall-clock) usando o binário release. Registrar 3-5 medições no PR ou em nota no quickstart § 7.
- [ ] T032 Validar SC-005 (config imutável após --engine) por teste automatizado em `tests/cli_compile.rs::compile_engine_flag_overrides_config_and_config_stays_unchanged` (T018 já cobre — task é confirmar green e mencionar SC-005 no commit).
- [ ] T033 Validar SC-006 (zero artefato do TempDir) por teste `compile_leaves_no_artefacts_in_tmp` (T005 já cobre — confirmar green).

**Checkpoint**: Feature completa, testada, documentada, performance validada.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências.
- **Foundational (Phase 2)**: depende de Setup. **BLOQUEIA todas as user stories.**
- **User Story 1 (P1 / Phase 3)**: depende de Foundational.
- **User Story 2 (P2 / Phase 4)**: depende de US1 (usa `resolve_engine` já implementado).
- **User Story 3 (P2 / Phase 5)**: depende de US1 (usa `compile_and_write` já implementado com flags).
- **User Story 4 (P3 / Phase 6)**: depende de US1 e reutiliza handler.
- **Polish (Phase 7)**: depende de todas as US completadas na versão MVP alvo.

### Within Each User Story

- Tests **antes** da implementação (falham → verde após).
- Funções puras (`SupportedEngine`, `tail_lines`) antes de funções com side-effect (`run_engine`, `compile_and_write`).
- Handler CLI é sempre a **última** camada.

### Parallel Opportunities

- Foundational: T003 é [P] (arquivo novo); T002 toca errors.rs, T004 toca cli.rs — independentes.
- US1: testes de integração (T005) e unit tests inline (T006) são [P] com implementação inicial. T007, T008, T009 são [P] entre si (funções livres). T011, T012 são [P].
- US2 tests (T018), US3 tests (T020), Polish (T027, T028, T029) todos [P].

---

## Parallel Example: User Story 1

```bash
# Depois de completar Foundational (T002-T004):
# Rodar em paralelo os testes que serão VALIDADOS depois:
Task: "T005 [P] [US1] Criar tests/cli_compile.rs"
Task: "T006 [P] [US1] Unit tests inline em src/compiler.rs"

# Estruturas e funções puras podem ir em paralelo:
Task: "T007 [P] [US1] SupportedEngine enum + methods"
Task: "T008 [P] [US1] FromStr for SupportedEngine"
Task: "T009 [P] [US1] tail_lines helper"

# Sequenciais (dependências claras):
Task: "T010 [US1] CompileOutcome struct"
Task: "T011 [US1] resolve_engine"
Task: "T012 [US1] validate_binary (which::which)"
Task: "T013 [US1] run_engine (Command::spawn)"
Task: "T014 [US1] compile_and_write (glue com TempDir)"
Task: "T015 [US1] handle_compile em src/cli.rs"
Task: "T016 [US1] confirm_compile_overwrite em interactive.rs"
Task: "T017 [US1] Rodar tests/cli_compile.rs até verde"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup (Phase 1) → Foundational (Phase 2) → US1 (Phase 3) → parar.
2. **STOP & VALIDATE**: `docker run … cargo test --test cli_compile` verde. `.tex` real produz PDF válido.
3. Deploy/demo se ready. Merge se ok.

### Incremental Delivery

1. Foundation → merge.
2. US1 → merge → **MVP** (compile básico).
3. US2 → merge → engine override.
4. US3 → merge → keep flags.
5. US4 → merge → menu interativo.
6. Polish (Phase 7) → merge.

Para consistência com specs 001–003, esta feature pode ser
entregue em **PR único com commits granulares por T-task**.

### Parallel Team Strategy

Projeto single-dev — não aplicável.

---

## Notes

- Todas as tarefas devem terminar com o teste correspondente verde antes de avançar.
- Docker container `tex-cli` (com tectonic instalado) reaproveitado sem mudança.
- Nenhuma tarefa toca `src/config.rs`, `src/render.rs`, `src/templates.rs`, `src/atomic.rs`, `src/paths.rs` — todos imutáveis nesta spec.
- `atomic::write_atomic` reutilizado para gravar PDF, `.tex` e `.log`.
- Commit por task ou por grupo lógico próximo. Preferir commits pequenos e verdes.
- Se durante US2-US4 aparecer necessidade de mudar `CompileArgs` ou `SupportedEngine` que quebre US1, revisitar `spec.md` e `data-model.md` antes de codar.
