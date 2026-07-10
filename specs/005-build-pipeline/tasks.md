---
description: "Task list for feature 005-build-pipeline"
---

# Tasks: Pipeline JSON → PDF (`build`)

**Input**: Design documents from `/specs/005-build-pipeline/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: Included — plan.md e contracts/cli.md declaram contratos observáveis explícitos (banner, PDF magic bytes, propagação de exit codes 20/22/30/31/40/41/42, FR-15 preservação do .tex em falha, atomicidade) que exigem verificação automatizada.

**Organization**: Tasks agrupadas por user story. MVP = US1 (build produz PDF) — fecha o pipeline JSON → PDF end-to-end junto com specs 003 e 004.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1..US4)
- File paths são relativos à raiz do repo.

## Path Conventions

- Rust code: `src/`
- Integration tests: `tests/`
- Docker: `docker/` (inalterado)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Verificar baseline após spec 004 mergeada. Nenhuma dep nova.

- [X] T001 Rodar `docker run --rm -v $(pwd):/src -w /src tex-cli cargo check --all` no container Docker para confirmar que a baseline das specs 001-004 continua compilando limpo, e que zero nova dep será necessária. Rodar também `cargo test --lib` (rápido, sem tectonic) pra garantir que os 80+ testes unitários passam.

**Checkpoint**: `cargo check --all` limpo, `cargo test --lib` verde.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Estrutura mínima que TODAS as user stories precisam — novo módulo `build.rs`, dispatcher CLI. **Zero mudança em `TexError`** e em outros módulos das specs anteriores.

- [X] T002 [P] Criar `src/build.rs` (stub vazio com header comment apontando pra T005+ onde a lógica chega) e adicionar `pub mod build;` em `src/lib.rs`.
- [X] T003 Estender `src/cli.rs`: adicionar variante `Commands::Build(BuildArgs)` ao enum `Commands`. Definir `pub struct BuildArgs { template_name: Option<String>, data_source: Option<String>, #[arg(short = 'o', long)] output: Option<PathBuf>, #[arg(short = 'e', long)] engine: Option<String>, #[arg(long, conflicts_with = "no_keep_tex")] keep_tex: bool, #[arg(long = "no-keep-tex", conflicts_with = "keep_tex")] no_keep_tex: bool, #[arg(long, conflicts_with = "no_keep_logs")] keep_logs: bool, #[arg(long = "no-keep-logs", conflicts_with = "keep_logs")] no_keep_logs: bool, #[arg(long)] force: bool }`. Handler `handle_build(args)` retorna `Err(anyhow!("não implementado"))`. Registrar dispatch em `src/main.rs`: `Commands::Build(args) => handle_build(args)`.

**Checkpoint**: `cargo build` compila. `tex-cli build tpl data.json` retorna "não implementado". Banner em stderr. Flags conflitantes → exit 2 (clap). Todos os 165+ testes das specs 001-004 continuam verdes.

---

## Phase 3: User Story 1 - Gerar PDF em uma invocação (Priority: P1) 🎯 MVP

**Goal**: Usuário roda `tex-cli build artigo dados.json` e obtém PDF válido em `<output_dir>/artigo.pdf` numa única invocação.

**Independent Test**: Fixture com template + JSON válidos → rodar `build` → validar PDF criado com magic bytes `%PDF-`, mode 0o644, exit 0, stdout menciona path e tempo total.

### Tests for User Story 1 (write first, ensure they FAIL before implementation) ⚠️

- [X] T004 [P] [US1] Criar `tests/cli_build.rs` com testes de integração usando `assert_cmd` + `assert_fs`:
  - `build_produces_pdf_with_default_output_path`: fixture `.tex` mínimo válido + JSON com dados → PDF em `<output_dir>/<template>.pdf`.
  - `build_pdf_has_pdf_magic_bytes`: `std::fs::read(&pdf)[0..4] == b"%PDF"`.
  - `build_pdf_has_mode_0644`: mode & 0o777 == 0o644.
  - `build_stdout_mentions_path_and_total_duration`: stdout regex `PDF gerado em .*\. Pipeline \(render \+ compile\) levou \d+(\.\d+)?s\.`.
  - `build_with_output_flag_uses_custom_path`.
  - `build_missing_template_exits_20`.
  - `build_malformed_json_exits_31`.
  - `build_json_top_level_array_exits_31`.
  - `build_tera_error_exits_30`.
  - `build_broken_tex_after_render_exits_40_with_log_tail`: template que produz `.tex` inválido.
  - `build_missing_config_exits_10`.
  - `build_overwrite_without_force_non_tty_exits_15`: PDF existente + `!--force` + sem TTY.
  - `build_force_overwrites_existing_pdf`: `--force` → sobrescreve, stdout menciona "sobrescrito".
- [ ] T005 [P] [US1] Unit tests inline em `src/build.rs` (`#[cfg(test)] mod tests`) cobrindo:
  - `resolve_output_pdf_default`: sem `--output` → `<output_dir>/<name>.pdf`.
  - `resolve_output_pdf_custom`: com `--output` → path expandido.
  - `resolve_intermediate_tex_path`: retorna `<output_dir>/<name>.tex`.

### Implementation for User Story 1

- [ ] T006 [P] [US1] Implementar `pub struct BuildOutcome` em `src/build.rs` conforme data-model.md D-04: campos `pdf_path`, `total_duration`, `render_duration`, `compile_duration`, `bytes_written`, `overwrote_existing`, `kept_tex`, `kept_logs`, `intermediate_tex_path: Option<PathBuf>`.
- [ ] T007 [P] [US1] Implementar `pub fn resolve_output_pdf(cfg: &Config, template_name: &str, output: Option<&Path>) -> PathBuf` em `src/build.rs`: `output` presente → `paths::expand_user_path`; ausente → `cfg.paths.output_dir.join(format!("{template_name}.pdf"))`.
- [ ] T008 [P] [US1] Implementar `pub fn resolve_intermediate_tex_path(cfg: &Config, template_name: &str) -> PathBuf` em `src/build.rs`: retorna `cfg.paths.output_dir.join(format!("{template_name}.tex"))`.
- [ ] T009 [US1] Implementar `pub fn build_pipeline(cfg: &Config, template_name: &str, data_source: &str, output_pdf: &Path, engine: SupportedEngine, keep_tex: bool, keep_logs: bool, force: bool, verbose: u8) -> Result<BuildOutcome, TexError>` em `src/build.rs` conforme fluxo do data-model.md:
  1. `let start_total = Instant::now();`
  2. `let template_bytes = templates::read_template(&cfg.paths.templates_dir, template_name)?`.
  3. `let template_src = std::str::from_utf8(&template_bytes).map_err(...)?`.
  4. `let value = render::load_json_source(data_source)?`.
  5. `let start_render = Instant::now();`
  6. `let rendered = render::render_template(template_name, template_src, &value)?`.
  7. `let render_duration = start_render.elapsed();`
  8. Log `tracing::info!("Render concluído em {}ms", render_duration.as_millis())` (research D-10).
  9. **Se `keep_tex`**: `let intermediate_path = resolve_intermediate_tex_path(cfg, template_name); atomic::write_atomic(&intermediate_path, rendered.as_bytes(), 0o644)?`. Guardar em `intermediate_tex_path: Some(...)`.
  10. `let temp = TempDir::new()?; let tex_in_temp = temp.path().join(format!("{template_name}.tex")); fs::write(&tex_in_temp, &rendered)?`.
  11. `let start_compile = Instant::now();`
  12. Log `tracing::info!("Compilando com engine '{engine}'...")`.
  13. `let compile_outcome = compiler::compile_and_write(cfg, &tex_in_temp, engine, output_pdf, /* keep_tex= */ false, keep_logs, force, verbose)?`. **NB**: passa `keep_tex=false` porque já lidamos com o `.tex` na etapa 9 (research D-06).
  14. `let compile_duration = start_compile.elapsed();`
  15. Log `tracing::info!("Compile concluído em {}s", compile_duration.as_secs_f32())`.
  16. Constrói `BuildOutcome { pdf_path: output_pdf.to_path_buf(), total_duration: start_total.elapsed(), render_duration, compile_duration, bytes_written: compile_outcome.bytes_written, overwrote_existing: compile_outcome.overwrote_existing, kept_tex: keep_tex, kept_logs: keep_logs, intermediate_tex_path }`.
- [ ] T010 [US1] Implementar `handle_build(args: BuildArgs)` em `src/cli.rs`:
  1. Load config (exit 10/11).
  2. Se `args.template_name.is_none() || args.data_source.is_none()` → delega para `handle_build_menu(&cfg)` (implementado em US4; por ora placeholder `anyhow!("modo interativo será implementado na US4")`).
  3. Expandir args positionais.
  4. Resolve `engine` via `compiler::resolve_engine(args.engine.as_deref(), &cfg.compiler.engine)` (exit 42 possível).
  5. Resolve `output_pdf` via `build::resolve_output_pdf(&cfg, template_name, args.output.as_deref())`.
  6. Resolve `keep_tex`/`keep_logs`: flag → override; senão `cfg.compiler.*`.
  7. Overwrite handling: se `output_pdf.exists() && !args.force`: TTY → `confirm_compile_overwrite(&output_pdf)?` (reusa spec 004); senão exit 15.
  8. `let outcome = build::build_pipeline(&cfg, template_name, data_source, &output_pdf, engine, keep_tex, keep_logs, args.force, 0)?`.
  9. Compor stdout:
     - `overwrote_existing = true` → `PDF gerado (sobrescrito) em {path}. Pipeline (render + compile) levou {sec:.1}s.`
     - senão → `PDF gerado em {path}. Pipeline (render + compile) levou {sec:.1}s.`
  10. Depende de T009.
- [ ] T011 [US1] Rodar `tests/cli_build.rs` no container. Todos os 13 testes de US1 devem passar. Ajustar mensagens/regex até casarem com predicates. Rodar `cargo test --lib build` para confirmar unit tests inline verdes.

**Checkpoint**: MVP funcional. `tex-cli build <tpl> <data>` produz PDF válido no output_dir. Container: `cargo test --test cli_build` verde.

---

## Phase 4: User Story 2 - Preservar `.tex` intermediário (Priority: P2)

**Goal**: `--keep-tex` grava `.tex` renderizado em `output_dir` **antes** do compile, permitindo debug se compile falhar (FR-15).

**Independent Test**: Rodar `build tpl broken-data.json --keep-tex` onde o template renderiza um `.tex` que falha na compilação → validar que exit=40, `<output_dir>/tpl.tex` EXISTE (não foi apagado), `<output_dir>/tpl.pdf` NÃO existe.

### Tests for User Story 2 ⚠️

- [ ] T012 [P] [US2] Estender `tests/cli_build.rs`:
  - `build_keep_tex_writes_intermediate_to_output_dir`: `--keep-tex` + sucesso → `<output_dir>/<template>.tex` existe com conteúdo renderizado (byte-for-byte esperado).
  - `build_keep_logs_writes_log_to_output_dir`: `--keep-logs` + sucesso → `<output_dir>/<template>.log` existe.
  - `build_keep_tex_preserves_intermediate_after_compile_fail`: template renderiza OK mas compile falha → exit 40, `.tex` intermediário existe no `output_dir`, `.pdf` NÃO existe. **Este teste materializa SC-004 e FR-15.**
  - `build_no_keep_tex_leaves_only_pdf`: config com keep_tex=true + `--no-keep-tex` → só `.pdf` fica no output.
  - `build_conflicting_keep_tex_flags_exit_2`: `--keep-tex --no-keep-tex` → exit 2.
  - `build_conflicting_keep_logs_flags_exit_2`.
  - `build_flags_override_config_defaults`: config keep_tex=true + `--no-keep-tex` → sem .tex.

### Implementation for User Story 2

- [ ] T013 [US2] A lógica de resolver `keep_tex`/`keep_logs` já foi implementada em T010 e `build_pipeline` (T009) já grava o intermediate `.tex` antes do compile (etapa 9 do fluxo). **Nenhuma nova implementação necessária.** Task é apenas validar via `tests/cli_build.rs` que os 7 testes de US2 passam. Se algum falhar, ajustar mensagens ou ordem do fluxo.

**Checkpoint**: FR-15 provado por teste (`build_keep_tex_preserves_intermediate_after_compile_fail`). Sem regressão em US1.

---

## Phase 5: User Story 3 - JSON via stdin (Priority: P2)

**Goal**: `echo '{"x":1}' | tex-cli build tpl -` consome stdin como JSON.

**Independent Test**: Piped JSON → mesmo resultado que arquivo. Stdin vazio → exit 31.

### Tests for User Story 3 ⚠️

- [ ] T014 [P] [US3] Estender `tests/cli_build.rs`:
  - `build_stdin_json_dash_arg`: `write_stdin('{"nome":"X"}')` + arg `-` → PDF gerado normalmente.
  - `build_stdin_empty_exits_31`: stdin vazio + arg `-` → exit 31.
  - `build_stdin_malformed_exits_31`: stdin com `not json` → exit 31.

### Implementation for User Story 3

- [ ] T015 [US3] `render::load_json_source` já suporta stdin (spec 003). `build_pipeline` (T009) passa o `data_source` diretamente. **Nenhuma nova implementação necessária.** Task é apenas validar via testes T014.

**Checkpoint**: Composição Unix via stdin funciona.

---

## Phase 6: User Story 4 - Modo interativo (Priority: P3)

**Goal**: `tex-cli build` sem args abre menu com Select do template + Text pro JSON + Confirm keep_tex/keep_logs.

**Independent Test**: TTY: menu completo. Sem TTY: exit 1. Sem templates: exit 22.

### Implementation for User Story 4

- [ ] T016 [US4] Implementar `handle_build_menu(cfg: &Config)` em `src/cli.rs`:
  1. `if !stdin().is_terminal()` → `anyhow!("Menu interativo de build requer terminal. Use tex-cli build <template> <data.json>.")` (exit 1).
  2. `let templates = templates::list_templates(&cfg.paths.templates_dir)?` (exit 22 se dir ausente).
  3. Se vazio → `anyhow!("Nenhum template encontrado em {}. Adicione um com tex-cli templates add.")` mapeado a exit 22.
  4. `let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();`
  5. `let template_name = prompt_template_name(&names)?;` (reuso spec 002).
  6. `let data_source = prompt_json_source()?;` (reuso spec 003).
  7. `let keep_tex = confirm_keep_tex(cfg.compiler.keep_tex)?;` (reuso spec 004).
  8. `let keep_logs = confirm_keep_logs(cfg.compiler.keep_logs)?;` (reuso spec 004).
  9. Constrói `BuildArgs { template_name: Some(template_name), data_source: Some(data_source), output: None, engine: None, keep_tex, no_keep_tex: !keep_tex, keep_logs, no_keep_logs: !keep_logs, force: false }` e chama `handle_build(args)`.
- [ ] T017 [US4] Atualizar `handle_build` (T010) para delegar realmente a `handle_build_menu(&cfg)` quando args positionais estão vazios (remover placeholder do T010).
- [ ] T018 [US4] Smoke test manual do modo interativo — documentado no quickstart § 4 "Modo interativo".

**Checkpoint**: Menu funcional em TTY, degrada gracefully.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Verificações transversais, higiene, docs, validação de performance.

- [ ] T019 [P] Estender `tests/cli_banner.rs` (existente): `banner_appears_on_build_stderr` — invoca `build` com args válidos, valida marker em stderr.
- [ ] T020 [P] Estender `tests/cli_build.rs` com testes de US extras:
  - `build_engine_flag_overrides_config_and_config_stays_unchanged` (SC-005).
  - `build_engine_not_supported_exits_42`.
  - `build_engine_not_installed_exits_41` (usa `PATH=/tmp/empty`).
  - `build_leaves_no_artefacts_in_tmp` (SC-006, mesmo pattern do compile).
- [ ] T021 [P] Rodar `cargo fmt --all -- --check` e `cargo clippy --all-targets --all-features -- -D warnings` no container. Corrigir violações.
- [ ] T022 [P] Estender `README.md` da raiz: seção "Pipeline JSON → PDF (`build`)" com exemplos dos fluxos principais (default, custom output, engine override, keep-*, stdin, menu interativo). Adicionar linha ao roadmap indicando o pipeline JSON → PDF fechado end-to-end.
- [ ] T023 Rodar o quickstart.md do início ao fim dentro do container Docker: build → init → templates add → build → verificar PDF válido. Documentar tempo total e outcomes no PR.
- [ ] T024 Validar SC-001 (`build` template simples via tectonic < 20 s) usando o binário release. Registrar 3-5 medições no PR ou em nota no quickstart § 7.
- [ ] T025 Validar SC-002 (economia >= 30% vs. render+compile separados). Bench comparativo em `quickstart § 7`: (a) rodar `render` + `compile` separadamente, medir wall-clock; (b) rodar `build` equivalente, medir wall-clock; (c) validar razão < 0.70. Registrar no PR.
- [ ] T026 Validar SC-004 (`.tex` preservado em compile-fail com `--keep-tex`) por teste automatizado em `build_keep_tex_preserves_intermediate_after_compile_fail` (T012 já cobre — confirmar green e mencionar SC-004 no commit final).
- [ ] T027 Validar SC-005 (config imutável) por teste automatizado (T020 cobre).
- [ ] T028 Validar SC-006 (zero artefato) por teste automatizado (T020 cobre).

**Checkpoint**: Feature completa, testada, documentada, performance validada.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências.
- **Foundational (Phase 2)**: depende de Setup. **BLOQUEIA todas as user stories.**
- **US1 (P1 / Phase 3)**: depende de Foundational.
- **US2 (P2 / Phase 4)**: depende de US1 (usa `build_pipeline` já implementado).
- **US3 (P2 / Phase 5)**: depende de US1.
- **US4 (P3 / Phase 6)**: depende de US1 e reutiliza handler.
- **Polish (Phase 7)**: depende de todas as US completadas.

### Within Each User Story

- Tests **antes** da implementação (falham → verde após).
- Funções puras (`resolve_output_pdf`, `resolve_intermediate_tex_path`) antes de `build_pipeline`.
- Handler CLI é sempre a **última** camada.

### Parallel Opportunities

- Foundational: T002 é [P] (arquivo novo); T003 toca cli.rs.
- US1: testes de integração (T004) e unit tests inline (T005) são [P] com implementação inicial. T006, T007, T008 são [P] entre si (funções livres/structs independentes).
- US2 tests (T012), US3 tests (T014), Polish (T019, T020, T021, T022) todos [P].

---

## Parallel Example: User Story 1

```bash
# Depois de completar Foundational (T002-T003):
# Rodar em paralelo os testes que serão VALIDADOS depois:
Task: "T004 [P] [US1] Criar tests/cli_build.rs"
Task: "T005 [P] [US1] Unit tests inline em src/build.rs"

# Structs + helpers podem ir em paralelo:
Task: "T006 [P] [US1] BuildOutcome struct"
Task: "T007 [P] [US1] resolve_output_pdf"
Task: "T008 [P] [US1] resolve_intermediate_tex_path"

# Sequenciais (dependências claras):
Task: "T009 [US1] build_pipeline (orquestração)"
Task: "T010 [US1] handle_build em src/cli.rs"
Task: "T011 [US1] Rodar tests/cli_build.rs até verde"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup (Phase 1) → Foundational (Phase 2) → US1 (Phase 3) → parar.
2. **STOP & VALIDATE**: `docker run … cargo test --test cli_build` verde. `.tex` real produz PDF válido em uma invocação.
3. Deploy/demo se ready. Merge se ok.

### Incremental Delivery

1. Foundation → merge.
2. US1 → merge → **MVP** (build básico).
3. US2 → merge → keep flags + FR-15.
4. US3 → merge → stdin.
5. US4 → merge → menu interativo.
6. Polish (Phase 7) → merge.

Para consistência com specs 001-004, esta feature pode ser
entregue em **PR único com commits granulares por T-task**.

### Parallel Team Strategy

Projeto single-dev — não aplicável.

---

## Notes

- Todas as tarefas devem terminar com o teste correspondente verde antes de avançar.
- Docker container `tex-cli` reaproveitado sem mudança.
- Nenhuma tarefa toca `src/config.rs`, `src/errors.rs`, `src/atomic.rs`, `src/paths.rs`, `src/render.rs`, `src/compiler.rs`, `src/templates.rs`, `src/interactive.rs` — todos imutáveis nesta spec.
- **Zero nova variante em `TexError`** — reuso puro dos erros das specs anteriores.
- Commit por task ou por grupo lógico próximo. Preferir commits pequenos e verdes.
