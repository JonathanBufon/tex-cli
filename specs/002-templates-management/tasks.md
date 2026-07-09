---
description: "Task list for feature 002-templates-management"
---

# Tasks: Gestão de Templates LaTeX

**Input**: Design documents from `/specs/002-templates-management/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: Included — o plan e o contracts/cli.md declaram contratos observáveis explícitos (banner, permissão `0644`, exit codes 20/21/22, atomicidade do `add`) que exigem verificação automatizada.

**Organization**: Tasks são agrupadas por user story para permitir implementação e teste independentes. MVP = User Story 1 (`templates list`) — entrega valor sozinho ao permitir descoberta dos templates existentes.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1..US5)
- File paths são relativos à raiz do repo.

## Path Conventions

- Rust code: `src/`
- Integration tests: `tests/` (cada arquivo é um binário de teste)
- Docker: `docker/` (inalterado desta spec)
- Examples: `examples/` (opt-in, ver Phase 8)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirmar que a base da spec 001 permanece intacta e que nenhuma nova dep é necessária.

- [X] T001 Verificar que a branch atual é `002-templates-management` e que o `Cargo.toml` não precisa de mudanças (research D-01: zero nova dep). Rodar `cargo check --all` no container Docker (`docker run --rm -v $(pwd):/src -w /src tex-cli cargo check --all`) para confirmar que o estado herdado da spec 001 compila limpo.

**Checkpoint**: base pronta. `cargo check --all` passa sem warnings.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Extensões estruturais que TODAS as user stories dependem — enum de erro, registro do novo módulo, esqueleto de subcomando. Nada de user story pode iniciar até esta fase estar completa.

- [X] T002 Estender `src/errors.rs`: adicionar variantes `TexError::TemplateNotFound { name: String, templates_dir: PathBuf }`, `TexError::InvalidUtf8 { source_path: PathBuf, detail: String }`, `TexError::TemplatesDirMissing { templates_dir: PathBuf }` conforme data-model.md. Atualizar `TexError::exit_code()` mapeando 20, 21, 22 respectivamente. Estender o `#[cfg(test)] mod tests` para cobrir os três novos códigos.
- [X] T003 [P] Criar `src/templates.rs` (stub vazio com comentário de header) e adicionar `pub mod templates;` em `src/lib.rs`.
- [X] T004 Estender `src/cli.rs`: adicionar variante `Commands::Templates(TemplatesCmd)` ao enum `Commands`. Definir `pub enum TemplatesCmd { List { format: ShowFormat }, Show { name: String }, Add(AddArgs), Remove { name: String, #[arg(long)] force: bool }, Menu }`. Definir `pub struct AddArgs { source_path: PathBuf, #[arg(short = 'n', long)] name: Option<String>, #[arg(long)] force: bool }`. Handlers `handle_templates_list/show/add/remove/menu` retornam `Err(anyhow!("não implementado"))` neste ponto. Registrar dispatch em `src/main.rs`. Nota: `templates` sem subcomando → `TemplatesCmd::Menu`; se clap não suporta subcommand-default trivialmente, tratar `Commands::Templates(None)` explicitamente.

**Checkpoint**: `cargo build` compila. `tex-cli templates list` retorna erro "não implementado". Banner continua em stderr. Todos os testes já existentes da spec 001 continuam verdes.

---

## Phase 3: User Story 1 - Listar templates disponíveis (Priority: P1) 🎯 MVP

**Goal**: Usuário consegue rodar `tex-cli templates list` (com `--format=humano` ou `--format=json`) e ver todos os arquivos `.tex` do `paths.templates_dir` com nome, tamanho e data.

**Independent Test**: Config pré-criado apontando pra `TempDir` com 3 arquivos `.tex` + arquivos ruído (`.bib`, `.png`). Rodar `templates list` humano → 3 linhas com nomes ordenados. Rodar `--format=json` → array com 3 objetos válidos por `jq empty`. Testar também dir vazio (`[]`), dir inexistente (exit 22).

### Tests for User Story 1 (write first, ensure they FAIL before implementation) ⚠️

- [X] T005 [P] [US1] Criar `tests/cli_templates_list.rs` com testes de integração usando `assert_cmd` + `assert_fs`:
  - `list_humano_shows_three_templates`: `HOME` isolado, config apontando pra tempdir com 3 `.tex`, valida stdout tem 3 linhas + colunas `NOME`/`TAMANHO`/`MODIFICADO`.
  - `list_json_produces_valid_array`: parse via `serde_json::Value`, `.len() == 3`, cada objeto tem `name`, `path`, `size_bytes`, `modified_at_epoch`.
  - `list_ignores_non_tex_files`: tempdir com `artigo.tex` + `refs.bib` + `img.png` → só `artigo` aparece.
  - `list_ignores_subdirs`: tempdir com `artigo.tex` + subdir `sub/` (que tem `x.tex` dentro) → só `artigo` na raiz.
  - `list_empty_dir_prints_message_and_exits_0`: tempdir vazio → stdout `Nenhum template encontrado…` + exit 0.
  - `list_empty_dir_json_returns_empty_array`: `--format=json` → stdout `[]` + exit 0.
  - `list_templates_dir_missing_exits_22`: config aponta pra path inexistente → exit 22, stderr menciona `config set paths.templates_dir`.
  - `list_missing_config_exits_10`: sem config → exit 10.
- [X] T006 [P] [US1] Unit tests inline em `src/templates.rs` (`#[cfg(test)] mod tests`) cobrindo:
  - `list_templates_returns_only_tex`: dir com `.tex` + `.bib` → só `.tex`.
  - `list_templates_sorts_alphabetically`: ordem determinística por `name` byte-order.
  - `list_templates_missing_dir_returns_templates_dir_missing`: dir não existe → erro.
  - `list_templates_empty_dir_returns_empty_vec`.
  - `template_serializes_to_expected_json`: roundtrip via `serde_json`, campos esperados.

### Implementation for User Story 1

- [X] T007 [P] [US1] Implementar `struct Template` em `src/templates.rs` conforme data-model.md: campos `name: String`, `path: PathBuf`, `size_bytes: u64`, `modified_at_epoch: u64`. Derivar `Debug, Clone, PartialEq, Eq, serde::Serialize`.
- [X] T008 [US1] Implementar `pub fn list_templates(dir: &Path) -> Result<Vec<Template>, TexError>` em `src/templates.rs`: (1) `dir.exists() && dir.is_dir()` → senão `TemplatesDirMissing`; (2) `fs::read_dir` mapeando `PermissionDenied` → `TexError::PermissionDenied`; (3) filtrar entradas `.file_type().is_file() && path.extension() == Some("tex")`; (4) construir `Template` via `metadata()`, com `modified_at_epoch = metadata.modified().and_then(|t| t.duration_since(UNIX_EPOCH)).map(|d| d.as_secs()).unwrap_or(0)`; (5) ordenar `by_key(|t| t.name.clone())`. Depende de T002, T007.
- [X] T009 [P] [US1] Implementar `pub fn render_template_list_humano(templates: &[Template]) -> String` em `src/templates.rs`: cabeçalho `NOME     TAMANHO   MODIFICADO`, uma linha por template com `name` truncado a 20 chars, `size_bytes` alinhado à direita, `modified_at_epoch` formatado como `YYYY-MM-DD HH:MM` via cálculo manual a partir de `SystemTime` (D-01 do research). Se vazio, retorna `"Nenhum template encontrado em {dir}.\n"`. Testado por unit test inline.
- [X] T010 [US1] Implementar `handle_templates_list(format: ShowFormat)` em `src/cli.rs`: (1) `Config::load(config_file_path()?)?`; (2) `list_templates(&cfg.paths.templates_dir)?`; (3) match no `format`: `Humano` → `render_template_list_humano(&templates)` + `print!`; `Json` → `serde_json::to_string_pretty(&templates)? + "\n"` + `print!`; `Toml` → retornar erro `anyhow!("--format=toml não suportado em templates list")` (contrato: só humano e json). Depende de T008, T009. Requer também exportar `ShowFormat` de `src/cli.rs` (já público desde spec 001).
- [X] T011 [US1] Rodar `tests/cli_templates_list.rs` no container. Todos os 8 testes devem passar. Ajustar mensagens/format para casarem com predicates. Rodar `cargo test --lib` para confirmar unit tests de US1 também passam.

**Checkpoint**: MVP funcional. Usuário consegue rodar `tex-cli templates list` (humano e json) sobre um diretório real e ver os templates. `docker run … cargo test --test cli_templates_list` verde.

---

## Phase 4: User Story 2 - Inspecionar conteúdo de um template (Priority: P2)

**Goal**: `tex-cli templates show <nome>` imprime bytes brutos do template no stdout. Aceita nome com ou sem `.tex`.

**Independent Test**: Fixture com `artigo.tex` contendo conteúdo conhecido; rodar `show artigo` e `show artigo.tex` → stdout byte-por-byte igual. Nome inexistente → exit 20.

### Tests for User Story 2 ⚠️

- [X] T012 [P] [US2] Criar `tests/cli_templates_show.rs`:
  - `show_prints_raw_content`: `diff` entre stdout do `show` e o arquivo original é vazio.
  - `show_accepts_name_with_or_without_extension`: `show artigo` e `show artigo.tex` produzem stdout idêntico.
  - `show_missing_template_exits_20`: nome inexistente → exit 20, stderr `Template '<name>' não existe em <path>`.
  - `show_case_sensitive`: dir com `artigo.tex` e `Artigo.tex`; `show artigo` retorna conteúdo de `artigo.tex`, `show Artigo` retorna `Artigo.tex`.
  - `show_missing_config_exits_10`: sem config → exit 10.
  - `show_templates_dir_missing_exits_22`.
- [ ] T013 [P] [US2] Unit tests inline em `src/templates.rs` para `resolve_template`:
  - `resolve_with_extension_finds_file`.
  - `resolve_without_extension_finds_file`.
  - `resolve_case_sensitive`.
  - `resolve_missing_returns_template_not_found`.

### Implementation for User Story 2

- [ ] T014 [P] [US2] Implementar `pub fn resolve_template(dir: &Path, name: &str) -> Result<PathBuf, TexError>` em `src/templates.rs` conforme D-02 do research: (1) se `dir` não existe → `TemplatesDirMissing`; (2) se `name.ends_with(".tex")`, tenta `dir.join(name)`; senão, tenta `dir.join(format!("{name}.tex"))`; (3) `path.exists() && path.is_file()` → retorna `path`, senão `TemplateNotFound { name: name.to_string(), templates_dir: dir.to_path_buf() }`. Depende de T002.
- [ ] T015 [US2] Implementar `pub fn read_template(dir: &Path, name: &str) -> Result<Vec<u8>, TexError>` em `src/templates.rs`: delega a `resolve_template` + `fs::read`, mapeando `PermissionDenied`. Depende de T014.
- [ ] T016 [US2] Implementar `handle_templates_show(name: String)` em `src/cli.rs`: (1) load config; (2) `read_template(&cfg.paths.templates_dir, &name)?`; (3) `io::stdout().write_all(&bytes)?` (bytes brutos, não `println!`). Depende de T015.
- [ ] T017 [US2] Rodar `tests/cli_templates_show.rs` e ajustar. Verificar que `show` não altera mtime do arquivo (mesmo padrão do `config show` da spec 001).

**Checkpoint**: `tex-cli templates show <nome>` funcional. Pipe pra `less` funciona.

---

## Phase 5: User Story 3 - Adicionar novo template (Priority: P3)

**Goal**: `tex-cli templates add <arquivo> [--name <nome>] [--force]` copia o arquivo pro `templates_dir` com validação UTF-8, escrita atômica, permissão `0644`, confirmação de overwrite.

**Independent Test**: `TempDir` fonte + `TempDir` templates_dir; rodar `add /tmp/x.tex` → arquivo novo aparece com conteúdo idêntico, modo `0644`. Testar `--name` (renomeando), `--force` (sobrescreve), binário (exit 21), overwrite sem `--force` em não-TTY (exit 15).

### Tests for User Story 3 ⚠️

- [ ] T018 [P] [US3] Criar `tests/cli_templates_add.rs`:
  - `add_creates_template_from_source`: source no `TempDir` externo, exec `add`, valida arquivo criado com conteúdo idêntico + modo `0644`.
  - `add_with_name_uses_custom_name`: `add /tmp/x.tex --name y` → arquivo salvo como `y.tex`.
  - `add_binary_source_exits_21`: `head -c 100 /dev/urandom` como source → exit 21, stderr contém `UTF-8`, arquivo NÃO criado no destino.
  - `add_overwrite_without_force_non_tty_exits_15`: template já existe → exit 15, arquivo original intocado.
  - `add_force_overwrites_existing`: `--force` → arquivo sobrescrito, stdout menciona "sobrescrito".
  - `add_atomic_leaves_no_partial_file`: teste conceitual — verificar que só existe o arquivo final, sem `.tmp*` órfão.
  - `add_missing_config_exits_10`, `add_templates_dir_missing_exits_22`.
- [ ] T019 [P] [US3] Unit tests inline em `src/templates.rs` para `is_utf8_ok(&[u8])` (helper interno):
  - `is_utf8_ok_true_for_valid_ascii`, `_true_for_valid_utf8_multibyte`, `_false_for_invalid_continuation_byte`.

### Implementation for User Story 3

- [ ] T020 [P] [US3] Implementar `pub fn add_template(dir: &Path, source: &Path, name: Option<&str>, force: bool) -> Result<AddedTemplate, TexError>` em `src/templates.rs` conforme data-model.md: (1) `dir` exists → senão `TemplatesDirMissing`; (2) `fs::read(source)?` (I/O genérico); (3) `str::from_utf8(&bytes).map_err(|e| InvalidUtf8 { source_path: source.to_path_buf(), detail: e.to_string() })?`; (4) `dest_name = name.unwrap_or(source.file_stem().unwrap().to_str().unwrap())`; (5) `dest_path = dir.join(format!("{dest_name}.tex"))`; (6) `overwrote_existing = dest_path.exists()`; (7) `overwrote_existing && !force` → retornar `UserAborted` (o caller decide se prompta antes); (8) `save_atomic_bytes(&bytes, &dest_path, 0o644)`: pattern D-03 do research — `NamedTempFile::new_in(dir)`, `write_all`, `set_permissions(0o644)`, `persist`; (9) retornar `AddedTemplate { name: dest_name, path: dest_path, bytes_written: bytes.len() as u64, overwrote_existing }`. Depende de T002, T007.
- [ ] T021 [US3] Estender `src/interactive.rs`: `pub fn confirm_overwrite_template(name: &str) -> Result<bool, TexError>` usando `inquire::Confirm::new("Sobrescrever template '{name}'?").with_default(false)`, mapeando erros como spec 001 (Ctrl+C → `UserAborted`).
- [ ] T022 [US3] Implementar `handle_templates_add(args: AddArgs)` em `src/cli.rs`: (1) load config; (2) `templates_dir` de `cfg.paths.templates_dir`; (3) checar TTY via `std::io::stdin().is_terminal()`; (4) se overwrite (`dest_path.exists()`) + `!args.force` + TTY → `confirm_overwrite_template` prompt; se responder "não" ou (não TTY sem `--force`) → `UserAborted`; (5) chamar `add_template(...)`; (6) stdout: `Template '{name}' adicionado em {path}.` ou `... sobrescrito em ...` conforme `overwrote_existing`. Depende de T020, T021.
- [ ] T023 [US3] Rodar `tests/cli_templates_add.rs`. Ajustar mensagens até casarem com predicates. Verificar modo `0644` em teste dedicado.

**Checkpoint**: `templates add` completo com validação UTF-8, atomicidade e confirmação.

---

## Phase 6: User Story 4 - Remover template obsoleto (Priority: P3)

**Goal**: `tex-cli templates remove <nome> [--force]` remove o arquivo com confirmação obrigatória (a menos que `--force`).

**Independent Test**: Fixture com `carta.tex`; `remove carta` em não-TTY sem `--force` → exit 15, arquivo persiste. Com `--force` → arquivo some. Nome inexistente → exit 20.

### Tests for User Story 4 ⚠️

- [ ] T024 [P] [US4] Criar `tests/cli_templates_remove.rs`:
  - `remove_without_force_non_tty_exits_15_and_keeps_file`: exit 15, arquivo intacto.
  - `remove_with_force_deletes_file`: exit 0, arquivo não existe, stdout `Template '<name>' removido de <caminho>`.
  - `remove_nonexistent_exits_20`: exit 20, sem alterar diretório.
  - `remove_missing_config_exits_10`, `remove_templates_dir_missing_exits_22`.

### Implementation for User Story 4

- [ ] T025 [P] [US4] Implementar `pub fn remove_template(dir: &Path, name: &str, force: bool) -> Result<PathBuf, TexError>` em `src/templates.rs`: (1) `dir` exists → senão `TemplatesDirMissing`; (2) `path = resolve_template(dir, name)?` (propaga `TemplateNotFound`); (3) se `!force` → retorna `UserAborted` (caller decide se prompta); (4) `fs::remove_file(&path)?` mapeando `PermissionDenied`; (5) retornar `path`. Depende de T014.
- [ ] T026 [US4] Estender `src/interactive.rs`: `pub fn confirm_remove_template(name: &str) -> Result<bool, TexError>` usando `inquire::Confirm::new("Remover template '{name}'?").with_default(false)`.
- [ ] T027 [US4] Implementar `handle_templates_remove(name: String, force: bool)` em `src/cli.rs`: (1) load config; (2) `path = resolve_template(...)?` para saber que existe e retornar exit 20 cedo se não; (3) TTY check via `IsTerminal`; (4) se `!force` + TTY → `confirm_remove_template` prompt; se "não" → `UserAborted` (exit 15); (5) se `!force` + não-TTY → `UserAborted` (exit 15), stderr `Template '<name>' não removido: use --force ou execute em terminal interativo.`; (6) `remove_template(dir, name, true)`; (7) stdout: `Template '{name}' removido de {path}.`. Depende de T025, T026.
- [ ] T028 [US4] Rodar `tests/cli_templates_remove.rs`.

**Checkpoint**: `templates remove` completo com todas as guardas.

---

## Phase 7: User Story 5 - Menu interativo `templates` (Priority: P3)

**Goal**: `tex-cli templates` sem subcomando abre menu inquire one-shot (research D-09) com 5 opções.

**Independent Test**: TTY: menu aparece, "Sair" → exit 0. Sem TTY: exit 1 com mensagem apropriada.

### Implementation for User Story 5

- [ ] T029 [US5] Estender `src/interactive.rs`: `pub enum TemplateMenuAction { List, Show, Add, Remove, Quit }` + `pub fn template_menu() -> Result<TemplateMenuAction, TexError>` usando `inquire::Select`. Adicionar helpers `prompt_template_name(available: &[String]) -> Result<String, TexError>` (Select populado com nomes) e `prompt_source_path() -> Result<PathBuf, TexError>` (Text input).
- [ ] T030 [US5] Implementar `handle_templates_menu()` em `src/cli.rs`: (1) load config; (2) se `!stdin().is_terminal()` → retornar `anyhow!("Menu interativo de templates requer terminal. Use um subcomando explícito: tex-cli templates list|show|add|remove.")` mapeado a exit 1; (3) `template_menu()?`; (4) match na ação, delegando aos handlers existentes (`handle_templates_list`, etc.) com prompts adicionais para nome/source; (5) "Quit" → `Ok(())` (exit 0). Depende de T010, T016, T022, T027, T029.
- [ ] T031 [US5] Smoke test manual no quickstart: rodar `tex-cli templates` num terminal, escolher cada opção uma vez. Documentar como "manual QA step" no PR.

**Checkpoint**: Menu funcional em TTY, degrada gracefully em não-TTY. Todas as user stories entregues.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Verificações transversais, higiene, docs, exemplos, validação de performance.

- [ ] T032 [P] Estender `tests/cli_banner.rs` (existente, spec 001) para cobrir os 4 novos subcomandos: `banner_appears_on_templates_list_stderr`, `banner_appears_on_templates_show_stderr`, `banner_appears_on_templates_add_stderr`, `banner_appears_on_templates_remove_stderr`. Padrão idêntico aos 3 casos originais.
- [ ] T033 [P] Rodar `cargo fmt --all -- --check` e `cargo clippy --all-targets --all-features -- -D warnings` no container. Corrigir violações.
- [ ] T034 [P] Estender `README.md` da raiz: seção "Gerenciar templates" com exemplos dos 4 subcomandos + menu interativo. Atualizar tabela de exit codes para incluir 20/21/22. Sem badges.
- [ ] T035 [P] Criar `examples/templates/` com 1–3 templates standalone (`.tex` simples, ex.: `artigo-basico.tex`, `carta.tex`) inspirados por [Eranot/Unotex](https://github.com/Eranot/Unotex) — **credite explicitamente** o autor. Adicionar `examples/LICENSE-EXAMPLES` com o texto do LPPL 1.3c (`curl -sSL https://www.latex-project.org/lppl/lppl-1-3c.txt > examples/LICENSE-EXAMPLES`) e `examples/README.md` explicando origem, atribuição e como usar (`cp examples/templates/*.tex ~/tex/templates/` ou `tex-cli templates add examples/templates/artigo-basico.tex`).
- [ ] T036 Rodar o quickstart.md do início ao fim dentro do container Docker: build → init → criar templates → `list` → `show` → `add` → `remove`. Documentar tempo total e outcomes no PR.
- [ ] T037 Validar SC-002 (`templates list --format=json` < 100 ms com 100 templates) usando o script do quickstart § 8. Registrar medição no PR.

**Checkpoint**: Feature completa, testada, documentada, com exemplos atribuídos e performance validada.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências.
- **Foundational (Phase 2)**: depende de Setup. **BLOQUEIA todas as user stories.**
- **User Story 1 (P1 / Phase 3)**: depende de Foundational.
- **User Story 2 (P2 / Phase 4)**: depende de Foundational.
- **User Story 3 (P3 / Phase 5)**: depende de Foundational.
- **User Story 4 (P3 / Phase 6)**: depende de Foundational e reaproveita `resolve_template` de US2 — se US2 ainda não entregou, precisa implementar T014 antes.
- **User Story 5 (P3 / Phase 7)**: depende de US1–US4 (menu chama todos os handlers).
- **Polish (Phase 8)**: depende de todas as US completadas na versão MVP alvo.

### User Story Dependencies

- **US1** é o MVP e pode ser entregue sozinho.
- **US2** independente de US1 mas independentemente testável.
- **US3** independente das anteriores.
- **US4** compartilha `resolve_template` (T014) com US2 — se US2 já mergeada, reaproveita; senão, entrega T014 dentro de US4.
- **US5** integrador — precisa de todos os handlers.

### Within Each User Story

- Tests **antes** da implementação (falham → verde após).
- Modelos (`Template`, helpers) antes de handlers.
- Handler CLI é sempre a **última** camada.

### Parallel Opportunities

- Foundational: T003 é [P] (arquivo novo); T002 e T004 tocam arquivos existentes independentes.
- Dentro de cada US: testes de integração (T005, T012, T018, T024) e unit tests inline ([T006], [T013], [T019]) são [P] com implementação inicial.
- Structs (T007), funções puras livre-de-side-effect (T014, T020, T025) são [P] entre si.
- Polish: T032, T033, T034, T035 são [P].

---

## Parallel Example: User Story 1

```bash
# Depois de completar Foundational (T002-T004):
# Rodar em paralelo os testes que serão VALIDADOS depois:
Task: "T005 [P] [US1] Criar tests/cli_templates_list.rs"
Task: "T006 [P] [US1] Unit tests inline em src/templates.rs"

# Depois struct pode ser paralela ao render:
Task: "T007 [P] [US1] Struct Template + serde"
Task: "T009 [P] [US1] render_template_list_humano"

# Depois sequenciais (dependências claras):
Task: "T008 [US1] list_templates(dir)"
Task: "T010 [US1] handle_templates_list em src/cli.rs"
Task: "T011 [US1] Rodar tests/cli_templates_list.rs até verde"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup (Phase 1) → Foundational (Phase 2) → US1 (Phase 3) → parar.
2. **STOP & VALIDATE**: `docker run … cargo test --test cli_templates_list` verde. `tex-cli templates list` funciona.
3. Deploy/demo se ready. Merge se ok.

### Incremental Delivery

1. Foundation → merge.
2. US1 → merge → **MVP** (usuário lista).
3. US2 → merge → usuário inspeciona.
4. US3 → merge → usuário adiciona.
5. US4 → merge → usuário remove.
6. US5 → merge → menu interativo.
7. Polish (Phase 8) → merge.

Para consistência com a spec 001, esta feature também pode ser
entregue em **PR único com commits granulares por T-task**
(disciplina `commit por implementação`), o que preserva o valor de
review sem coordenar 6 PRs sequenciais.

### Parallel Team Strategy

Projeto single-dev — não aplicável. Se um segundo dev entrar após
Foundational: US1..US4 são independentes; pequena coordenação em
`src/templates.rs` para evitar merge conflict.

---

## Notes

- Todas as tarefas devem terminar com o teste correspondente verde antes de avançar.
- Docker container `tex-cli` da spec 001 é reaproveitado sem mudança.
- Nenhuma tarefa toca `src/config.rs` — a struct `Config` permanece
  imutável.
- Commit por task ou por grupo lógico próximo. Preferir commits
  pequenos e verdes.
- Se durante US3–US5 aparecer necessidade de mudar `Template` ou
  `TexError` que quebre US1/US2, revisitar `spec.md` e
  `data-model.md` antes de codar — evitar deriva silenciosa da spec.
