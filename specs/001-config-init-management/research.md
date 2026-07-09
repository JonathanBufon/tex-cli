# Phase 0 — Research: Configuração Inicial e Gestão do Config do Tex

**Feature**: `001-config-init-management`
**Date**: 2026-07-09

Todos os NEEDS CLARIFICATION do Technical Context foram resolvidos.
Este documento registra as decisões técnicas e as alternativas
rejeitadas, para servir de referência ao `/speckit-tasks` e à
revisão de código.

---

## D-01. Serialização TOML: `toml` vs `toml_edit`

**Decision**: Usar `toml` (v0.8+, com `serde` feature).

**Rationale**:

- O crate `toml` (sucessor recente do antigo `toml_edit` para casos
  simples) suporta `serde::Serialize`/`Deserialize` roundtrip, o que
  cobre 100% dos requisitos: carregar em struct, editar campo, salvar
  de volta.
- A spec (Assumptions) já declara explicitamente que o Tex **não é
  obrigado a preservar comentários ou formatação arbitrária** do
  arquivo — o que remove o principal motivo para escolher `toml_edit`
  (edit preservando comentários).
- Menos dependências transitivas; API simples.

**Alternatives considered**:

- **`toml_edit`**: preservaria comentários e ordem exata. Rejeitado
  porque o requisito é explicitamente relaxado, e a complexidade extra
  do DOM não paga.
- **Manual TOML** (regex): rejeitado imediatamente — fragilidade e
  risco de corromper o arquivo (violaria FR-021 e FR-003a).

---

## D-02. Escrita atômica: `tempfile::NamedTempFile` vs manual `rename`

**Decision**: `tempfile::NamedTempFile::new_in(target_dir)` +
`persist(target_path)`.

**Rationale**:

- `NamedTempFile::new_in` **garante que o tmpfile fique no mesmo
  filesystem** que o destino — condição necessária para `rename(2)`
  ser atômico (POSIX). Criar em `/tmp` seria bug em setups onde
  `/tmp` é tmpfs separado.
- `.persist(dst)` faz `rename` sem race entre unlink e create.
- API já expõe `File` para escrever conteúdo antes do persist.
- Permissão `0600` deve ser aplicada **antes** do persist para
  eliminar janela de visibilidade (via `std::os::unix::fs::PermissionsExt`
  no `File` do tmpfile).

**Alternatives considered**:

- **Write direto + `std::fs::rename`**: funciona, mas duplica lógica
  já pronta em `tempfile` (nomeação, cleanup em drop).
- **`std::fs::write` sem rename**: janela clara de corrupção — viola
  FR-003a.
- **`fsync` explícito antes do rename**: overkill para uso pessoal
  single-user; `rename` em ext4/xfs já dá durabilidade suficiente para
  este workload. Se o caso mudar (ex.: shell fecha e o kernel não
  flushou), documentar como known limitation.

---

## D-03. Resolução de caminho: `dirs::config_dir()` vs `env::var("XDG_CONFIG_HOME")`

**Decision**: `dirs::config_dir()` + append `"tex/config.toml"`.

**Rationale**:

- `dirs` já implementa a especificação XDG completa: honra
  `$XDG_CONFIG_HOME` se definido, senão `~/.config`, e trata bordas
  (ex.: `$HOME` ausente em containers minimais).
- É a lib da stack canônica (constitution) — evita adicionar
  dependência nova.
- Multiplataforma (útil se um dia macOS/Windows entrarem no roadmap
  — mesmo código funciona).

**Alternatives considered**:

- **Ler `XDG_CONFIG_HOME` na mão**: reimplementaria bordas já
  resolvidas em `dirs`.
- **Hardcode `$HOME/.config/tex/`**: quebra se usuário mover config
  root via XDG — violaria expectativa Linux moderna.

---

## D-04. Expansão de `~` e caminhos relativos

**Decision**: Função `paths::expand_user_path(raw: &str) -> PathBuf` no
módulo `paths.rs`, que:

1. Se começa com `~/` ou é literal `~`: substitui por `dirs::home_dir()`.
2. Se relativo (não começa com `/`): canonicaliza contra `std::env::current_dir()?`.
3. Se absoluto: normaliza (`Path::components` colapsando `.` e `..`).
4. Retorna erro claro (`TexError::HomeDirUnavailable`) se `~` for
   usado mas home não existir.

**Rationale**:

- FR-016 exige expansão tanto de `~` quanto de relativos.
- Manter em uma única função elimina duplicação entre `init` e
  `config set`.
- `PathBuf` (não `String`) evita quebrar em paths com bytes não-UTF8.

**Alternatives considered**:

- **`shellexpand` crate**: adicionaria dependência fora da stack
  canônica. Rejeitado — a lógica de que precisamos cabe em ~20 linhas.
- **`fs::canonicalize`**: exige que o path exista no disco (falha para
  diretório de output que o usuário quer criar depois). Rejeitado por
  esse motivo — normalizamos manualmente.

---

## D-05. Mapeamento de `-v` (repetido) → filtro do `tracing`

**Decision**: Contar ocorrências de `-v` via clap
(`#[arg(short, long, action = clap::ArgAction::Count)]`) e mapear:

| Count | Level  |
|-------|--------|
| 0     | WARN   |
| 1     | INFO   |
| 2     | DEBUG  |
| ≥3    | TRACE  |

Configurado em `main.rs` via
`tracing_subscriber::fmt().with_max_level(level).with_writer(std::io::stderr).init()`.
Sempre em stderr (nunca stdout — para não sujar `config show --format=json`).

**Rationale**:

- Padrão Unix clássico (`curl -v`, `ssh -vv`).
- Clap tem suporte nativo via `ArgAction::Count` — zero código extra.
- Levar tracing para stderr preserva FR-024.

**Alternatives considered**:

- **Env `TEX_LOG=…` estilo `RUST_LOG`**: rejeitado nesta feature
  (Clarification Q5 já resolveu: silencioso + `-v/-vv`). Pode ser
  aditivo no futuro.
- **`--quiet` flag**: default já é silencioso (WARN); `-q` seria
  redundante nesta feature. Pular.

---

## D-06. Aplicação de dotted-path key no config

**Decision**: Enum `ConfigKey` gerado exaustivamente:

```rust
enum ConfigKey {
    PathsTemplatesDir,      // "paths.templates_dir"
    PathsOutputDir,         // "paths.output_dir"
    CompilerEngine,         // "compiler.engine"
    CompilerKeepTex,        // "compiler.keep_tex"
    CompilerKeepLogs,       // "compiler.keep_logs"
    BehaviorAskOutputPath,  // "behavior.ask_output_path_every_time"
}
```

Com `FromStr` que aceita **exatamente** os literais dotted-path (sem
aliases, sem case-insensitive). `Display` retorna a string canônica.
Método `Config::apply(key, raw_value)` faz o match e parseia o
valor para o tipo correto.

**Rationale**:

- Enum fechado torna FR-014 impossível de burlar acidentalmente:
  qualquer chave nova requer nova variante (compilador enforce).
- FR-015 (listar chaves suportadas em erro) sai grátis: `strum` ou
  `impl Display` para todas as variantes.
- Match exhaustive garante que adicionar campo ao `Config` sem
  atualizar `apply()` quebra o build.

**Alternatives considered**:

- **Manipular `toml::Value` genericamente por path**: mais flexível,
  mas perde type-safety e valida menos coisas em compile-time. Erros
  ficariam runtime-only, contradizendo o princípio de fail-fast.
- **`serde_path_to_error`**: útil para relatar onde está erro em
  parse, mas não resolve a aplicação de uma mudança pontual.

---

## D-07. Estratégia de teste "sem tectonic no PATH"

**Decision**: Test helper que executa o binário com
`env_clear()` + `env("PATH", "/tmp/empty-path-for-test")` via
`assert_cmd::Command::env`. Diretório `/tmp/empty-path-for-test`
criado com `assert_fs::TempDir` sem nenhum binário.

**Rationale**:

- Isola de forma robusta sem depender do PATH real do container.
- Rodável no host **ou** no Docker (não obriga tudo a ir para
  container).

**Alternatives considered**:

- **`unset PATH`**: em muitos shells causa `command not found` para o
  próprio binário do teste. Rejeitado.
- **Mockar `which::which`**: exigiria injeção de dependência no
  binário — over-engineering para um teste tão simples.

---

## D-08. Dockerfile mínimo para testes

**Decision**:

```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential curl ca-certificates tectonic pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /src
CMD ["cargo", "test", "--all"]
```

**Rationale**:

- `debian:bookworm-slim` é a base padrão declarada pelo usuário e é
  pequena (~30MB base).
- `tectonic` está nos repositórios oficiais Debian bookworm — não
  precisa build de fonte.
- Rust via rustup dentro do container mantém o build reprodutível e
  desacoplado da versão do host.
- Sem `USER` custom — testes rodam como root dentro do container, que
  é aceitável para uso pessoal isolado.

**Alternatives considered**:

- **`rust:1-slim`** como base: teria Rust pronto, mas exigiria
  instalar `tectonic` do repositório Debian mesmo assim, e a base do
  usuário é Debian por definição de projeto.
- **`FROM tectonictypesetting/tectonic`**: imagem oficial, mas
  provavelmente não traz toolchain Rust; iríamos misturar bases sem
  ganho real.

---

## D-09. Códigos de saída documentados

**Decision**:

| Código | Significado                                                      |
|--------|------------------------------------------------------------------|
| 0      | Sucesso                                                          |
| 1      | Erro genérico não classificado (uso da fronteira `anyhow`)       |
| 2      | Erro de argumento CLI (mapeado automaticamente pelo `clap`)      |
| 10     | Config ausente (FR-011, FR-020)                                  |
| 11     | Config existente corrompido (TOML inválido) (FR-012)             |
| 12     | Chave de config não reconhecida (FR-015)                         |
| 13     | Valor inválido para chave (booleano malformado etc.) (FR-017)    |
| 14     | Permissão negada para gravar config (FR-021)                     |
| 15     | Usuário abortou operação interativa (Ctrl+C, resposta "não")     |

Documentados no `contracts/cli.md`. Cobre FR-022 (códigos
determinísticos).

**Rationale**:

- Faixa 10+ evita colisão com códigos convencionais do shell (1, 2)
  e do próprio Rust panic.
- Códigos específicos permitem scripts CI reagir apropriadamente
  (SC-006).

**Alternatives considered**:

- **Só 0 e 1**: perde granularidade que scripts precisam.
- **Enum `sysexits.h`** (64+): não é padrão em CLIs modernos; convido
  confusão. Rejeitado.

---

## D-10. Banner "TEX CLI": renderização e canal

**Decision**:

- Asset em `assets/banner.txt` (block letters Unicode, sem bordas
  `//` do arquivo original — decisão editorial na Clarification
  suplementar).
- Embutido no binário via `const BANNER: &str = include_str!("../assets/banner.txt");`.
- Emitido em **stderr** por `main.rs` como primeira ação após
  inicializar tracing, antes do dispatch para qualquer subcomando.
- Uma única chamada `eprint!("{BANNER}")` (sem `eprintln!` — o
  asset já termina em newline).

**Rationale**:

- Aparece em **todos** os subcomandos automaticamente (init, config
  show, config set) sem precisar espalhar chamadas em cada handler.
- Stderr preserva `config show --format=json | jq …` (stdout limpo).
- `include_str!` elimina I/O em runtime e garante que o binário
  distribuído contém o banner (sem depender de caminho relativo).
- Nenhum caminho pode "esquecer" de imprimir o banner porque o local
  é único e alto na call chain.

**Alternatives considered**:

- **Print em cada handler**: dispersa a lógica; fácil de esquecer
  ao adicionar comando novo. Rejeitado.
- **Ler `assets/banner.txt` em runtime**: quebra distribuição
  standalone (precisaria empacotar o arquivo junto). Rejeitado.
- **Stdout**: quebraria FR-010 (formato JSON/TOML determinístico).
  Rejeitado.
- **Suprimível via `--no-banner` ou `TEX_NO_BANNER=1`**: sem caso
  de uso comprovado na v1; adiado para quando/se um usuário
  reclamar. Deixar decisão registrada aqui.
