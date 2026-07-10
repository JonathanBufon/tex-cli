# Phase 0 — Research: Renderização JSON → `.tex` via Tera

**Feature**: `003-render-tera`
**Date**: 2026-07-09

Resolução das decisões técnicas do plan. Registra rationale e
alternativas rejeitadas.

Cross-reference: reaproveita decisões D-01..D-10 da spec 001
(atomic write, banner, exit-code mapping, `-v` verbose) e D-01..D-10
da spec 002 (extensão de `TexError`, TTY detection via `IsTerminal`,
padrão inquire). Somente decisões **novas** aparecem aqui.

---

## D-01. API do tera: `Tera::one_off` vs. registrar template numa instância

**Decision**: Usar `tera::Tera::one_off(src, context, autoescape=false)`
por invocação — sem cache de instâncias.

**Rationale**:

- O caso de uso é single-shot: um render por invocação de CLI.
  Cache de instância `Tera` só compensa quando o mesmo template é
  renderizado múltiplas vezes no mesmo processo.
- `one_off` aceita a string do template diretamente — não precisa
  compilar arquivo externo. Simplifica o pipeline: `read_template`
  já entrega bytes.
- Escapar HTML (`autoescape=true`) quebraria `.tex` (`&` viraria
  `&amp;`, `%` viraria `%25`). Autoescape=false é obrigatório.
- API estável desde tera 1.0.

**Alternatives considered**:

- **Registrar via `Tera::new(glob)`**: útil para SSR/web onde o
  mesmo template é reusado. Overkill para CLI single-shot.
- **`Tera::default()` + `render_str`**: função existe mas requer
  registrar template com nome; `one_off` é o wrapper conveniente.
  Rejeitado por ergonomia.

---

## D-02. Conversão JSON → tera Context: `Context::from_value` vs. mapeamento manual

**Decision**: `tera::Context::from_value(serde_json::Value)`.

**Rationale**:

- `tera::Context` implementa `TryFrom<serde_json::Value>` (via
  `from_value`) exatamente para este caso.
- Preserva estruturas aninhadas (objects → objects, arrays →
  arrays, tipos primitivos preservados).
- Zero código extra. Rejeição de "não é objeto no top-level"
  acontece naturalmente porque `from_value` só aceita `Value::Object`
  como raiz.

**Alternatives considered**:

- **Mapear manualmente `Value` → `Context`**: reimplementaria
  `from_value`. Sem ganho. Rejeitado.
- **Aceitar `Value` arbitrário e converter arrays em `{items:[...]}`**:
  adiciona magic behavior que confunde o dev. Rejeitado por
  princípio da menor surpresa.

---

## D-03. Escrita atômica: extrair helper `write_atomic_0644` ou duplicar?

**Decision**: **Extrair** `write_atomic_0644(target, bytes)` de
`src/templates.rs` para um novo módulo `src/atomic.rs` ou como
`pub fn` na crate root, e chamar dos dois consumidores (spec 002 e
spec 003).

**Rationale**:

- A função é literalmente 20 linhas e já foi copiada uma vez (spec
  001 → spec 002 com pequena adaptação de mode 0600→0644).
- Extrair evita a terceira cópia + drift (a spec 004 compile
  também vai precisar de escrita atômica pra PDF).
- Interface: `pub fn write_atomic(target: &Path, bytes: &[u8], mode:
  u32) -> Result<(), TexError>`. `templates.rs` chama com `0o644`,
  `render.rs` também. Config (`0o600`) já usa uma variação inline
  em `config.rs`; refactor daquela também pode ser feito como task
  de polish, sem urgência.

**Alternatives considered**:

- **Duplicar em `render.rs`**: 20 linhas × 3 arquivos. Fica caro
  quando spec 004 aparecer. Rejeitado.
- **Manter em `templates.rs` e re-exportar**: acopla dois módulos
  não relacionados semanticamente. Rejeitado por baixa coesão.
- **Trait genérico `AtomicWriter`**: over-engineering. Uma função
  livre resolve. Rejeitado por YAGNI.

---

## D-04. Leitura do JSON: arquivo, stdin, e o literal `-`

**Decision**: Detectar `-` como marker de stdin. Arquivo comum caso
contrário. Erro se arg omitido (obrigatório).

Assinatura pública:

```rust
pub fn load_json_source(source: &str) -> Result<serde_json::Value, TexError>
```

- Se `source == "-"`: `io::stdin().read_to_string` + `from_str`.
- Senão: `fs::read_to_string(source)` + `from_str`.

`InvalidJson { source: "stdin"|"<path>", detail }` para erros de
parse. Path do arquivo faltando (I/O) mapeia para `TexError::Io` →
exit 1 genérico.

**Rationale**:

- Convenção Unix (`-` = stdin) é bem estabelecida (`cat -`,
  `xargs -a -`).
- `String` (não `&[u8]`) porque `serde_json` já valida UTF-8
  implicitamente ao parsear, mensagem de erro fica melhor.
- Bufferizar tudo em `String` antes de parsear é aceitável — JSONs
  de dados de template raramente passam de dezenas de KB.

**Alternatives considered**:

- **`--stdin` flag em vez de `-`**: redundante. `-` é padrão
  universal. Rejeitado.
- **Streaming JSON via `serde_json::from_reader`**: economiza uma
  cópia em memória, mas complica mensagens de erro e não muda o
  ceiling de tamanho. Rejeitado.

---

## D-05. Rejeição de JSON top-level não-objeto

**Decision**: Após `from_str::<Value>`, verificar `value.is_object()`.
Se não, retornar `TexError::InvalidJson { source, detail: "esperado
objeto JSON no topo, recebido <tipo>" }` → exit 31.

**Rationale**:

- Tera não sabe usar `Value::Array` ou `Value::String` como
  contexto raiz. `from_value(non_object)` retorna erro do próprio
  tera com mensagem genérica.
- Nossa mensagem localizada em pt-BR ("esperado objeto…") é mais útil
  do que "expected map".
- Cheap check — `.is_object()` é O(1).

**Alternatives considered**:

- **Deixar tera falhar**: mensagem em inglês, exit genérico.
  Rejeitado por UX.
- **Wrappar array como `{items: [...]}`**: comportamento mágico.
  Rejeitado (mesma justificativa do D-02).

---

## D-06. Extensão de `TexError`: variantes 30 e 31

**Decision**: Adicionar ao enum `TexError`:

```rust
#[error("Falha ao renderizar template '{template_name}': {detail}")]
TeraRenderError {
    template_name: String,
    detail: String,
},

#[error("JSON inválido em {source}: {detail}")]
InvalidJson {
    source: String,   // "stdin" ou path
    detail: String,   // mensagem localizada (line/col do serde_json)
},
```

Mapeamento em `exit_code()`:

- `TeraRenderError { .. }` → **30**
- `InvalidJson { .. }` → **31**

**Rationale**:

- Faixa 30+ mantém consistência com o padrão spec 001 (10-15) →
  spec 002 (20-22) → spec 003 (30-31): cada spec tem sua janela.
- `template_name` no display ajuda debug quando o usuário roda
  `render` como parte de pipeline com múltiplos templates.
- `detail` carrega a mensagem localizada do tera/serde_json (que
  já inclui linha/coluna quando possível — cumpre SC-003).

**Alternatives considered**:

- **Uma única variante `RenderFailed`**: perde distinção
  tera-vs-json que scripts precisam. Rejeitado.
- **Aninhar `#[from] tera::Error`**: acopla o enum público à API
  do tera; se trocarmos a engine no futuro, cascata de mudanças.
  Rejeitado — usamos `detail: String` como buffer.

---

## D-07. Autoescape: `false` fixo, sem configuração

**Decision**: `Tera::one_off(src, ctx, false)` — autoescape sempre
desativado nesta v1.

**Rationale**:

- `.tex` não é HTML/XML. Escapar `&` para `&amp;` corromperia o
  arquivo.
- Configurar por flag/config adiciona superfície sem caso de uso
  real (usuário LaTeX quer literal, sempre).
- Usuário responsável por escapar caracteres reservados do LaTeX
  no próprio template. Documentado nas Assumptions da spec.

**Alternatives considered**:

- **Filtro custom `latex_escape` registrado no tera**: útil,
  mas não é MVP — dá pra adicionar como enhancement futuro sem
  breaking change.
- **Autoescape=true e filtro `raw` universal**: over-engineering.
  Rejeitado.

---

## D-08. Detecção de `--dry-run` prevalecendo sobre `--output`

**Decision**: No handler, checar `args.dry_run` **antes** de calcular
o output path. Se dry-run: renderiza e escreve em `io::stdout()`,
retorna. Se não: usa `--output` (ou default) e chama write atômico.

Se ambos flags forem passados, `--output` é silenciosamente
ignorado. Emitir `warn!` em `-v` para dev que estiver testando.

**Rationale**:

- Semanticamente, `--dry-run` **é** "não grave arquivo". Um path
  de output custom não faz sentido nesse cenário.
- `warn!` só em `-v` evita ruído em uso normal.
- Alternativa "erro quando ambos" seria pedante — dev pode ter deixado
  `--output` no comando reutilizado do histórico shell.

**Alternatives considered**:

- **Erro se ambos**: exit 2 (arg error). Rejeitado por rigidez.
- **`--output` obrigatório mesmo com dry-run**: redundante e
  confuso. Rejeitado.

---

## D-09. Interface do `handle_render` para o modo interativo

**Decision**: Refatorar `handle_render` para aceitar uma struct
`RenderArgs` (produzida por clap OU pelo menu interativo). O menu
constrói a struct e chama o mesmo handler.

```rust
pub struct RenderArgs {
    pub template_name: String,
    pub data_source: String,   // path ou "-"
    pub output: Option<PathBuf>,
    pub dry_run: bool,
    pub force: bool,
}
```

`handle_render_menu()` (chamado quando `Commands::Render` vem sem
subcomando):
- Guard TTY (mesmo padrão do `templates` menu — spec 002).
- Select do template (via `list_templates` + `prompt_template_name`
  da spec 002 — reuso direto).
- Text pro path do JSON.
- Confirm pro dry-run.
- Constrói `RenderArgs` e chama `handle_render(args)`.

**Rationale**:

- **Single choke point**: mesma lógica de renderização, dois
  entry points. Elimina drift.
- Reuso de helpers da spec 002 (`prompt_template_name`) — zero código
  duplicado.

**Alternatives considered**:

- **Duas funções paralelas**: risco de drift. Rejeitado.
- **Menu como wrapper que executa `render` no shell**: bug-prone e
  contrária à filosofia self-contained. Rejeitado.

---

## D-10. Extração de `write_atomic_0644` — momento e onde

**Decision**: Extrair para `src/atomic.rs` como parte desta spec
(dentro de Phase 2 Foundational), **antes** de US1 começar a
consumir. Assinatura: `pub fn write_atomic(target: &Path, bytes:
&[u8], mode: u32) -> Result<(), TexError>`.

Atualizar `src/templates.rs::add_template` para chamar o helper
extraído (rename do `write_atomic_0644` privado desaparece; virou
`atomic::write_atomic(dest_path, &bytes, 0o644)`).

`src/config.rs::save_atomic` **fica como está nesta spec** — o
mode `0o600` e a serialização toml embutida têm particularidades
que não valem a refactor agora. Anotado para spec futura ou tarefa
de polish.

**Rationale**:

- Extrai **quando** o segundo consumidor aparece (regra dos três,
  seguindo YAGNI). Config foi consumidor #1 (mode 0600, escreve
  toml), templates consumidor #2 (mode 0644, escreve bytes), render
  consumidor #3 (mode 0644, escreve bytes) — a partir daqui vale.
- Templates + render usam o mesmo shape (bytes + mode), então
  extração fica limpa.
- Config fica de fora porque `Config::save_atomic` serializa
  internamente com `toml::to_string_pretty` — não recebe bytes de
  fora.

**Alternatives considered**:

- **Extrair depois** (quando spec 004 vier): drift entre templates
  e render nesta spec. Rejeitado.
- **Extrair também o de config**: `save_atomic<T: Serialize>` genérico
  vira over-engineering. Rejeitado.
