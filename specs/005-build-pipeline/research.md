# Phase 0 — Research: Pipeline JSON → PDF (`build`)

**Feature**: `005-build-pipeline`
**Date**: 2026-07-10

Resolução das decisões técnicas do plan. Registra rationale e
alternativas rejeitadas.

Cross-reference: reaproveita 100% das decisões das specs 001-004
(atomic write, banner, exit-code mapping, IsTerminal guards,
inquire prompts, TempDir pattern, SupportedEngine enum, `--keep-*`
flag pattern). Só decisões **novas ou específicas do orquestrador**
aparecem aqui.

---

## D-01. Orquestração: novo módulo `build.rs` vs. handler direto em `cli.rs`

**Decision**: Novo módulo `src/build.rs` contendo `build_pipeline`
(função pura de orquestração) + `BuildOutcome` struct. O handler
`handle_build` em `src/cli.rs` fica curto: parse args, chama
`build_pipeline`, formata stdout.

**Rationale**:

- Simetria com specs 003/004 que têm `src/render.rs` e
  `src/compiler.rs` — o módulo é o "cerne", o handler é a "casca".
- Testável isoladamente via unit tests (o handler exige
  `assert_cmd` + processo real).
- Se surgir a necessidade futura de expor `build_pipeline` como
  API pública (ex.: uma spec 006 que orquestra várias `build`s),
  fica trivial.

**Alternatives considered**:

- **Handler inline em `cli.rs`**: ~60 linhas na `handle_build`
  quebrariam a coesão do arquivo. Rejeitado.
- **Trait `Pipeline` abstrata**: over-engineering. Rejeitado por
  YAGNI.

---

## D-02. Ordem das etapas: read → parse → render → keep_tex → compile

**Decision**: Ordem canônica:

1. `read_template(dir, name)` → bytes.
2. `load_json_source(source)` → `Value`.
3. `render_template(name, tpl_src, &value)` → String.
4. **Se `keep_tex=true`**: `atomic::write_atomic(<output_dir>/<name>.tex,
   rendered.as_bytes(), 0o644)`.
5. Cria `TempDir`, escreve `rendered` em `<temp>/<name>.tex`.
6. `compile_and_write(cfg, <temp>/<name>.tex, engine, output_pdf,
   /* keep_tex=false — já fizemos manualmente na etapa 4 */,
   keep_logs, force, verbose)`.
7. Retorna `BuildOutcome`.

**Rationale**:

- **Etapa 4 vem antes da 5/6** para satisfazer FR-015: se compile
  falhar, o `.tex` intermediário permanece no `output_dir` pra
  debug do usuário.
- Passamos `keep_tex=false` para `compile_and_write` porque o
  `build` já tratou o `.tex` na etapa 4 — evita cópia dupla.
- `keep_logs` é passado como está — só faz sentido depois do
  compile.

**Alternatives considered**:

- **Compilar primeiro, gravar `.tex` só se sucesso**: viola FR-015
  (usuário perde debug em falha). Rejeitado.
- **Sempre passar `keep_tex=true` para compile e não gravar
  manualmente**: `compile_and_write` só grava o `.tex` **após**
  o compile bem-sucedido — não cobre FR-015. Rejeitado.

---

## D-03. Path do `.tex` intermediário quando `keep_tex=true`

**Decision**: `<output_dir>/<template_name>.tex` (mesmo basename
do PDF final). Escrita atômica com mode `0o644`.

**Rationale**:

- Consistente com o path que o `render <tpl> <data>` já grava por
  default (spec 003).
- Basename = template_name para consistência com o PDF (que também
  usa template_name — assumption da spec).
- `atomic::write_atomic` cria dirs pais se necessário.

**Alternatives considered**:

- **`<output_dir>/<template_name>.build.tex` (sufixo `.build`)**:
  diferencia de um `.tex` produzido por `render` isolado. Rejeitado
  — usuário fica confuso com dois padrões de naming.
- **Escrever em `<output_dir>/build/<template_name>.tex`**: cria
  subdir sem valor. Rejeitado.

---

## D-04. `BuildOutcome`: exposição das durations

**Decision**: `BuildOutcome` expõe três `Duration`s:
`total_duration`, `render_duration`, `compile_duration`. Handler
formata stdout usando `total_duration` (round to 1 decimal).

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    pub pdf_path: PathBuf,
    pub total_duration: Duration,
    pub render_duration: Duration,
    pub compile_duration: Duration,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
    pub kept_tex: bool,
    pub kept_logs: bool,
    pub intermediate_tex_path: Option<PathBuf>,
}
```

**Rationale**:

- `total_duration` é o número que aparece na stdout final (SC-001
  budget é sobre isso).
- `render_duration` + `compile_duration` são úteis pra `-v` (logs
  granulares) e possíveis testes de regressão de performance.
- `intermediate_tex_path` é `Some(...)` sse `keep_tex=true` — permite
  o handler mencionar no stdout se o usuário quiser inspecionar
  depois.

**Alternatives considered**:

- **Só `total_duration`**: perde granularidade útil para debug.
  Rejeitado.
- **Reusar `CompileOutcome` e ignorar duração do render**: SC-002
  exige medição do overhead — precisamos das duas. Rejeitado.

---

## D-05. Formato do stdout final

**Decision**: Uma única linha com formato canônico:

```text
PDF gerado em <path-absoluto>. Pipeline (render + compile) levou X.Ys.
```

Ou com "sobrescrito" se aplicável:

```text
PDF gerado (sobrescrito) em <path>. Pipeline (render + compile) levou X.Ys.
```

**Rationale**:

- Mesmo pattern do `compile` da spec 004 (`PDF gerado em … Compilação
  levou X.Ys.`) — usuário reconhece imediatamente.
- Menciona "(render + compile)" explicitamente pra deixar claro que
  o tempo é combinado.
- Uma casa decimal (`{:.1}s`).

**Alternatives considered**:

- **Múltiplas linhas com detalhamento por etapa**: sujaria a
  saída pra scripts (`$(tex-cli build ... | head -1)`). Rejeitado.
- **Formato JSON**: fora do escopo v1 (assumption da spec).

---

## D-06. Interação com `compile_and_write`: parâmetro `keep_tex`

**Decision**: `build` passa `keep_tex=false` para
`compile_and_write` porque o `build` já gravou o `.tex` manualmente
na etapa 4 (se `keep_tex=true` para o build). Isso evita dupla
gravação.

Semanticamente:

- Do ponto de vista do usuário: `--keep-tex` no `build` = mantém
  o `.tex` no output_dir.
- Do ponto de vista do compile: recebemos `keep_tex=false` porque
  o `build` já lida com o `.tex` (o compile só precisa gravar o
  PDF).

**Rationale**:

- Duplicar `atomic::write_atomic(<output_dir>/<name>.tex, ...)` em
  ambos os caminhos causaria I/O redundante.
- A ordem correta (etapa 4 antes de compile) já satisfaz FR-15.

**Alternatives considered**:

- **Passar `keep_tex=<user_flag>` para compile**: compile gravaria o
  `.tex` uma segunda vez após sucesso — desperdício, e ainda quebra
  FR-15 (o `.tex` só apareceria após compile bem-sucedido).
  Rejeitado.

---

## D-07. Reuso dos prompts existentes no modo interativo

**Decision**: Reusar sem modificação:

- `prompt_template_name` (spec 002) — Select do template.
- `prompt_json_source` (spec 003) — Text pro path do JSON.
- `confirm_keep_tex(default)` (spec 004) — Confirm.
- `confirm_keep_logs(default)` (spec 004) — Confirm.

Nenhum novo prompt precisa ser adicionado ao `src/interactive.rs`.

**Rationale**:

- Consistência UX: usuário vê os mesmos prompts em
  `render`/`compile`/`build` — reduz superfície de aprendizado.
- Zero código duplicado.

**Alternatives considered**:

- **Prompt custom `prompt_build_config`**: over-engineering.
  Rejeitado.

---

## D-08. Menu interativo do `build`: order e defaults

**Decision**: Ordem canônica:

1. Select template (`prompt_template_name(&names)`).
2. Text data path (`prompt_json_source()`).
3. Confirm keep_tex (default = `cfg.compiler.keep_tex`).
4. Confirm keep_logs (default = `cfg.compiler.keep_logs`).

**Rationale**:

- Ordem lógica: template primeiro (identidade), depois dados
  (input), depois flags (behavior modifiers).
- Defaults vêm do config → menu respeita a preferência já
  configurada; usuário só desvia se quiser.

**Alternatives considered**:

- **Ordem inversa (flags primeiro)**: contra-intuitivo; usuário
  ainda não sabe o que vai fazer. Rejeitado.

---

## D-09. `--engine` no modo interativo

**Decision**: O menu interativo **não** oferece prompt para
`--engine`. Sempre usa o engine do config. Usuário que quer
sobrescrever engine na invocação usa o subcomando direto.

**Rationale**:

- Menu interativo é para casos casuais/exploratórios; engine
  override é caso avançado.
- Reduz cognitive load do menu (4 prompts em vez de 5).
- Consistente com o menu do `compile` da spec 004 (também não
  oferece engine).

**Alternatives considered**:

- **Adicionar Select de engine ao menu**: aumenta atrito para o
  99% dos casos que só usam tectonic. Rejeitado.

---

## D-10. Logging por etapa quando `-v` (INFO)

**Decision**: Emitir três `tracing::info!` no fluxo:

```
info!("Renderizando template '{name}'...");
info!("Render concluído em {:.0}ms", render_duration.as_millis());
info!("Compilando com engine '{engine}'...");
info!("Compile concluído em {:.1}s", compile_duration.as_secs_f32());
```

Sem `-v`: silent (só stdout final).

Com `-vv`: `compile_and_write` já herda o `verbose=2` do chamador
que faz `Stdio::inherit()` para stream em tempo real do child
process (comportamento da spec 004).

**Rationale**:

- `-v` (INFO) atende SC-007 ("mensagens identificam etapa onde
  falhou") — usuário sabe se render terminou ou não.
- `-vv` propaga o comportamento de streaming do compile.
- Consistente com pattern das specs 003/004.

**Alternatives considered**:

- **Sem logging** — usuário não sabe onde travou. Rejeitado.
- **Log em stderr direto (sem `tracing`)** — quebra o padrão do
  projeto. Rejeitado.
