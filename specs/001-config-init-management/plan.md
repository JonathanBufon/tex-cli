# Implementation Plan: Configuração Inicial e Gestão do Config do Tex

**Branch**: `001-config-init-management` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-config-init-management/spec.md`

## Summary

Entregar três subcomandos do binário `tex-cli` — `init`, `config show` e
`config set` — que criam e mantêm o arquivo TOML em
`~/.config/tex/config.toml`. `init` é interativo (`inquire`); os outros
dois são scriptáveis via `clap`, respeitando dotted-path canônico,
escrita atômica com permissões `0600`, flag `--format=<humano|json|toml>`
no `show` e verbosidade `-v/-vv/-vvv` em todos.

Abordagem técnica: binário Rust único com módulos separados por
responsabilidade (`cli`, `config`, `paths`, `interactive`, `errors`),
serialização via `toml` + `serde`, escrita atômica via `tempfile` +
`rename`, verificação de binário externo via `which`. Testes de
integração que checam presença de `tectonic` rodam em container Debian.

## Technical Context

**Language/Version**: Rust stable (edição 2021, MSRV = versão stable
mais recente à data do commit; verificado no CI).

**Primary Dependencies**:

- `clap` (v4, feature `derive`) — parsing de subcomandos, args globais
  (`-v/--verbose`), grupos mutuamente exclusivos.
- `inquire` — prompts sequenciais do `init` (texto para paths,
  select para engine, confirm para overwrite/create-dir).
- `serde` (feature `derive`) + `toml` — desserialização/serialização
  do arquivo `config.toml`.
- `serde_json` — apenas para a saída `--format=json` do `config show`
  (não para storage).
- `dirs` — resolver `~/.config/tex/` de forma que respeite XDG.
- `tempfile` — `NamedTempFile::new_in(target_dir)` para persistir a
  escrita atômica no mesmo filesystem.
- `anyhow` — tipo de erro na fronteira (`main`, handlers dos
  subcomandos), com contexto (`.context("…")`).
- `thiserror` — enum `TexError` central em `errors.rs`, com `Display`
  humano e conversão de erros internos.
- `tracing` + `tracing-subscriber` — inicializados em `main.rs` com
  filtro derivado de `-v` (mapeamento: 0=`warn`, 1=`info`, 2=`debug`,
  3+=`trace`).
- `which` — validação da presença do binário do compilador escolhido.

**Storage**: arquivo texto TOML em `~/.config/tex/config.toml`.
Permissão `0600` no arquivo final e no tmpfile intermediário. Sem
banco de dados, sem estado externo.

**Assets embutidos**: `assets/banner.txt` (ASCII "TEX CLI" em block
letters Unicode, sem bordas) — incorporado ao binário via
`include_str!("../assets/banner.txt")` e emitido em stderr no início
de todo subcomando (FR-025).

**Testing**:

- **Unit tests** (`cargo test`) — puros: `paths::expand`, `config::parse`,
  `config::apply_dotted_key`, roundtrip serde. Rodam no host ou no
  container indistintamente.
- **Integration tests** (`tests/`) — exercitam CLI real com `assert_cmd`
  e ambientes temporários (`assert_fs` ou `tempfile`). Duas variações
  quanto ao ambiente:
  - "com tectonic": container `tex-cli` (Debian) tem `tectonic` no PATH →
    valida ausência de warning em `init`.
  - "sem tectonic": mesmo container com `PATH=/tmp/empty` → valida que
    `init` emite warning e ainda persiste config.
- **Docker workflow**: `docker/Dockerfile` (base `debian:bookworm-slim`)
  com toolchain Rust + `tectonic`. Testes rodados com
  `docker run --rm -v $(pwd):/src tex-cli cargo test`. PDFs de futuras
  features são copiados via `docker cp tex-cli:<path>
  /home/jonathan/Downloads/dockers-sharefiles/`.

**Target Platform**: Linux x86_64 (primário). Nenhuma dependência de
API Linux-específica além das que `dirs` já abstrai; `unix` cfg-gate
apenas para `PermissionsExt::set_mode(0o600)` (documentado no código).

**Project Type**: CLI — single binário Rust (`tex-cli`).

**Performance Goals**:

- `tex-cli config show`: < 100 ms wall-clock em Linux comum
  (SC-002). Justificativa: leitura + parse de <1 KB TOML + render
  humano. Sem I/O adicional.
- `tex-cli init` (do disparo à mensagem final): < 30 s incluindo
  digitação dos três prompts (SC-001). CPU do processo é irrelevante;
  tempo dominante é humano.
- `tex-cli config set`: < 100 ms wall-clock, mesmo racional do `show`
  mais uma escrita atômica.

**Constraints**:

- Config file MUST ser gravado com `0600` (FR-003b).
- Escrita MUST ser atômica via tempfile no mesmo diretório + rename
  (FR-003a).
- Chaves de `config set` MUST usar dotted-path canônico, sem aliases
  (FR-014).
- `config show` MUST suportar `--format=<humano|json|toml>` (FR-010).
- Erros MUST ir para stderr como texto humano; stdout reservado a
  saída utilizável (FR-021, FR-024).
- Flag global `-v/--verbose` (repetível) presente em todos os
  subcomandos (FR-024).
- Banner "TEX CLI" MUST ser emitido em stderr no início de todo
  subcomando; nunca em stdout (FR-025).

**Scale/Scope**: single-user, single-process, config <10 chaves,
<1 KB. Sem concorrência, sem escala.

## Constitution Check

Gates avaliados contra [`.specify/memory/constitution.md`](../../.specify/memory/constitution.md) v1.0.0.

| Princípio | Gate | Status | Justificativa |
|-----------|------|--------|---------------|
| I. Escopo pessoal e não-substituição do LaTeX | Feature ajuda usuário individual a gerar documentos LaTeX mais rápido? Zero reimplementação de mecanismos internos do LaTeX? | ✅ PASS | Apenas persistência de preferências; não toca em LaTeX interno. |
| II. Separação Dados/Template/PDF | Feature respeita a separação e não borra as fronteiras? | ✅ PASS | Não manipula JSON, template ou PDF; só o próprio config. |
| III. Interface Dupla CLI + Interativo | `init` interativo (`inquire`) **e** demais subcomandos scriptáveis (`clap`, exit codes, `--format=json`)? | ✅ PASS | `init` → inquire; `config show`/`set` → clap puro, sem prompts. |
| IV. Compilador Plugável | Chave `compiler.engine` isolada e nome futuro aceito com aviso? | ✅ PASS | `config set compiler.engine …` aceita nomes fora de tectonic com warning; nenhuma decisão do compilador vaza para outros módulos. |
| V. Segurança na Manipulação de Arquivos | Escrita atômica, permissão `0600`, confirmação de overwrite, validação, mensagens humanas? | ✅ PASS | FR-003a/003b/006/016–021/024 cobrem todos os pontos. |

**Stack fixada** (Restrições Técnicas): todas as libs da tabela
canônica são usadas; nenhuma adição fora dela. Zero violações
justificam Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/001-config-init-management/
├── plan.md              # este arquivo
├── spec.md              # spec (já com Clarifications)
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output — struct Config
├── quickstart.md        # Phase 1 output — como rodar e testar
├── contracts/
│   └── cli.md           # gramática dos subcomandos (contrato de UX)
├── checklists/
│   └── requirements.md  # criado por /speckit-specify
└── tasks.md             # criado depois por /speckit-tasks
```

### Source Code (repository root)

Adota a estrutura declarada na constitution (Restrições Técnicas ›
Estrutura interna esperada). Esta feature toca apenas o subconjunto
listado abaixo — módulos como `templates.rs`, `renderer.rs` e
`compiler.rs` ficam fora do escopo e virão nas próximas specs.

```text
Cargo.toml
Cargo.lock
assets/
└── banner.txt           # ASCII "TEX CLI" embutido via include_str!
src/
├── main.rs              # entrypoint, tracing init, emite banner, delega para cli
├── cli.rs               # clap derive: Cli, Commands, ConfigCmd
├── config.rs            # struct Config, load/save atômico, apply_key
├── paths.rs             # resolve config path, expande ~ e relativos
├── interactive.rs       # prompts inquire do `init`
└── errors.rs            # TexError (thiserror), Display humano

tests/
├── cli_config_show.rs   # integração: config show em cenários variados
├── cli_config_set.rs    # integração: config set (dotted keys, boolean, path)
├── cli_init.rs          # integração: init (com/sem overwrite, com/sem tectonic)
└── cli_banner.rs        # integração: banner em stderr, nunca em stdout

docker/
├── Dockerfile           # debian:bookworm-slim + rust + tectonic
└── README.md            # como buildar e rodar testes no container
```

**Structure Decision**: Single project layout (Option 1 do template).
CLI puro, sem front/back split. Testes de integração em `tests/`
consomem o binário compilado via `assert_cmd`. Docker isolado em
`docker/` — não interfere com o build normal (`cargo build`).

## Complexity Tracking

Sem violações da constitution. Tabela deixada em branco por
transparência.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| —         | —          | —                                    |
