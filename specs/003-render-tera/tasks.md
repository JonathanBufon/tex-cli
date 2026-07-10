---
description: "Task list for feature 003-render-tera"
---

# Tasks: Renderização JSON → `.tex` via Tera

**Input**: Design documents from `/specs/003-render-tera/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: Included — plan.md e contracts/cli.md declaram contratos observáveis explícitos (banner, autoescape=false, exit codes 30/31, atomicidade, dry-run byte-identical) que exigem verificação automatizada.

**Organization**: Tasks agrupadas por user story para permitir implementação e teste independentes. MVP = User Story 1 (`render` grava `.tex` no output_dir) — entrega valor sozinho materializando a ponte JSON → LaTeX central da constitution.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1..US4)
- File paths são relativos à raiz do repo.

## Path Conventions

- Rust code: `src/`
- Integration tests: `tests/`
- Docker: `docker/` (inalterado desta spec)
- Examples: `examples/` (opt-in, ver Phase 8)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Ativar `tera` na Cargo.toml e verificar a baseline dos specs 001+002.

- [X] T001 Adicionar `tera = "1"` à seção `[dependencies]` do `Cargo.toml` (crate já listado na stack canônica da constitution, apenas ativado agora). Rodar `docker run --rm -v $(pwd):/src -w /src tex-cli cargo check --all` para confirmar que a resolução baixa `tera` + `pest` sem quebrar nada.

**Checkpoint**: `cargo check --all` passa. `Cargo.lock` inclui `tera`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extensões estruturais que TODAS as user stories dependem — enum de erro, extração do write_atomic helper, registro do novo módulo, esqueleto de subcomando.

- [X] T002 Estender `src/errors.rs`: adicionar variantes `TexError::TeraRenderError { template_name: String, detail: String }` e `TexError::InvalidJson { source: String, detail: String }` conforme data-model.md e research D-06. Atualizar `TexError::exit_code()` mapeando 30 e 31 respectivamente. Estender o `#[cfg(test)] mod tests` para cobrir os dois novos códigos + os Display (`TeraRenderError` menciona nome do template; `InvalidJson` menciona source).
- [X] T003 [P] Criar `src/atomic.rs` com `pub fn write_atomic(target: &Path, bytes: &[u8], mode: u32) -> Result<(), TexError>` extraído de `src/templates.rs::write_atomic_0644`. Adicionar `pub mod atomic;` em `src/lib.rs`. Unit tests inline (`#[cfg(test)]`) cobrindo: (a) grava novo arquivo com mode custom (`0o600`, `0o644`), (b) sobrescreve arquivo existente, (c) cria dirs pais via `create_dir_all`, (d) PermissionDenied em parent read-only → `TexError::PermissionDenied`.
- [X] T004 Atualizar `src/templates.rs::add_template` para chamar `crate::atomic::write_atomic(&dest_path, &bytes, 0o644)` em vez do helper privado. Remover `fn write_atomic_0644` do templates.rs. Rodar `cargo test --lib` no container para garantir que os 28 tests de templates continuam verdes (zero regressão).
- [X] T005 [P] Criar `src/render.rs` (stub vazio com header comment). Adicionar `pub mod render;` em `src/lib.rs`.
- [X] T006 Estender `src/cli.rs`: adicionar variante `Commands::Render(RenderArgs)` ao enum `Commands`. Definir `pub struct RenderArgs { template_name: Option<String>, data_source: Option<String>, #[arg(short = 'o', long)] output: Option<PathBuf>, #[arg(long)] dry_run: bool, #[arg(long)] force: bool }` — args positionais são `Option` para permitir modo interativo quando ausentes. Handler `handle_render(args: RenderArgs)` retorna `Err(anyhow!("não implementado"))`. Registrar dispatch em `src/main.rs`: `Commands::Render(args) => handle_render(args)`.

**Checkpoint**: `cargo build` compila. `tex-cli render` retorna "não implementado". Banner continua em stderr. 45+ testes das specs 001/002 continuam verdes.

---

## Phase 3: User Story 1 - Renderizar template com JSON válido (Priority: P1) 🎯 MVP

**Goal**: Usuário roda `tex-cli render <template> <data.json>` e obtém um `.tex` renderizado no `paths.output_dir`, com escrita atômica e mode 0o644.

**Independent Test**: Fixture com template `artigo.tex` contendo `{{ titulo }}` + JSON `{"titulo":"T"}` → rodar `render artigo dados.json` → validar que `output_dir/artigo.tex` existe com "T" substituído, exit 0, stdout `Renderizado em <path>`.

### Tests for User Story 1 (write first, ensure they FAIL before implementation) ⚠️

- [X] T007 [P] [US1] Criar `tests/cli_render.rs` com testes de integração usando `assert_cmd` + `assert_fs`:
  - `render_writes_file_with_default_output_path`: template + JSON válidos → arquivo criado em `<home>/.config/tex/output/<name>.tex` com conteúdo renderizado (byte-a-byte esperado), mode 0o644.
  - `render_with_output_flag_uses_custom_path`: `--output /tmp/x.tex` → arquivo em `/tmp/x.tex`, e o default NÃO existe.
  - `render_missing_template_exits_20`: nome inexistente → exit 20, stderr menciona nome + templates_dir.
  - `render_malformed_json_exits_31`: JSON com syntax error → exit 31, stderr menciona linha:col do serde_json.
  - `render_json_top_level_array_exits_31`: `["array"]` → exit 31, stderr `esperado objeto`.
  - `render_tera_error_exits_30`: template com `{{ ausente }}` sem chave no JSON → exit 30, stderr `Falha ao renderizar template 'X'`.
  - `render_missing_config_exits_10`: sem config.
  - `render_templates_dir_missing_exits_22`.
  - `render_overwrite_without_force_non_tty_exits_15`: arquivo pré-existente + assert_cmd (não-TTY) → exit 15, arquivo original intacto.
  - `render_force_overwrites_existing`: `--force` → sobrescreve, stdout `Renderizado (sobrescrito)`.
- [X] T008 [P] [US1] Unit tests inline em `src/render.rs` cobrindo `render_template`:
  - `render_template_basic`: template `"Olá, {{ nome }}."` + `{"nome":"X"}` → `"Olá, X."`.
  - `render_template_missing_var_returns_tera_error`: template com `{{ ausente }}` + `{}` → `Err(TexError::TeraRenderError { template_name: "test", .. })`.
  - `render_template_uses_default_filter`: `{{ x | default(value="Y") }}` + `{}` → `"Y"`.
  - `render_template_nested_access`: `{{ obj.chave }}` + `{"obj":{"chave":"Z"}}` → `"Z"`.
  - `render_template_no_autoescape_for_latex`: `"{{ x }}"` + `{"x":"& % $"}` → `"& % $"` (não `&amp; %25 $`).
  - `render_template_top_level_array_returns_invalid_json`: value é array → `Err(TexError::InvalidJson { .. })`.

### Implementation for User Story 1

- [X] T009 [US1] Implementar `struct RenderArgs` (já definido em src/cli.rs pelo T006) e `struct RenderOutcome { output_path: Option<PathBuf>, bytes_written: u64, overwrote_existing: bool, dry_run: bool }` em `src/render.rs` conforme data-model.md.
- [X] T010 [US1] Implementar `pub fn render_template(template_name: &str, template_src: &str, json_value: &serde_json::Value) -> Result<String, TexError>` em `src/render.rs`: (1) `if !json_value.is_object() → return Err(TexError::InvalidJson { source: "<template context>".into(), detail: format!("esperado objeto no topo, recebido {}", type_name(json_value)) })`; (2) `let ctx = tera::Context::from_value(json_value.clone()).map_err(|e| TexError::InvalidJson { .. })?`; (3) `tera::Tera::one_off(template_src, &ctx, false).map_err(|e| TexError::TeraRenderError { template_name: template_name.to_string(), detail: e.to_string() })`. Helper `fn type_name(v: &serde_json::Value) -> &'static str` retorna "object"/"array"/"string"/etc. Depende de T002, T009.
- [X] T011 [P] [US1] Implementar `pub fn load_json_source(source: &str) -> Result<serde_json::Value, TexError>` em `src/render.rs` conforme D-04: (1) se `source == "-"`: `io::stdin().read_to_string`; senão `fs::read_to_string(source)` (mapeando `PermissionDenied` → `TexError::PermissionDenied`); (2) `if content.trim().is_empty() → Err(TexError::InvalidJson { source: display, detail: "sem conteúdo".into() })`; (3) `serde_json::from_str::<Value>(&content).map_err(|e| TexError::InvalidJson { source, detail: e.to_string() })`. Display source = "stdin" ou path.
- [X] T012 [US1] Implementar `pub fn resolve_output_path(cfg: &Config, args: &RenderArgs) -> Result<PathBuf, TexError>` em `src/render.rs`: se `args.output = Some(path)` → `paths::expand_user_path(path)`; senão → `cfg.paths.output_dir.join(format!("{}.tex", args.template_name))`.
- [X] T013 [US1] Implementar `pub fn render_and_write(args: &RenderArgs, cfg: &Config) -> Result<RenderOutcome, TexError>` em `src/render.rs` conforme data-model.md fluxo: read_template → load_json_source → render_template → write_atomic (via `crate::atomic::write_atomic(&path, rendered.as_bytes(), 0o644)`). Cria dirs pais via `fs::create_dir_all(path.parent())` antes do write. Retorna `RenderOutcome`. Depende de T010, T011, T012, T003.
- [X] T014 [US1] Implementar `handle_render(args: RenderArgs)` em `src/cli.rs`:
  1. Load config (exit 10/11).
  2. Se `args.template_name.is_none() || args.data_source.is_none()` → delega para `handle_render_menu()` (implementado em Phase 6 US4; por enquanto retorna `anyhow!("modo interativo requer args em US1 — implementado na US4")` para preservar green US1 sem menu).
  3. Constrói `RenderArgs` "concreto" (unwrap dos Options).
  4. Chama `render_and_write(&args, &cfg)?`.
  5. stdout: `Renderizado em {path}.` ou `Renderizado (sobrescrito) em {path}.` baseado em `overwrote_existing`.
  6. Depende de T013.
- [X] T015 [US1] Rodar `tests/cli_render.rs` no container. Todos os 10 testes de US1 devem passar. Ajustar mensagens até casarem com predicates. Rodar `cargo test --lib render` para confirmar unit tests inline verdes.

**Checkpoint**: MVP funcional. `tex-cli render greeting /tmp/data.json` produz `.tex` renderizado no output_dir. Container: `cargo test --test cli_render` verde.

---

## Phase 4: User Story 2 - Dry-run em stdout (Priority: P2)

**Goal**: `tex-cli render <template> <data.json> --dry-run` imprime o `.tex` renderizado em stdout sem gravar arquivo.

**Independent Test**: Executar dry-run, capturar stdout, comparar byte-a-byte com o arquivo que seria gravado no caminho normal (rodado depois em ambiente limpo). Validar que dry-run NÃO cria arquivo no output_dir.

### Tests for User Story 2 ⚠️

- [X] T016 [P] [US2] Estender `tests/cli_render.rs`:
  - `render_dry_run_prints_to_stdout_and_no_file`: `--dry-run` → stdout tem `.tex` renderizado, `output_dir/<name>.tex` NÃO existe (SC-005 preparação).
  - `render_dry_run_byte_identical_to_written_file`: renderiza duas vezes — uma com `--dry-run` (stdout), outra sem (arquivo). Comparar bytes idênticos (SC-005 completo).
  - `render_dry_run_ignores_output_flag`: `--dry-run --output /tmp/x.tex` → stdout tem conteúdo, `/tmp/x.tex` NÃO é criado.
  - `render_dry_run_still_returns_tera_error`: `--dry-run` sobre template com var ausente → exit 30 (dry-run não suprime erros).

### Implementation for User Story 2

- [X] T017 [US2] Estender `render_and_write` em `src/render.rs`: (1) se `args.dry_run` → `io::stdout().write_all(rendered.as_bytes())?`; retornar `RenderOutcome { output_path: None, bytes_written: rendered.len() as u64, overwrote_existing: false, dry_run: true }`; (2) senão continua fluxo atual (T013).
- [X] T018 [US2] Estender `handle_render`: se `outcome.dry_run` → não imprime linha `Renderizado em ...` (mantém stdout limpo pro pipe). Emitir `tracing::warn!` se `args.dry_run && args.output.is_some()` (D-08 do research).
- [X] T019 [US2] Rodar `tests/cli_render.rs`. Novos 4 testes verdes; os 10 originais continuam.

**Checkpoint**: `render --dry-run` pipeável, byte-identical, ignora `--output`.

---

## Phase 5: User Story 3 - JSON via stdin (Priority: P2)

**Goal**: `echo '{"x":1}' | tex-cli render tpl -` consome stdin como JSON.

**Independent Test**: Piped JSON → mesmo resultado que passar arquivo. Stdin vazio → exit 31.

### Tests for User Story 3 ⚠️

- [X] T020 [P] [US3] Estender `tests/cli_render.rs`:
  - `render_stdin_json_dash_arg`: `write_stdin('{"nome":"X"}')` + arg `-` → mesmo output esperado que arquivo equivalente.
  - `render_stdin_empty_exits_31`: stdin vazio + arg `-` → exit 31, stderr `sem conteúdo`.
  - `render_stdin_malformed_exits_31`: stdin com `not json` → exit 31, stderr `stdin` no source.

### Implementation for User Story 3

- [X] T021 [US3] `load_json_source` já cobre este caso via T011. **Nenhuma nova implementação necessária.** Task é apenas validar via `tests/cli_render.rs` que os 3 testes passam. Se algum falhar, ajustar a mensagem de erro em `load_json_source` (ex.: mudar detail de `sem conteúdo` para casar com predicate).

**Checkpoint**: Composição Unix via stdin funciona.

---

## Phase 6: User Story 4 - Modo interativo `render` (Priority: P3)

**Goal**: `tex-cli render` sem args abre menu inquire (Select template + Text JSON path + Confirm dry-run).

**Independent Test**: TTY: menu completo. Sem TTY: exit 1. Sem templates: exit 22.

### Implementation for User Story 4

- [ ] T022 [US4] Estender `src/interactive.rs`: (a) `pub fn prompt_json_source() -> Result<String, TexError>` — `inquire::Text::new("Caminho do arquivo JSON (- para stdin):").prompt()` mapeado ao `TexError::UserAborted` em cancel; (b) `pub fn confirm_dry_run() -> Result<bool, TexError>` — `inquire::Confirm::new("Modo dry-run?").with_default(false)`. Reusa `prompt_template_name` da spec 002 (já público).
- [ ] T023 [US4] Implementar `handle_render_menu(cfg: &Config)` em `src/cli.rs`:
  1. `if !stdin().is_terminal()` → `anyhow!("Menu interativo de render requer terminal. Use tex-cli render <template> <data.json>.")` (exit 1).
  2. `let templates = list_templates(&cfg.paths.templates_dir)?` (herda exit 22 se dir ausente).
  3. Se `templates.is_empty()` → `anyhow!("Nenhum template encontrado em {}. Adicione um com tex-cli templates add.", cfg.paths.templates_dir.display())`. Mapear para exit 22 no path do main (`downcast` ou custom).
  4. `let names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();`
  5. `let template_name = prompt_template_name(&names)?;`
  6. `let data_source = prompt_json_source()?;`
  7. `let dry_run = confirm_dry_run()?;`
  8. Constrói `RenderArgs { template_name: Some(template_name), data_source: Some(data_source), output: None, dry_run, force: false }` e chama `handle_render(args)`.
- [ ] T024 [US4] Atualizar `handle_render` (T014) para delegar realmente a `handle_render_menu(&cfg)` quando args positionais estão vazios (remover o placeholder anyhow do T014). Depende de T023.
- [ ] T025 [US4] Smoke test manual do modo interativo (documentado como manual QA no PR, não automatizado — inquire requer TTY). Registrar procedimento no quickstart § 4 "Modo interativo".

**Checkpoint**: Menu funcional em TTY, degrada gracefully.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Verificações transversais, higiene, docs, examples, validação de performance.

- [ ] T026 [P] Estender `tests/cli_banner.rs` (existente) para cobrir `render`:
  - `banner_appears_on_render_stderr` — invoca `render` com args válidos, valida marker em stderr.
  - `banner_never_leaks_on_render_dry_run` — `render ... --dry-run`, verifica que **nenhuma linha do banner asset** aparece em stdout (protege pipe pra `less`/`jq`).
- [ ] T027 [P] Rodar `cargo fmt --all -- --check` e `cargo clippy --all-targets --all-features -- -D warnings` no container. Corrigir violações.
- [ ] T028 [P] Estender `README.md` da raiz: seção "Renderizar JSON → .tex" com exemplos dos 4 fluxos principais (arquivo, dry-run, stdin, menu interativo). Atualizar tabela de exit codes com 30/31. Adicionar linha ao roadmap indicando spec 004 (compile) como próxima.
- [ ] T029 [P] Adicionar template + JSON de exemplo em `examples/`:
  - `examples/data/greeting.json` — objeto com 2-3 campos.
  - Documentar em `examples/README.md` a receita `tex-cli render greeting examples/data/greeting.json` (assume que `templates/greeting.tex` já foi adicionado). Zero mudança em `examples/LICENSE-EXAMPLES`.
- [ ] T030 Rodar o quickstart.md do início ao fim dentro do container Docker: build → init → templates add → `render` (arquivo, dry-run, stdin, force, output custom). Documentar tempo total e outcomes no PR.
- [ ] T031 Validar SC-001 (`render` com template ~10 KB + 10 vars < 100 ms) usando o binário release. Registrar 3-5 medições no PR ou em nota no quickstart § 7.
- [ ] T032 Validar SC-005 (`dry-run` byte-a-byte idêntico ao arquivo gravado) por teste automatizado em `tests/cli_render.rs::render_dry_run_byte_identical_to_written_file` (T016 já cobre — task é confirmar green e mencionar SC-005 no commit message).

**Checkpoint**: Feature completa, testada, documentada, examples atribuídos, performance validada.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências.
- **Foundational (Phase 2)**: depende de Setup. **BLOQUEIA todas as user stories.**
- **User Story 1 (P1 / Phase 3)**: depende de Foundational.
- **User Story 2 (P2 / Phase 4)**: depende de US1 (usa `render_and_write` já implementado).
- **User Story 3 (P2 / Phase 5)**: depende de US1 (`load_json_source` já implementado — task é só validar).
- **User Story 4 (P3 / Phase 6)**: depende de US1 e reutiliza handler existente.
- **Polish (Phase 7)**: depende de todas as US completadas na versão MVP alvo.

### Within Each User Story

- Tests **antes** da implementação (falham → verde após).
- Funções puras (`render_template`, `load_json_source`) antes de handlers.
- Handler CLI é sempre a **última** camada.

### Parallel Opportunities

- Foundational: T003 e T005 são [P] (arquivos novos); T002 toca errors.rs, T004 toca templates.rs, T006 toca cli.rs — independentes entre si.
- US1: testes de integração (T007) e unit tests inline (T008) são [P] com implementação inicial. T011 (load_json_source) e T010 (render_template) são [P] entre si (funções livres, arquivos diferentes de implementação embora ambas em render.rs — cuidado com merge conflict, coordenar via T009 primeiro).
- US2 tests (T016), US3 tests (T020), Polish (T026, T027, T028, T029) todos [P].

---

## Parallel Example: User Story 1

```bash
# Depois de completar Foundational (T002-T006):
# Rodar em paralelo os testes que serão VALIDADOS depois:
Task: "T007 [P] [US1] Criar tests/cli_render.rs"
Task: "T008 [P] [US1] Unit tests inline em src/render.rs"

# Depois structs+enums podem ir em paralelo com funções puras:
Task: "T009 [US1] Struct RenderOutcome"
Task: "T011 [P] [US1] load_json_source"

# Sequenciais (dependências claras):
Task: "T010 [US1] render_template (tera + type_name)"
Task: "T012 [US1] resolve_output_path"
Task: "T013 [US1] render_and_write (glue)"
Task: "T014 [US1] handle_render em src/cli.rs"
Task: "T015 [US1] Rodar tests/cli_render.rs até verde"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup (Phase 1) → Foundational (Phase 2) → US1 (Phase 3) → parar.
2. **STOP & VALIDATE**: `docker run … cargo test --test cli_render` verde. Renderização real produz `.tex` esperado.
3. Deploy/demo se ready. Merge se ok.

### Incremental Delivery

1. Foundation → merge.
2. US1 → merge → **MVP** (render arquivo).
3. US2 → merge → dry-run.
4. US3 → merge → stdin.
5. US4 → merge → menu interativo.
6. Polish (Phase 7) → merge.

Para consistência com as specs 001/002, esta feature também pode ser
entregue em **PR único com commits granulares por T-task**
(disciplina `commit por implementação`).

### Parallel Team Strategy

Projeto single-dev — não aplicável. Se um segundo dev entrar após
Foundational: US1..US4 são independentes; coordenação leve em
`src/render.rs` (mesmo arquivo).

---

## Notes

- Todas as tarefas devem terminar com o teste correspondente verde antes de avançar.
- Docker container `tex-cli` reaproveitado sem mudança.
- Nenhuma tarefa toca `src/config.rs` — a struct `Config` permanece imutável.
- `src/atomic.rs` é o único módulo novo compartilhado. Refactor de `Config::save_atomic` para consumi-lo fica anotado como task futura (fora desta spec).
- Commit por task ou por grupo lógico próximo. Preferir commits pequenos e verdes.
- Se durante US2-US4 aparecer necessidade de mudar `RenderArgs` que quebre US1, revisitar `spec.md` e `data-model.md` antes de codar — evitar deriva silenciosa da spec.
