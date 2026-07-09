# Phase 0 — Research: Gestão de Templates LaTeX

**Feature**: `002-templates-management`
**Date**: 2026-07-09

Este documento resolve todos os NEEDS CLARIFICATION do `plan.md` e
registra as decisões técnicas com alternativas rejeitadas para o
`/speckit-tasks` e para reviewers.

Cross-reference: várias decisões desta spec reaproveitam as decisões
D-01..D-10 da spec 001 sem alteração (I/O atômico, banner, exit-code
mapping, etc.) — só decisões **novas ou expandidas** aparecem aqui.

---

## D-01. Formato de `modified_at` no JSON: `chrono` vs. Unix epoch vs. formatação manual

**Decision**: **Emitir `modified_at_epoch` (u64, segundos desde
UNIX_EPOCH) no JSON e um formato humano-legível YYYY-MM-DD HH:MM na
saída `humano`.** Zero nova dep.

**Rationale**:

- `SystemTime::duration_since(UNIX_EPOCH)` já está no `std`; converter
  para `u64` é uma linha.
- Scripts têm caminho trivial pro formato desejado: `jq '.[] |
  (.modified_at_epoch | strftime("%Y-%m-%dT%H:%M:%SZ"))'`.
- Adicionar `chrono` (ou `time`) só para formatar RFC3339 seria uma
  **mudança na stack canônica da constitution** (Restrições Técnicas
  › Stack fixada) — o custo de emenda supera o benefício num
  projeto pessoal.
- O formato humano da saída `list` (não-JSON) usa cálculo simples via
  `SystemTime` + módulo/divisão para dividir em anos/dias/horas — o
  usuário na saída humana quer legibilidade, não precisão milissegundo.

**Alternatives considered**:

- **`chrono` v0.4**: API madura, RFC3339 out of the box. Rejeitado —
  expande stack canônica sem ganho proporcional; timezone handling
  desnecessário no escopo (single-user local machine).
- **`time` v0.3**: mais moderno que `chrono`, ainda menos difundido.
  Rejeitado pelo mesmo motivo.
- **Formatação RFC3339 manual em `std`**: 20-30 linhas de código para
  dividir seconds em year/month/day/hour/min/sec, respeitando bissexto
  etc. Rejeitado por fragilidade (bugs sutis) e teste caro.
- **Emitir string ISO-8601 UTC via `SystemTime` + libc**: mistura de
  padrões, fica pior do que a solução escolhida.

**Impacto no Complexity Tracking do plan**: linha removida — não há
mais dep fora da stack canônica.

---

## D-02. Resolução do nome do template (`show`/`remove`)

**Decision**: `resolve_template_name(dir, name)` aceita
`name` **com ou sem** sufixo `.tex`:

1. Se `name.ends_with(".tex")`, procura `dir/{name}` diretamente.
2. Caso contrário, procura `dir/{name}.tex`.
3. Ambos aplicam **matching case-sensitive** — sistema de arquivos
   POSIX pode distinguir `artigo.tex` de `Artigo.tex`, e a v1 respeita
   essa distinção (nenhuma normalização de case).
4. Se o arquivo não existe, `TexError::TemplateNotFound { name,
   templates_dir }` → exit 20.

**Rationale**:

- FR-006 exige aceitar ambos os formatos. Case-sensitivity está no
  spec (Edge Cases › "Dois arquivos com case diferente").
- Rejeita a ambiguidade que surgiria de fuzzy matching (`--artigo`
  batendo com `Artigo.tex`) — usuário sempre sabe qual arquivo abriu.
- Se dois arquivos existem no mesmo dir (`artigo.tex` e `Artigo.tex`),
  ambos aparecem no `list` como templates distintos; `show artigo`
  retorna `artigo.tex` (o exato match), `show Artigo` retorna
  `Artigo.tex`.

**Alternatives considered**:

- **Case-insensitive match**: mais amigável mas ambíguo em POSIX.
  Rejeitado.
- **Fuzzy match (levenshtein)**: over-engineering para o caso comum
  de <100 templates. Rejeitado.
- **Sempre exigir extensão explícita**: viola FR-006 direto.
  Rejeitado.

---

## D-03. Escrita atômica do `add`: cópia de bytes vs. `tempfile::persist`

**Decision**: `tempfile::NamedTempFile::new_in(templates_dir)` +
`write_all(bytes)` + `set_permissions(0o644)` + `.persist(target)`.
Idêntico ao pattern do `Config::save_atomic` da spec 001, exceto pela
permissão (0644 em vez de 0600 — ver D-04 abaixo).

**Rationale**:

- Reaproveita exatamente o padrão já provado na spec 001 (D-02 dessa
  spec).
- `tempfile` já está na stack canônica.
- Garante que uma falha durante a cópia deixa o destino intocado (só
  o tempfile temporário, que é limpo no drop).
- Facilita reuso: `pub fn write_atomic(target, bytes, mode)` poderia
  virar um helper compartilhado entre `config.rs` e `templates.rs`
  numa refactor futura — **fora do escopo desta spec**, mas anotado.

**Alternatives considered**:

- **`std::fs::write` direto**: janela clara de corrupção se o
  processo cair durante a escrita. Rejeitado (viola FR-012 e o
  Princípio V da constitution).
- **`std::fs::copy`**: atômico apenas se o kernel implementa
  copy_file_range com semântica atômica — não é garantia POSIX,
  frágil. Rejeitado.

---

## D-04. Permissão do arquivo gravado pelo `add`: `0644` vs. `0600` vs. umask default

**Decision**: **`0644` explícito** (via
`std::os::unix::fs::PermissionsExt::set_mode(0o644)` no tempfile
antes do persist).

**Rationale**:

- FR-013 é explícito: templates são candidatos a versionamento git,
  portanto **não** devem receber o `0600` restrito do config.
- Umask default do usuário poderia ser `0644` (comum) ou `0600`
  (paranoico); depender dele torna comportamento não-determinístico
  entre setups. Fixar `0644` remove ambiguidade e passa em CI/testes.
- Não é decisão de segurança perigosa: o arquivo já está no diretório
  do usuário; `0644` só permite outros usuários **do mesmo host**
  lerem o template, o que é aceitável (não contém segredos).

**Alternatives considered**:

- **`0600` como no config**: sobre-restringe; usuário não consegue
  compartilhar o template com colegas via o mesmo home (raro, mas
  possível) e complica versionamento git em ambientes multiusuário.
  Rejeitado.
- **Herdar umask**: comportamento inconsistente entre setups.
  Rejeitado.
- **`0640` (grupo)**: intermediário, mas premissa de grupo compartilhado
  não é universal em uso pessoal. Rejeitado.

---

## D-05. Validação UTF-8 no `add`: `std::str::from_utf8` no buffer inteiro vs. streaming

**Decision**: `std::str::from_utf8(&bytes)` sobre o buffer completo
carregado em memória.

**Rationale**:

- Templates típicos < 10 KB, worst case razoável < 1 MB. Carregar
  tudo em memória é trivial.
- Streaming (`Utf8Reader`) adiciona complexidade (buffer management,
  handling de sequências multi-byte cortadas na borda) sem ganho.
- Falha (retorna `Utf8Error`) → `TexError::InvalidUtf8 { source_path,
  detail }` → exit 21. Mensagem em pt-BR humana.

**Alternatives considered**:

- **`String::from_utf8_lossy`**: silenciaria bytes inválidos como
  U+FFFD, corrompendo o template. Rejeitado (viola FR-010).
- **Streaming com `utf8parse` crate**: 3rd-party dep, over-engineering
  para arquivos pequenos. Rejeitado.
- **Nenhuma validação**: viola FR-010 e Princípio V. Rejeitado.

---

## D-06. Detecção de TTY para não travar em CI: `IsTerminal`

**Decision**: `std::io::IsTerminal` (stable desde Rust 1.70) sobre
`io::stdin()` para decidir se prompts interativos devem rodar ou se o
comando aborta com exit 15.

Locais de uso:

- `templates add` sem `--force` sobre template existente.
- `templates remove` sem `--force`.
- `templates` sem subcomando (menu).

Padrão: `if stdin.is_terminal() { run_prompt } else if !force {
return Err(TexError::UserAborted) }`.

**Rationale**:

- `IsTerminal` está no std desde 1.70 — nenhuma dep nova.
- Comportamento previsível em CI: sem TTY + sem `--force` = falha
  cedo com exit 15, orientando o usuário a passar `--force`.
- Consistente com o padrão que a spec 001 já estabeleceu (init com
  `--force` bypassa o prompt de overwrite).

**Alternatives considered**:

- **`atty` crate**: pré-1.70 era a solução comum; hoje é redundante
  com `std::io::IsTerminal`. Rejeitado.
- **Sempre tentar prompt e falhar naturalmente**: inquire retornaria
  `InquireError::IO`, mensagem opaca. UX pior. Rejeitado.
- **Env var `TEX_CLI_NON_INTERACTIVE=1`**: redundante com detecção
  automática de TTY. Rejeitado.

---

## D-07. Estratégia de listagem: `read_dir` a cada invocação vs. cache

**Decision**: **Sempre `read_dir` na hora**, sem cache.

**Rationale**:

- Até 100 templates, `read_dir` + `metadata` × N é submilissegundo em
  SSD.
- Cache introduz problema de invalidação (o usuário pode editar
  arquivos com `$EDITOR` sem passar pela CLI).
- SC-002 (< 100ms) já cumprido sem cache, medido em POC informal.

**Alternatives considered**:

- **Cache em memória com watch (`notify` crate)**: nova dep, complexidade,
  processo tem que ficar rodando. Rejeitado.
- **Cache em disco (manifest side-car)**: divergência com estado real
  se o usuário mexe fora da CLI. Viola FR-001 implicitamente. Rejeitado.

---

## D-08. Exit codes 20/21/22: extensão do enum `TexError`

**Decision**: Adicionar três variantes ao `TexError`:

```rust
#[error("Template '{name}' não existe em {templates_dir}.")]
TemplateNotFound { name: String, templates_dir: PathBuf },

#[error("Arquivo '{source_path}' não é texto UTF-8 válido: {detail}.")]
InvalidUtf8 { source_path: PathBuf, detail: String },

#[error("Diretório de templates '{templates_dir}' não existe. Rode 'tex-cli config set paths.templates_dir <path>' ou crie o diretório.")]
TemplatesDirMissing { templates_dir: PathBuf },
```

`TexError::exit_code()` estendido:

- `TemplateNotFound` → 20
- `InvalidUtf8` → 21
- `TemplatesDirMissing` → 22

**Rationale**:

- Mesma abordagem da spec 001 (D-09): faixa 20+ evita colisão com
  códigos shell padrão e com os já usados (10..15).
- Fixed `impl Display` humano em português, sem debug format
  (padrão spec 001).

**Alternatives considered**:

- **Reaproveitar `ConfigMissing` (10) para `TemplatesDirMissing`**:
  ambíguo — script CI não distingue "sem config" de "config OK mas dir
  ausente". Rejeitado.
- **Códigos > 30**: sem motivo pra pular; 20-22 mantém sequência
  natural. Rejeitado.

---

## D-09. Modo interativo (`tex-cli templates` sem subcomando): loop vs. one-shot

**Decision**: **One-shot** — escolhe uma ação, executa, retorna. Não
volta pro menu depois.

**Rationale**:

- Menor complexidade de estado. Se o usuário quer múltiplas ações,
  usa CLI direto ou re-invoca.
- Comportamento consistente com o padrão do inquire (Select é
  one-shot).
- Loop introduz corner cases (Ctrl+C no meio de submenu, "voltar",
  navegação) que não têm ROI num projeto pessoal.

**Alternatives considered**:

- **Loop com "Voltar ao menu"**: útil pra sessão exploratória mas
  aumenta código de controle de fluxo. Rejeitado pra v1; anotado como
  possível melhoria futura.

---

## D-10. Estrutura interna do módulo `templates.rs`: funções livres vs. struct `TemplatesDir`

**Decision**: Funções livres (`pub fn list_templates(dir: &Path) ->
Result<Vec<Template>, TexError>` etc.), estado zero.

**Rationale**:

- Sem estado a preservar entre chamadas (config é lido pelo caller).
- Testes unitários mais simples — passam `TempDir` e assertam.
- Consistente com o estilo de `paths.rs` (funções livres) da spec 001.

**Alternatives considered**:

- **`struct TemplatesDir { path: PathBuf }` com métodos**: seria útil
  se houvesse invariantes cachear ou estado. Não há. Rejeitado por
  YAGNI.
- **Trait `TemplatesRepo` para permitir mock**: over-engineering para
  operações filesystem-triviais que já são unit-testáveis com
  `tempfile`. Rejeitado.
