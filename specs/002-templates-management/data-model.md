# Phase 1 — Data Model: Gestão de Templates LaTeX

**Feature**: `002-templates-management`
**Date**: 2026-07-09

Modelo de dados **interno** desta feature: a entidade principal é
`Template`, uma view stateless do arquivo `.tex` no disco. Não há
persistência dedicada — a informação vive nos próprios arquivos e nos
metadados do sistema de arquivos.

Cross-reference: reaproveita 100% do modelo da spec 001 (`Config`,
`ConfigKey`, `TexError`) sem modificações estruturais além da adição
de três variantes ao `TexError` (D-08 do research).

---

## Entidade: `Template`

View não-persistida de um arquivo `.tex` no `paths.templates_dir`.
Construída sob demanda por `list_templates` e serializada no
`--format=json` do `templates list`.

### Struct Rust

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Template {
    /// Nome canônico do template (basename sem `.tex`).
    pub name: String,

    /// Caminho absoluto do arquivo no disco.
    pub path: PathBuf,

    /// Tamanho em bytes (do `Metadata::len()`).
    pub size_bytes: u64,

    /// Última modificação em segundos desde UNIX_EPOCH.
    /// Ver research D-01 para justificativa.
    pub modified_at_epoch: u64,
}
```

### Regras de construção

| Regra | Aplicação | Referência |
|-------|-----------|------------|
| `name` = `path.file_stem()` como String UTF-8 | listing | FR-003 |
| Apenas arquivos com extensão exatamente `.tex` (case-sensitive) contam | listing | FR-001, Assumption "só `.tex`" |
| Subdiretórios são ignorados na v1 | listing | Edge Case "subdirs ignorados" |
| `size_bytes` = `Metadata::len()` | listing | FR-003 |
| `modified_at_epoch` = `Metadata::modified().duration_since(UNIX_EPOCH).as_secs()`; falha silente → `0` (edge case raro) | listing | Research D-01 |
| Ordem de saída: alfabética por `name` (POSIX byte order) | listing | Determinismo pra scripts |

### Serialização JSON

```json
{
  "name": "artigo",
  "path": "/home/user/tex/templates/artigo.tex",
  "size_bytes": 4321,
  "modified_at_epoch": 1741550055
}
```

Uma lista de templates é um array JSON desses objetos. Diretório vazio
retorna `[]` (nunca `null`) — SC-006.

### Formato humano

Renderizado por `render_template_list_humano(&[Template])`:

```text
NOME               TAMANHO   MODIFICADO
artigo               4321  2026-07-09 14:30
carta                2100  2026-05-01 09:00
relatorio_2026       9876  2026-07-08 22:15
```

- `NOME` truncado a 20 caracteres se necessário (ellipsis).
- `TAMANHO` em bytes, alinhado à direita.
- `MODIFICADO` formatado manualmente a partir do `u64` epoch como
  `YYYY-MM-DD HH:MM` no fuso local (sem timezone info visível — é
  puramente informacional).

---

## Entidade auxiliar: `AddedTemplate`

Retornado por `add_template(...)` para o handler compor a mensagem
final em stdout. Não é persistida nem serializada.

```rust
pub struct AddedTemplate {
    pub name: String,
    pub path: PathBuf,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
}
```

Regras:

- `bytes_written` = tamanho final do arquivo persistido.
- `overwrote_existing` = `true` sse existia um template com o mesmo
  nome antes do `add`.

---

## Extensão de `TexError` (spec 001)

Três novas variantes conforme D-08 do research:

```rust
#[error("Template '{name}' não existe em {templates_dir}.")]
TemplateNotFound { name: String, templates_dir: PathBuf },

#[error("Arquivo '{source_path}' não é texto UTF-8 válido: {detail}.")]
InvalidUtf8 { source_path: PathBuf, detail: String },

#[error(
    "Diretório de templates '{templates_dir}' não existe. \
     Rode 'tex-cli config set paths.templates_dir <path>' \
     ou crie o diretório."
)]
TemplatesDirMissing { templates_dir: PathBuf },
```

`TexError::exit_code()` estendido:

- `TemplateNotFound { .. }` → **20**
- `InvalidUtf8 { .. }` → **21**
- `TemplatesDirMissing { .. }` → **22**

Todos os demais mapeamentos (10..15, 1) permanecem inalterados.

---

## Regras de validação por operação

### `list_templates(dir: &Path) -> Result<Vec<Template>, TexError>`

| Cenário | Comportamento |
|---------|---------------|
| `dir` não existe | `TexError::TemplatesDirMissing { templates_dir: dir.to_path_buf() }` → exit 22 |
| `dir` existe mas não é diretório | `TexError::TemplatesDirMissing { .. }` (usuário aponta pra arquivo — trata como "não é dir válido") |
| `dir` sem permissão de leitura | `TexError::PermissionDenied { path: dir }` → exit 14 (herda spec 001) |
| `dir` vazio | `Ok(vec![])` — não é erro |
| Entradas não-`.tex` | filtradas silentemente |
| Entradas que são subdirs | filtradas silentemente |
| Entradas com nome não-UTF8 | filtradas silentemente com `warn!` (log em `-v`) |

### `resolve_template(dir: &Path, name: &str) -> Result<PathBuf, TexError>`

| Cenário | Comportamento |
|---------|---------------|
| `name` termina em `.tex` e `dir/{name}` existe | `Ok(dir/name)` |
| `name` não termina em `.tex` e `dir/{name}.tex` existe | `Ok(dir/{name}.tex)` |
| Nada bate | `TexError::TemplateNotFound { name, templates_dir: dir }` → exit 20 |

### `read_template(dir: &Path, name: &str) -> Result<Vec<u8>, TexError>`

Delega a `resolve_template` + `fs::read`. Não valida UTF-8 (deixa
`show` cru — pode até ser um `.tex` com bytes latin1 legado; usuário
que vê).

### `add_template(dir, source_path, name, force) -> Result<AddedTemplate, TexError>`

Pré-condições:

| Cenário | Comportamento |
|---------|---------------|
| `dir` não existe | `TexError::TemplatesDirMissing { .. }` → exit 22 |
| `source_path` não existe | `TexError::Io(e)` do `read` → exit 1 (herdado spec 001) |
| `source_path` não é UTF-8 | `TexError::InvalidUtf8 { source_path, detail }` → exit 21 |
| `dir/{name}.tex` já existe e `force = false` | Caller decide: se TTY, `confirm_overwrite_template` (D-06 research); senão, `TexError::UserAborted` → exit 15 |
| `dir/{name}.tex` já existe e `force = true` | Sobrescreve via `save_atomic` (D-03 research), sem prompt |

Pós-condição em sucesso:

- Arquivo `dir/{name}.tex` existe com o conteúdo de `source_path`.
- Permissão `0644` (D-04 research).
- `AddedTemplate` retornado com `overwrote_existing` refletindo o
  estado pré.

### `remove_template(dir, name, force) -> Result<PathBuf, TexError>`

Pré-condições:

| Cenário | Comportamento |
|---------|---------------|
| `dir` não existe | `TexError::TemplatesDirMissing { .. }` → exit 22 |
| Template não existe | `TexError::TemplateNotFound { .. }` → exit 20 |
| `force = false` e não-TTY | `TexError::UserAborted` → exit 15 (sem tocar no arquivo) |
| `force = false` e TTY: user responde "não" | `TexError::UserAborted` → exit 15 |
| `force = true` OU TTY user responde "sim" | `fs::remove_file` |

Pós-condição em sucesso:

- Arquivo não existe mais em `dir/{name}.tex`.
- Retorna o `PathBuf` que foi removido para o handler compor a
  mensagem stdout.

---

## Não é entidade

Os itens abaixo aparecem no fluxo mas **não** viram struct persistida:

- Resposta do `inquire::Confirm` em `add` / `remove` — booleano local
  do handler.
- Escolha do menu interativo (`templates` sem subcomando) — enum
  local `TemplateMenuAction`.
- Bytes lidos do template no `show` — passam direto para stdout, sem
  intermediação.
