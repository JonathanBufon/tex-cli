# Implementation Plan: Gestão de Templates LaTeX

**Branch**: `002-templates-management` | **Date**: 2026-07-09 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-templates-management/spec.md`

## Summary

Entregar quatro subcomandos do grupo `tex-cli templates` — `list`,
`show`, `add`, `remove` — mais um menu interativo (`tex-cli templates`
sem subcomando) para gerenciar arquivos `.tex` que residem em
`config.paths.templates_dir` (configurado pela spec 001).

Abordagem técnica: novo módulo `src/templates.rs` que abstrai I/O do
diretório de templates, com o `handle_templates_*` correspondente
adicionado a `src/cli.rs` (reaproveita 100% da infra existente:
`Config::load`, `TexError`, banner, `-v`, `inquire`, `assert_cmd`).
Zero mudanças no `src/config.rs` ou na struct `Config`. Escrita atômica
no `add` via `tempfile` + `persist` já provada na spec 001.

Três novos exit codes são introduzidos: `20` (TemplateNotFound), `21`
(InvalidUtf8), `22` (TemplatesDirMissing) — estendem o enum `TexError`
mantendo compat backward.

## Technical Context

**Language/Version**: Rust stable (edição 2021) — mesmo target da
spec 001; MSRV = versão stable à data do commit.

**Primary Dependencies** (todas já na `Cargo.toml`, zero adições):

- `clap` v4 — novo subcomando `templates` com quatro variantes.
- `inquire` — menu do `templates` (sem subcomando) e prompts de
  confirmação (`Confirm` para overwrite/remove).
- `serde` + `serde_json` — saída `--format=json` do `list`.
- `dirs`, `tempfile` — via módulos existentes (`paths`, `config`).
- `tracing` — spans `templates_list`, `templates_show`, `templates_add`,
  `templates_remove` com `#[instrument]`.
- `anyhow` + `thiserror` — mesmo modelo de erro nas fronteiras.
- **Data no JSON**: `modified_at_epoch` em `u64` (segundos desde
  UNIX_EPOCH), calculado com `SystemTime::duration_since` do std —
  **zero nova dep** (research D-01). Scripts convertem para RFC3339
  via `jq '... | strftime(...)'`.

**Storage**: arquivos `.tex` no filesystem em
`config.paths.templates_dir`. Sem banco de dados, sem manifest
side-car. A "listagem" é recomputada por `std::fs::read_dir` em cada
chamada — o número de templates é pequeno (≤100 tipicamente), então
zero cache.

**Assets embutidos**: nenhum. Herda `assets/banner.txt` da spec 001.

**Testing**:

- **Unit tests** (`cargo test --lib`): puros em `src/templates.rs`
  — `list_dir_returns_only_tex`, `resolve_name_accepts_with_or_without_ext`,
  `is_utf8_ok_for_valid_utf8`, `is_utf8_ok_rejects_invalid_bytes`.
- **Integration tests** (`tests/`):
  - `tests/cli_templates_list.rs`: lista humano/json, diretório
    vazio, arquivos não-`.tex` ignorados, subdirs ignorados,
    templates_dir inexistente → exit 22.
  - `tests/cli_templates_show.rs`: nome sem/com `.tex`, template
    ausente → exit 20, template com bytes não-UTF8 → shows bytes
    brutos (não valida no `show`; `add` que valida).
  - `tests/cli_templates_add.rs`: adição limpa, `--name` custom,
    overwrite sem `--force` sem TTY → exit 15, `--force` sobrescreve,
    binário → exit 21, atomicidade (arquivo parcial não permanece).
  - `tests/cli_templates_remove.rs`: sem `--force` sem TTY → exit
    15, `--force` remove, nome inexistente → exit 20.
  - `tests/cli_banner.rs` **existente**: estender para cobrir os
    quatro novos subcomandos.
- **Docker workflow**: mesmo container `tex-cli` da spec 001. Nenhuma
  dep nova de sistema.

**Target Platform**: Linux x86_64 (mesmo da spec 001). O uso de
`std::os::unix::fs::PermissionsExt` no `add` fica atrás do mesmo
`cfg(unix)` já estabelecido.

**Project Type**: CLI — extensão do binário `tex-cli` existente.

**Performance Goals**:

- `templates list --format=json`: < 100 ms wall-clock com até 100
  templates (SC-002). Justificativa: um `read_dir` + N `metadata`
  calls + serialização JSON < 10 KB.
- `templates show`: dominado pelo tamanho do arquivo. Alvo implícito:
  < 100 ms para templates < 1 MB.
- `templates add` / `remove`: < 100 ms wall-clock (SC implícito),
  mesma ordem que `config set` da spec 001.

**Constraints**:

- Escrita do `add` MUST ser atômica (mesmo padrão do `config
  save_atomic`: tempfile-in-same-dir + rename).
- Arquivo gravado pelo `add` MUST ter permissão `0644` (não `0600`)
  para permitir compartilhamento/versionamento via git (FR-013).
- Content de `add` MUST ser validado como UTF-8 antes de qualquer
  I/O de escrita (FR-010).
- Banner emitido em stderr, nunca stdout (FR-019 — herda da spec 001).
- Nenhuma nova dep fora da stack canônica sem justificativa em
  research + Complexity Tracking.

**Scale/Scope**: single-user, single-process. Diretório com até ~100
templates é o caso comum; ≤10 KB por template típico. Sem
concorrência.

## Constitution Check

Gates avaliados contra
[`.specify/memory/constitution.md`](../../.specify/memory/constitution.md)
v1.0.0.

| Princípio | Gate | Status | Justificativa |
|-----------|------|--------|---------------|
| I. Escopo pessoal e não-substituição do LaTeX | Feature ajuda usuário individual a gerar documentos LaTeX mais rápido? Zero reimplementação de LaTeX interno? | ✅ PASS | Só gerencia arquivos `.tex` como blobs de texto — não parseia, não compila, não renderiza. |
| II. Separação Dados/Template/PDF | Feature respeita a separação e não borra fronteiras? | ✅ PASS | Toca apenas a camada Template; não vê JSON, não gera PDF. |
| III. Interface Dupla CLI + Interativo | Cada op é acessível pelo CLI direto (scriptável, exit codes) **e** pelo menu interativo? | ✅ PASS | Todos os 4 subcomandos têm flag CLI direta; `tex-cli templates` (sem subcomando) abre menu inquire. Comportamento não-TTY do `add`/`remove` sem `--force` retorna exit 15 (não trava). |
| IV. Compilador Plugável | Feature respeita isolamento do compilador em `compiler.rs`? | ✅ PASS | Não toca `compiler.engine` nem sabe da existência do compilador. |
| V. Segurança na Manipulação de Arquivos | Escrita atômica, confirmação de overwrite, validação, mensagens humanas? | ✅ PASS | `add` é atômico + confirma overwrite; `remove` exige `--force` ou confirmação; validação UTF-8 antes de gravar; erros como texto humano em pt-BR reaproveitando padrões da spec 001. |

**Stack fixada** (Restrições Técnicas da constitution): todas as libs
listadas em Primary Dependencies são canônicas — zero adição.
Research D-01 evita a nova dep de datas usando Unix epoch. Zero
violações.

## Project Structure

### Documentation (this feature)

```text
specs/002-templates-management/
├── plan.md              # este arquivo
├── spec.md              # spec (checklists PASS)
├── research.md          # Phase 0 output (decisões técnicas D-01..)
├── data-model.md        # Phase 1 output — struct Template + funções
├── quickstart.md        # Phase 1 output — como rodar e testar
├── contracts/
│   └── cli.md           # gramática dos subcomandos + menu
├── checklists/
│   └── requirements.md  # 16/16 PASS
└── tasks.md             # criado depois por /speckit-tasks
```

### Source Code (repository root)

Adota a estrutura declarada na constitution (Restrições Técnicas ›
Estrutura interna esperada). Esta feature adiciona **um único módulo
novo** (`src/templates.rs`) e estende `src/cli.rs` e `src/errors.rs`
sem tocar em outros arquivos.

```text
src/
├── main.rs              # inalterado
├── cli.rs               # + Commands::Templates, TemplatesCmd enum,
│                        #   handle_templates_list/show/add/remove/menu
├── config.rs            # inalterado (Config::load reaproveitado)
├── errors.rs            # + TemplateNotFound, InvalidUtf8,
│                        #   TemplatesDirMissing (exit codes 20/21/22)
├── interactive.rs       # + template_menu(), confirm_remove(),
│                        #   confirm_overwrite_template(), prompt_name()
├── paths.rs             # inalterado
├── lib.rs               # + pub mod templates;
└── templates.rs         # NOVO — list_templates, resolve_template,
                         #   read_template, add_template, remove_template

tests/
├── cli_banner.rs        # estendido: cobre 4 novos subcomandos
├── cli_init.rs          # inalterado
├── cli_config_show.rs   # inalterado
├── cli_config_set.rs    # inalterado
├── cli_templates_list.rs      # NOVO
├── cli_templates_show.rs      # NOVO
├── cli_templates_add.rs       # NOVO
└── cli_templates_remove.rs    # NOVO

docker/
├── Dockerfile           # inalterado
└── README.md            # inalterado

examples/                # NOVO (fora do escopo estrito da spec, mas
├── templates/           #   documentado no PR para trazer templates
│   ├── artigo.tex       #   inspirados por Eranot/Unotex com
│   └── ...              #   atribuição LPPL 1.3c — decisão fora spec
├── LICENSE-EXAMPLES     # LPPL 1.3c
└── README.md            # crédito ao Eranot
```

**Structure Decision**: Single project layout (mesma opção da spec
001). O novo módulo `templates.rs` fica no crate binário existente
(`tex-cli` + lib `tex_cli`). Testes de integração em `tests/`
consomem o binário via `assert_cmd`. Nenhuma reorganização do layout
declarado pela constitution.

## Complexity Tracking

Sem violações da constitution. Research D-01 resolveu a dúvida sobre
formato de data via Unix epoch no JSON + formatação humana manual
(zero nova dep). Tabela deixada em branco por transparência.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| —         | —          | —                                    |
