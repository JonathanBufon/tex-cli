---
description: "Task list for feature 001-config-init-management"
---

# Tasks: Configuração Inicial e Gestão do Config do Tex

**Input**: Design documents from `/specs/001-config-init-management/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: Included — plan.md e contracts/cli.md declaram contratos observáveis explícitos que exigem verificação automatizada (banner em stderr, permissões `0600`, exit codes, atomicidade).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing. MVP = User Story 1 (init) — pode ser entregue sozinho e já produz um `~/.config/tex/config.toml` válido.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- File paths são relativos à raiz do repo

## Path Conventions

- Rust code: `src/`
- Integration tests: `tests/` (cada arquivo é um binário de teste)
- Assets: `assets/`
- Docker: `docker/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Bootstrap do projeto Rust + Docker.

- [X] T001 Criar `Cargo.toml` na raiz do repo declarando o binário `tex-cli` (`[[bin]] name = "tex-cli" path = "src/main.rs"`), edition = "2021", e as dependências canônicas da constitution: `clap` (v4, feature `derive`), `inquire`, `serde` (feature `derive`), `serde_json`, `toml`, `dirs`, `tempfile`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber` (feature `fmt`), `which`. `[dev-dependencies]`: `assert_cmd`, `assert_fs`, `predicates`.
- [X] T002 [P] Criar `docker/Dockerfile` (base `debian:bookworm-slim`, instalar `build-essential curl ca-certificates tectonic pkg-config libssl-dev`, instalar Rust stable via rustup, `WORKDIR /src`, `CMD ["cargo","test","--all"]`). Conteúdo conforme D-08 do research.md.
- [X] T003 [P] Criar `.gitignore` na raiz cobrindo `target/`, `Cargo.lock` (mantém — projeto é binário, não lib), `**/*.rs.bk`.
- [X] T004 [P] Criar `docker/README.md` documentando `docker build -t tex-cli -f docker/Dockerfile .`, `docker run --rm --name tex-cli -v $(pwd):/src -w /src tex-cli cargo test --all`, convenção `docker cp tex-cli:<src> /home/jonathan/Downloads/dockers-sharefiles/<dst>` (referência memory `workflow-docker-testing`).

**Checkpoint**: `cargo check` passa (sem código de produção ainda, só dependências resolvem).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Módulos e infra que TODAS as user stories dependem. Nada de user story pode iniciar até esta fase estar completa.

- [X] T005 [P] Implementar `src/errors.rs`: enum `TexError` via `thiserror` cobrindo `ConfigMissing`, `ConfigCorrupted { detail }`, `UnknownKey { key, accepted }`, `InvalidBoolValue { key, value }`, `PermissionDenied { path }`, `UserAborted`, `HomeDirUnavailable`, `EngineBinaryMissing { engine }`, `Io(std::io::Error)`. `impl Display` humano em português (sem "backtrace" ou termos técnicos). Função pública `TexError::exit_code() -> i32` mapeando: ConfigMissing→10, ConfigCorrupted→11, UnknownKey→12, InvalidBoolValue→13, PermissionDenied→14, UserAborted→15, Io→1 (genérico), HomeDirUnavailable→1. Conforme D-09 do research.md.
- [X] T006 [P] Implementar `src/paths.rs`: função pública `pub fn config_file_path() -> Result<PathBuf, TexError>` retornando `dirs::config_dir()/tex/config.toml` (erro `HomeDirUnavailable` se nulo); função `pub fn expand_user_path(raw: &str) -> Result<PathBuf, TexError>` que expande `~` e `~/…` via `dirs::home_dir()`, canonicaliza caminhos relativos contra `env::current_dir()` (sem exigir que o path exista) e normaliza `.`/`..`. Unit tests inline (`#[cfg(test)] mod tests`) cobrindo: `~`, `~/foo`, `./bar`, `/abs`, path com `..`.
- [X] T007 Implementar `src/main.rs`: (a) `const BANNER: &str = include_str!("../assets/banner.txt");` (b) inicializar `tracing_subscriber` com filtro derivado do `-v` (D-05 do research), writer = stderr; (c) `eprint!("{BANNER}")` como primeira saída (antes do dispatch); (d) parse do `Cli` (definido em T008); (e) dispatch para handler (por ora, cada variante retorna erro "não implementado" para fechar o compile); (f) `main` retorna `ExitCode`, usando `TexError::exit_code()` para erros conhecidos, `1` para outros. Depende de T005.
- [X] T008 Implementar esqueleto de `src/cli.rs`: struct `Cli` (`#[derive(Parser)]`) com `#[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)] verbose: u8`, subcomando `command: Commands`. Enum `Commands` com variantes `Init`, `Config(ConfigCmd)`. Enum `ConfigCmd` com `Show { #[arg(long, default_value = "humano")] format: ShowFormat }` e `Set { key: String, value: String }`. Enum `ShowFormat { Humano, Json, Toml }` (`#[derive(clap::ValueEnum)]`). Handlers como `fn handle_init(...) -> Result<()> { Err(anyhow!("não implementado")) }` etc. — serão substituídos nas user stories.
- [X] T009 Declarar módulos em `src/lib.rs` (novo arquivo): `pub mod cli; pub mod config; pub mod errors; pub mod interactive; pub mod paths;`. `src/config.rs` e `src/interactive.rs` podem existir vazios/stubs neste ponto para satisfazer `pub mod`. Ajustar `Cargo.toml` para `[lib] name = "tex_cli" path = "src/lib.rs"` além do `[[bin]]`, permitindo que `tests/` importem os módulos internos para unit tests avançados.

**Checkpoint**: `cargo build` compila. `tex-cli init` executa mas retorna erro "não implementado". Banner aparece em stderr.

---

## Phase 3: User Story 1 - Primeira configuração após instalar o Tex (Priority: P1) 🎯 MVP

**Goal**: Um usuário novo consegue rodar `tex-cli init`, responder três prompts (`templates_dir`, `output_dir`, `compiler.engine`), e ter o `~/.config/tex/config.toml` criado com permissão `0600` de forma atômica.

**Independent Test**: Em ambiente limpo (sem `~/.config/tex/`), rodar `tex-cli init` com entradas válidas via `assert_cmd::Command::write_stdin(...)` ou script; validar que o arquivo é criado, contém as respostas, tem modo `0600` e sobreviver o teste de crash simulado (via `SIGKILL` mid-write, opcional).

### Tests for User Story 1 (write first, ensure they FAIL before implementation) ⚠️

- [X] T010 [P] [US1] Criar `tests/cli_init.rs` com testes de integração usando `assert_cmd` + `assert_fs`:
  - `init_creates_config_with_valid_answers`: `HOME` isolado (`assert_fs::TempDir`), envia respostas via stdin, verifica que arquivo existe no path esperado, contém as três respostas, tem `modo & 0o777 == 0o600`.
  - `init_refuses_overwrite_without_confirmation`: cria config pré-existente, roda init, responde "não" à confirmação, arquivo original permanece byte-a-byte igual.
  - `init_prompts_to_create_missing_templates_dir`: informa dir inexistente, responde "sim", verifica que dir foi criado.
  - `init_warns_but_persists_when_tectonic_missing`: usa `env("PATH","/tmp/empty")`, verifica que stderr tem warning `'tectonic' não foi encontrado`, mas arquivo é gravado com sucesso.
  - `init_banner_in_stderr_never_stdout`: roda init, captura stdout e stderr separados, verifica banner presente em stderr, ausente em stdout.
- [ ] T011 [P] [US1] Unit tests em `src/config.rs` (`#[cfg(test)]`): (a) roundtrip serde de `Config` completo; (b) `deny_unknown_fields` rejeita campo extra; (c) `save_atomic` sobre path inexistente cria; (d) `save_atomic` sobre path existente substitui atomicamente (verificar via read imediatamente após); (e) permissão do arquivo final é `0o600` (via `std::os::unix::fs::PermissionsExt`).

### Implementation for User Story 1

- [X] T012 [P] [US1] Implementar structs em `src/config.rs` conforme `data-model.md`: `Config`, `PathsConfig`, `CompilerConfig`, `BehaviorConfig`. Aplicar `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` e `#[serde(deny_unknown_fields)]` em cada struct filha. `PathsConfig` usa `PathBuf` (com `#[serde(serialize_with = ..., deserialize_with = ...)]` se necessário para produzir strings TOML absolutas — validar comportamento default de `toml` crate primeiro).
- [ ] T013 [US1] Implementar `Config::new_from_prompts(paths_templates_dir, paths_output_dir, compiler_engine) -> Config` em `src/config.rs` aplicando defaults do `data-model.md` (`keep_tex=true`, `keep_logs=true`, `ask_output_path_every_time=false`).
- [ ] T014 [US1] Implementar `Config::save_atomic(&self, target: &Path) -> Result<(), TexError>` em `src/config.rs`: criar dirs pais com `fs::create_dir_all`, `tempfile::NamedTempFile::new_in(target.parent())`, escrever `toml::to_string_pretty(self)` no tmpfile, `set_permissions(Permissions::from_mode(0o600))` no tmpfile ANTES do persist, `tempfile.persist(target)` (conforme D-02 do research). Depende de T012.
- [ ] T015 [P] [US1] Implementar `src/interactive.rs`: função pública `pub fn run_init_prompts() -> Result<InitAnswers, TexError>` retornando struct com `templates_dir: PathBuf`, `output_dir: PathBuf`, `engine: String`. Sequência: `inquire::Text::new("Diretório de templates LaTeX:")`, `inquire::Text::new("Diretório padrão de saída dos PDFs:")`, `inquire::Select::new("Compilador LaTeX:", vec!["tectonic","latexmk","pdflatex","xelatex","lualatex"]).with_starting_cursor(0)`. Cada path passa por `paths::expand_user_path`. `Ctrl+C`/abort do inquire → `TexError::UserAborted`. Depende de T006.
- [ ] T016 [US1] Implementar `pub fn confirm_overwrite(path: &Path) -> Result<bool, TexError>` e `pub fn confirm_create_dir(path: &Path) -> Result<bool, TexError>` em `src/interactive.rs` usando `inquire::Confirm` com default `No`.
- [ ] T017 [US1] Implementar handler `handle_init` em `src/cli.rs`: (1) resolve `config_path` via `paths::config_file_path()`; (2) se existe, `confirm_overwrite` — falso → retorna `TexError::UserAborted`; (3) `run_init_prompts()`; (4) para cada dir (templates, output) que não existe, `confirm_create_dir` e `fs::create_dir_all` se sim; (5) `which::which(engine)` → emite warning em stderr se ausente (nunca aborta); (6) se engine ≠ "tectonic" → warning "não suportado ainda"; (7) `Config::new_from_prompts(...).save_atomic(&config_path)`; (8) print em stdout `Config gravado em: {config_path}`. Depende de T005, T007, T012–T016.
- [ ] T018 [US1] Rodar `tests/cli_init.rs` e verificar que todos passam. Ajustar mensagens de erro/warning para casarem com `predicates::str::contains(...)` dos testes.

**Checkpoint**: MVP funcional. Usuário consegue rodar `tex-cli init` e produzir um config válido. `docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --test cli_init` passa.

---

## Phase 4: User Story 2 - Inspeção do config atual (Priority: P2)

**Goal**: `tex-cli config show` exibe o config com formato padrão humano, com fallback para `--format=json` (script) e `--format=toml` (re-serialização).

**Independent Test**: Com config pré-criado (fixture), rodar `tex-cli config show`, `tex-cli config show --format=json` e `tex-cli config show --format=toml`; validar saídas em stdout, ausência de mtime alterada, exit 0. Também validar exit 10 sem config e exit 11 com TOML corrompido.

### Tests for User Story 2 ⚠️

- [ ] T019 [P] [US2] Criar `tests/cli_config_show.rs`:
  - `show_humano_prints_all_sections`: config válido → stdout contém "paths", "compiler", "behavior".
  - `show_format_json_is_valid_and_pipeable`: parseia stdout como `serde_json::Value`, verifica `.compiler.engine == "tectonic"`.
  - `show_format_toml_roundtrips`: parseia stdout como TOML, semanticamente igual ao arquivo original.
  - `show_missing_config_exits_10`: sem config → exit `10`, stderr contém "Rode 'tex-cli init'".
  - `show_corrupted_config_exits_11`: grava TOML inválido → exit `11`, stderr contém "inválido".
  - `show_does_not_modify_mtime`: mtime antes == mtime depois.

### Implementation for User Story 2

- [ ] T020 [P] [US2] Implementar `Config::load(path: &Path) -> Result<Config, TexError>` em `src/config.rs`: `fs::read_to_string` → `TexError::ConfigMissing` se `NotFound`, `TexError::PermissionDenied` se `PermissionDenied`; `toml::from_str::<Config>` → `TexError::ConfigCorrupted { detail: e.to_string() }` em erro (extrair a mensagem humana do `toml::de::Error`, evitando debug format).
- [ ] T021 [P] [US2] Implementar `fn render_humano(c: &Config) -> String` em `src/config.rs`: agrupa por seção, alinha `chave = valor` com padding fixo. Sem cores nesta v1.
- [ ] T022 [US2] Estender handler `handle_config_show(format: ShowFormat)` em `src/cli.rs`: `Config::load(config_file_path()?)?`, então match no format: `Humano` → `println!("{}", render_humano(&c))`, `Json` → `println!("{}", serde_json::to_string_pretty(&c)?)`, `Toml` → `println!("{}", toml::to_string_pretty(&c)?)`. Depende de T020, T021.
- [ ] T023 [US2] Rodar `tests/cli_config_show.rs` e ajustar.

**Checkpoint**: `tex-cli config show` funcional em três formatos. Scripts CI podem consumir `--format=json`.

---

## Phase 5: User Story 3 - Alteração pontual de configuração (Priority: P3)

**Goal**: `tex-cli config set <chave-dotted> <valor>` altera exatamente uma chave, com validação estrita de tipo, sem tocar em outras chaves.

**Independent Test**: Config pré-existente → `config set compiler.keep_tex false` → verificar que apenas essa chave mudou (hash das outras linhas TOML idênticas ou comparação semântica campo a campo).

### Tests for User Story 3 ⚠️

- [ ] T024 [P] [US3] Criar `tests/cli_config_set.rs`:
  - `set_boolean_flips_only_target_key`: config semanticamente comparado antes/depois — apenas a chave alvo mudou.
  - `set_path_expands_tilde`: `config set paths.templates_dir '~/x'` → arquivo contém `/home/<user>/x` absoluto.
  - `set_unknown_key_exits_12_and_lists_accepted`: `config set foo.bar baz` → exit `12`, stderr lista as 6 chaves canônicas.
  - `set_invalid_bool_exits_13_and_leaves_file_intact`: `config set compiler.keep_tex maybe` → exit `13`, hash do arquivo idêntico.
  - `set_missing_config_exits_10`: sem config → exit `10`.
  - `set_engine_non_tectonic_warns_but_persists`: `config set compiler.engine latexmk` → exit `0`, stderr contém warning.
  - `set_path_nonexistent_dir_warns_but_persists`: exit `0`, stderr contém "não existe no momento".
- [ ] T025 [P] [US3] Unit tests para `ConfigKey` em `src/config.rs`: `FromStr` aceita exatamente as 6 canônicas, rejeita variantes ("templates_dir" sem prefixo, uppercase, alias); `Display` retorna canonical; `ConfigKey::ALL.len() == 6`.

### Implementation for User Story 3

- [ ] T026 [P] [US3] Implementar enum `ConfigKey` em `src/config.rs` conforme `data-model.md`: variantes exaustivas, `impl FromStr` com match exato (sem `to_lowercase`, sem trim), erro `TexError::UnknownKey { key, accepted: ConfigKey::ALL.iter().map(|k| k.canonical()).collect() }`, `impl Display` via `canonical()`, `const ALL: &[ConfigKey]`.
- [ ] T027 [US3] Implementar `Config::apply(&mut self, key: ConfigKey, raw_value: &str) -> Result<AppliedChange, TexError>` em `src/config.rs`: match na chave, parseia `raw_value` conforme tipo (`bool` estrito aceita apenas "true"/"false"; `path` via `paths::expand_user_path`; `string` verifica não-vazio). Retorna struct `AppliedChange { key, normalized_value, warnings: Vec<String> }`. Warnings preenchidos para engine ≠ tectonic (não suportado), engine sem binário no PATH, e path a diretório inexistente. Depende de T012, T026.
- [ ] T028 [US3] Estender handler `handle_config_set(key: String, value: String)` em `src/cli.rs`: (1) `Config::load` (exit 10 se ausente); (2) `ConfigKey::from_str(&key)?` (exit 12); (3) `config.apply(key, &value)?` (exit 13 se bool inválido); (4) emitir warnings retornados em stderr; (5) `config.save_atomic(&config_path)?`; (6) stdout `Config atualizado: {key} = {normalized_value}`. Depende de T014, T020, T026, T027.
- [ ] T029 [US3] Rodar `tests/cli_config_set.rs` e ajustar mensagens.

**Checkpoint**: Feature completa — `init`, `config show` e `config set` funcionais. Todos os testes de integração das três user stories passam.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Verificações transversais, higiene de código, documentação mínima. Independente das user stories mas exige que todas estejam completas.

- [ ] T030 [P] Criar `tests/cli_banner.rs` validando contratos transversais do banner (`contracts/cli.md` › seção Banner):
  - `banner_appears_on_init_stderr`
  - `banner_appears_on_config_show_stderr`
  - `banner_appears_on_config_set_stderr`
  - `banner_never_appears_on_stdout_for_any_subcommand` — captura stdout puro e verifica ausência de `TEX CLI` (nem substring, nem qualquer linha do banner).
  - `banner_content_matches_asset` — lê `assets/banner.txt` no host, compara com stderr do binário.
- [ ] T031 [P] Rodar `cargo fmt --all -- --check` e `cargo clippy --all-targets --all-features -- -D warnings`. Corrigir violações. Adicionar a comando padrão do dev flow no `docker/README.md`.
- [ ] T032 [P] Escrever `README.md` na raiz (substituindo o atual de 2 linhas): título, descrição de 2 parágrafos, seção "Instalação" (`cargo install --path .`), seção "Uso básico" com os três comandos, link para `specs/001-config-init-management/quickstart.md` e para a constitution. Sem badges (evitar YAGNI).
- [ ] T033 Rodar o quickstart.md do início ao fim dentro do container Docker: `docker build`, `docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all`. Verificar que 100% dos testes passam. Documentar tempo total no PR/commit final.
- [ ] T034 Validar SC-002 (`config show < 100ms`) e SC-006 (script CI não-interativo lê e altera) usando os binários compilados. Registrar tempo no commit final ou em nota no quickstart.

**Checkpoint**: Feature pronta para code review e merge.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências — pode começar imediatamente.
- **Foundational (Phase 2)**: depende de Setup completo. **BLOQUEIA todas as user stories.**
- **User Story 1 (P1 / Phase 3)**: depende de Foundational.
- **User Story 2 (P2 / Phase 4)**: depende de Foundational. Compartilha `Config::save_atomic` (T014) com US1 — se US1 já entregou, reaproveita; senão, precisa cumprir T012 + T014 primeiro.
- **User Story 3 (P3 / Phase 5)**: depende de Foundational. Compartilha `Config::save_atomic` e `Config::load` — se US1 e US2 já entregues, reaproveita. Precisa também de T012.
- **Polish (Phase 6)**: depende de todas as user stories na versão MVP alvo (mínimo US1; ideal US1+US2+US3).

### User Story Dependencies

- **US1** é o MVP (pode ser entregue sozinho).
- **US2** é logicamente independente mas herda `Config` struct e `Config::save_atomic` (não estritamente necessário para `show`, mas o teste `save_atomic` de US1 valida o load implicitamente).
- **US3** herda `Config::load` de US2 (mas pode reimplementar rapidamente se entregue antes) e a lógica de `save_atomic` de US1.

### Within Each User Story

- Tests escritos **antes** da implementação (falham → verde após).
- Modelos antes de serviços; serviços antes de handlers do CLI.
- Handler CLI é sempre a **última** camada dentro de uma US.

### Parallel Opportunities

- Todas as tarefas Setup marcadas [P] (T002, T003, T004) rodam em paralelo depois que T001 estiver pronto.
- Foundational: T005 e T006 são [P] entre si; T007, T008, T009 têm dependência entre si mas não com T005/T006.
- Dentro de cada user story: os testes [P] (T010+T011, T019, T024+T025) são independentes dos módulos de código.
- Dentro de US1: T012 [P] e T015 [P] podem ser feitos em paralelo antes de T013/T014/T016/T017.
- Polish: T030, T031, T032 são [P].

---

## Parallel Example: User Story 1

```bash
# Depois de completar Foundational (T005-T009):
# Rodar em paralelo todos os testes que serão VALIDADOS depois:
Task: "T010 [P] [US1] Criar tests/cli_init.rs com testes de integração"
Task: "T011 [P] [US1] Unit tests inline em src/config.rs para Config e save_atomic"

# Rodar em paralelo os stubs independentes:
Task: "T012 [P] [US1] Implementar structs em src/config.rs (Config, PathsConfig, CompilerConfig, BehaviorConfig)"
Task: "T015 [P] [US1] Implementar src/interactive.rs (run_init_prompts)"

# Depois sequenciais (dependências claras):
Task: "T013 [US1] Config::new_from_prompts"
Task: "T014 [US1] Config::save_atomic"
Task: "T016 [US1] confirm_overwrite / confirm_create_dir"
Task: "T017 [US1] handle_init em src/cli.rs"
Task: "T018 [US1] Rodar tests/cli_init.rs até verde"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup (Phase 1) → Foundational (Phase 2) → US1 (Phase 3) → parar.
2. **STOP & VALIDATE**: `docker run … cargo test --test cli_init` verde. Config real gerado, permissão `0600`.
3. Deploy/demo se ready. Merge se ok.

### Incremental Delivery

1. Foundation → merge.
2. US1 → merge → **MVP**.
3. US2 → merge → usuário pode inspecionar.
4. US3 → merge → usuário pode editar via CLI.
5. Polish (Phase 6) → merge.

### Parallel Team Strategy

Como é projeto single-dev, não aplicável — mas se um segundo dev entrar após Foundational, US2 e US3 são independentes (pequena coordenação em `src/config.rs` para evitar merge conflict).

---

## Notes

- Todas as tarefas devem terminar com o teste correspondente verde antes de avançar. Se um teste da US1 falhar, o loop é: ajustar código → rerodar → repetir.
- `docker/Dockerfile` pode ser buildado 1 vez e reusado — não precisa rebuild a cada tarefa.
- Nenhuma tarefa toca `src/templates.rs`, `src/renderer.rs` ou `src/compiler.rs` — esses módulos serão criados nas specs 2 e 3.
- Commit por task ou por grupo lógico. Preferir commits pequenos e verdes.
- Se durante US2 ou US3 aparecer necessidade de mudar `Config`/`ConfigKey` que quebre US1, revisitar `spec.md` e `data-model.md` antes de codar — evitar deriva silenciosa da spec.
