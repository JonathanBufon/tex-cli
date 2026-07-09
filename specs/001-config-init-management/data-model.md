# Phase 1 — Data Model: Configuração do Tex

**Feature**: `001-config-init-management`
**Date**: 2026-07-09

Modelo de dados **interno** desta feature: a única entidade
persistente é o `Config` (arquivo TOML em `~/.config/tex/config.toml`).
Este documento fixa a forma canônica dessa struct em Rust e as regras
de validação, sem entrar no detalhe de assinaturas de função (isso
fica para o código).

---

## Entidade: `Config`

Persistida como TOML. Struct Rust roundtrip via `serde`.

### Estrutura canônica

```toml
[paths]
templates_dir = "/home/jonathan/Documents/tex/templates"
output_dir    = "/home/jonathan/Documents/tex/output"

[compiler]
engine    = "tectonic"
keep_tex  = true
keep_logs = true

[behavior]
ask_output_path_every_time = false
```

### Struct Rust

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub paths: PathsConfig,
    pub compiler: CompilerConfig,
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PathsConfig {
    pub templates_dir: PathBuf,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompilerConfig {
    pub engine: String,      // "tectonic" (default) | outros com warning
    pub keep_tex: bool,
    pub keep_logs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BehaviorConfig {
    pub ask_output_path_every_time: bool,
}
```

### Defaults

Aplicados **quando o usuário roda `init` e não fornece o campo**
(fora de escopo da spec atual perguntar tudo, então `init` só
pergunta 3 campos e o resto vai por default):

| Campo                                      | Default        |
|--------------------------------------------|----------------|
| `compiler.keep_tex`                        | `true`         |
| `compiler.keep_logs`                       | `true`         |
| `behavior.ask_output_path_every_time`      | `false`        |
| `compiler.engine`                          | `"tectonic"`   |

`paths.templates_dir` e `paths.output_dir` **não têm default** —
`init` obrigatoriamente pergunta.

### Validação em tempo de load/save

| Regra | Momento | Referência |
|-------|---------|------------|
| TOML deve parsear como estrutura acima | load | FR-002, FR-012 |
| `paths.templates_dir` e `paths.output_dir` devem ser strings de path não vazias | load + save | FR-005 |
| `paths.*` são absolutas ao serem gravadas (expansão feita antes) | save | FR-016 |
| `compiler.engine` presente | load | FR-005 |
| Booleans **estritos** (aceita apenas `true`/`false`; não `1`, `"yes"`) | load + apply | FR-017 |
| `compiler.engine` reconhecido como suportado é apenas `tectonic`; outros são aceitos mas geram warning | save + apply | FR-008, FR-019 |
| Arquivo gravado com permissão `0600` | save | FR-003b |
| Gravação atômica (tmpfile + rename) | save | FR-003a |

### Comportamento de round-trip

- **Ordem das seções**: `[paths]` → `[compiler]` → `[behavior]`,
  determinística (garantida pela ordem dos campos na struct).
- **Comentários**: descartados (documentado nas Assumptions da spec).
- **Campos desconhecidos no arquivo**: rejeitados na deserialização
  (`#[serde(deny_unknown_fields)]` em cada struct filha) — evita silenciar
  typos.

---

## Entidade: `ConfigKey` (chave dotted-path para `config set`)

Enum fechado que representa cada chave editável. Não é persistido —
existe apenas como valor de argumento de CLI.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigKey {
    PathsTemplatesDir,
    PathsOutputDir,
    CompilerEngine,
    CompilerKeepTex,
    CompilerKeepLogs,
    BehaviorAskOutputPathEveryTime,
}

impl ConfigKey {
    pub const ALL: &'static [ConfigKey] = &[
        ConfigKey::PathsTemplatesDir,
        ConfigKey::PathsOutputDir,
        ConfigKey::CompilerEngine,
        ConfigKey::CompilerKeepTex,
        ConfigKey::CompilerKeepLogs,
        ConfigKey::BehaviorAskOutputPathEveryTime,
    ];

    pub fn canonical(&self) -> &'static str { /* dotted string */ }
}

impl FromStr for ConfigKey { /* exact match, no aliases */ }
impl Display   for ConfigKey { /* returns canonical() */ }
```

### Mapeamento chave → campo → tipo esperado do valor

| Dotted-path                              | Campo Rust                                     | Tipo do valor CLI |
|------------------------------------------|------------------------------------------------|-------------------|
| `paths.templates_dir`                    | `Config.paths.templates_dir`                   | path (String)     |
| `paths.output_dir`                       | `Config.paths.output_dir`                      | path (String)     |
| `compiler.engine`                        | `Config.compiler.engine`                       | string            |
| `compiler.keep_tex`                      | `Config.compiler.keep_tex`                     | bool              |
| `compiler.keep_logs`                     | `Config.compiler.keep_logs`                    | bool              |
| `behavior.ask_output_path_every_time`    | `Config.behavior.ask_output_path_every_time`   | bool              |

### Regras de parse do valor (por tipo)

- **path**: aceita literal, expande via `paths::expand_user_path`.
  Se resultado não existir no disco: **grava mesmo assim** + warning
  (FR-018).
- **bool**: aceita **apenas** `true` ou `false`. Qualquer outro
  valor → erro `TexError::InvalidBoolValue` → exit code 13 (FR-017).
- **string** (`compiler.engine`): não vazia. Se não for `tectonic`,
  grava + warning "não suportado ainda" (FR-019).

---

## Entidade: `SupportedEngine` (auxiliar, não persistido)

Lista dos nomes de engine que a **v1** reconhece como preferência
(mesmo que só `tectonic` funcione de verdade). Usada para dois
propósitos:

1. Preencher o `Select` do `inquire` no comando `init`.
2. Emitir warnings direcionados em `config set` / `init`.

```rust
pub enum SupportedEngine {
    Tectonic,   // fully supported
    Latexmk,    // reconhecido, warning "não suportado ainda"
    Pdflatex,   // idem
    Xelatex,    // idem
    Lualatex,   // idem
}
```

Regra: apenas `Tectonic` passa no check funcional; os outros são
persistidos como strings livres com aviso não-bloqueante. Isso
respeita o Princípio IV (plugável, sem hardcode fora do módulo do
compilador).

---

## Não é entidade

Os itens abaixo aparecem no fluxo mas **não** viram struct persistida:

- Respostas do `inquire` durante `init`: mantidas em variáveis
  locais até a montagem do `Config` final.
- Verbosidade CLI: parâmetro contável, não persistido.
- Formato de saída (`--format=<…>`): enum de CLI, não persistido.
