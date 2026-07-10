# Phase 0 — Research: Compilação `.tex` → PDF (Compilador Plugável)

**Feature**: `004-compile-tectonic`
**Date**: 2026-07-10

Resolução das decisões técnicas do plan. Registra rationale e
alternativas rejeitadas.

Cross-reference: reaproveita decisões das specs 001–003 (atomic
write, banner, exit-code mapping, IsTerminal guards, inquire
prompts, TempDir pattern do render). Só decisões **novas ou
expandidas** aparecem aqui.

---

## D-01. Como invocar o engine LaTeX: subprocess vs. embedding

**Decision**: `std::process::Command` como subprocess externo.
Todos os engines suportados (tectonic, latexmk, pdflatex, xelatex,
lualatex) são binários instaláveis via package manager ou via
install script.

**Rationale**:

- Zero nova dep. `std::process::Command` é stdlib.
- Uniforme entre engines — mesmo pattern serve pra todos.
- Debugging trivial: usuário pode reproduzir na mão copiando o
  argv do log verbose.
- Isolamento: child pode ser SIGKILL'd sem risco pro processo pai.
- Testes de integração podem mockar via `PATH=/tmp/empty` (padrão
  já usado na spec 001 para o test de tectonic ausente).

**Alternatives considered**:

- **Bindings Rust do tectonic** (crate `tectonic`): existe mas
  puxa massa nativa gigante (libpng, freetype, harfbuzz), quebra
  hermeticidade do compilador plugável (força tectonic-only ou
  crate feature complexo), e o binário `tex-cli` engordaria de
  ~10 MB pra >100 MB. Rejeitado — viola Princípio IV.
- **Sistema de plugins dinâmicos** (dlopen etc.): over-engineering
  para 5 engines conhecidos. Rejeitado por YAGNI.

---

## D-02. Enum `SupportedEngine`: fechado, sem alias

**Decision**: Enum Rust fechado com exatamente 5 variantes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedEngine {
    Tectonic,
    Latexmk,
    Pdflatex,
    Xelatex,
    Lualatex,
}
```

`impl FromStr` matches exatos (sem case-insensitive, sem aliases —
mesmo pattern do `ConfigKey` da spec 001).

**Rationale**:

- Compilador enforce completude: adicionar variante quebra o build
  em qualquer `match` sem arm — evita bugs silenciosos.
- FR-006 exige exatamente essa lista. Aliases (`plain-tex` →
  `tectonic`, `luatex` → `lualatex`) só criam ambiguidade em erro.
- Matching exato ajuda scripts CI a serem determinísticos.

**Alternatives considered**:

- **String livre + validação em runtime**: menos type-safe,
  mensagens de erro pobres. Rejeitado.
- **Alias map** (`plain-tex` → `tectonic`): não pedido pelo spec,
  YAGNI. Rejeitado.

---

## D-03. Args por engine

**Decision**: Método `args_for(tex_filename: &str) -> Vec<String>`
mapeando cada engine:

| Engine     | Args                                                              |
|------------|-------------------------------------------------------------------|
| Tectonic   | `["--outdir=.", "--keep-logs", "--keep-intermediates", <tex>]`   |
| Latexmk    | `["-pdf", "-interaction=nonstopmode", "-halt-on-error", <tex>]`  |
| Pdflatex   | `["-interaction=nonstopmode", "-halt-on-error", <tex>]`          |
| Xelatex    | `["-interaction=nonstopmode", "-halt-on-error", <tex>]`          |
| Lualatex   | `["-interaction=nonstopmode", "-halt-on-error", <tex>]`          |

**Rationale**:

- Tectonic: `--outdir=.` grava artefatos no cwd (que é o TempDir);
  `--keep-logs` garante `.log` disponível pra tail em erros;
  `--keep-intermediates` mantém `.aux` etc. dentro do TempDir (não
  vazam).
- Latexmk: `-pdf` força output PDF; `-interaction=nonstopmode` +
  `-halt-on-error` fazem falha explícita em vez de prompt
  interativo do LaTeX.
- pdflatex/xelatex/lualatex: mesmo pattern non-stop + halt-on-error;
  single-pass (múltiplas passes de bib estão fora do escopo v1).

**Alternatives considered**:

- **`-file-line-error` em pdflatex-family**: melhora format de
  erro. Considerar como enhancement v2 — não bloqueante.
- **Modo bibtex explícito**: fora do escopo v1 (assumption
  explícita da spec).

---

## D-04. Directory strategy: TempDir + copy pra output

**Decision**: `tempfile::TempDir::new()?` cria dir em `$TMPDIR` (default
`/tmp`). Fluxo:

1. Copia `.tex` fonte → `<temp>/<basename>.tex`.
2. Executa `engine` com `cwd = <temp>`, args conforme D-03.
3. Se sucesso: PDF em `<temp>/<basename>.pdf` → lido pra bytes →
   `atomic::write_atomic(<dest>, &bytes, 0o644)`.
4. Se `keep_tex`: `fs::copy(<temp>/<basename>.tex,
   <output_dir>/<basename>.tex)` (ou re-write atomicamente).
5. Se `keep_logs`: mesmo padrão pro `.log`.
6. `TempDir` cai fora de scope no final — Drop apaga tudo.

**Rationale**:

- Isola artefatos LaTeX (`.aux`, `.bbl`, `.blg`, `.out`, `.toc`,
  etc.) que não interessam ao usuário.
- Cwd = TempDir garante que engine encontra o `.tex` sem path
  navigation.
- Cleanup automático via Drop — cumpre SC-006 (zero resíduo).
- Escrita atômica no destino via helper compartilhado — mesma
  garantia da spec 003.

**Alternatives considered**:

- **Compilar no dir do `.tex` original**: polui o dir do usuário
  com `.aux/.log/.bbl` residuais. Rejeitado por UX/higiene.
- **In-memory compile via bindings**: viola D-01. Rejeitado.
- **Copiar todo o dir do `.tex` fonte** (pra pegar assets como
  `.bib`, imagens): fora do escopo v1 (assumption). Se necessário,
  spec futura adiciona `--include-dir <path>` que copia recursivo.

---

## D-05. `.tex` no TempDir: cópia via `fs::copy` vs. leitura+write

**Decision**: `std::fs::copy(source_path, temp.path().join(basename))`
— cópia direta, sem passar por buffer em memória.

**Rationale**:

- `fs::copy` já preserva bytes (não faz UTF-8 validation).
- Rápido: syscall único no kernel (typicamente).
- Se falhar (permissão, disco cheio), erro claro.

**Alternatives considered**:

- **Read + write**: mesmo resultado, mais código. Rejeitado.
- **Symlink em vez de copy**: complica cleanup (Drop do TempDir
  não segue symlink). Rejeitado.

---

## D-06. Captura de stdout+stderr do child: buffer completo vs. streaming

**Decision**: `Command::output()` (blocking, buffer completo em
`Vec<u8>`) — captura tudo, decide o que fazer no final.

Quando `verbose >= DEBUG`: reroteia via `Command::spawn` +
`Child::stdout(Stdio::inherit())`, `stderr(Stdio::inherit())` pra
streaming visível.

**Rationale**:

- Buffer é aceitável — `.log` de LaTeX raramente passa de MBs.
- Simples: um path de código para o caso default (silent).
- Em erro (exit code ≠ 0), o buffer é imediatamente
  transformável em `tail_lines(&stderr_or_log, 30)`.

**Alternatives considered**:

- **Sempre streaming** (`Stdio::inherit`): pollui a tela em uso
  normal. Rejeitado.
- **Streaming via thread + channel + buffer**: complexidade
  adicional pra um caso onde o buffer completo funciona bem.
  Rejeitado.

---

## D-07. Formato do `log_tail` na `TexError::CompileFailed`

**Decision**: `log_tail: String` com as **últimas 30 linhas**
(joined por `\n`) do arquivo `.log` do engine (não do stderr do
subprocess). Se `.log` não existe: fallback pro stderr capturado.

Helper `pub fn tail_lines(s: &str, n: usize) -> String` no
`src/compiler.rs`, testado como unit.

**Rationale**:

- `.log` do LaTeX tem mais contexto (linhas com `l.42` apontando
  arquivo:linha, marcadores `!` em erros) do que o stderr do
  subprocess.
- 30 linhas é o "sweet spot" observado: cobre mensagem de erro +
  contexto sem inundar a stderr do CLI.
- Fallback pro stderr do processo cobre engines/casos onde `.log`
  não é gerado (raro).

**Alternatives considered**:

- **Log inteiro**: barulhento; cortaria fluxo scriptável.
  Rejeitado.
- **Últimas N linhas com marcadores `!`**: heurística frágil se
  engine não seguir convenção. Rejeitado.
- **Configurável via flag `--log-tail <n>`**: over-engineering para
  v1. Rejeitado.

---

## D-08. Extensão de `TexError`: variantes 40/41/42

**Decision**: Adicionar ao enum `TexError`:

```rust
#[error("Falha ao compilar '{}': engine '{engine}' retornou erro.\n\
         Últimas linhas do log:\n{log_tail}\n",
        tex_path.display())]
CompileFailed {
    engine: String,
    tex_path: PathBuf,
    log_tail: String,
},

#[error("Engine '{engine}' não está instalado no PATH. \
         Instale-o antes ou use --engine <outro>.")]
EngineNotInstalled { engine: String },

#[error("Engine '{engine}' não é suportado. Aceitos:\n{}",
        format_accepted(accepted))]
EngineNotSupported {
    engine: String,
    accepted: Vec<&'static str>,
},
```

Mapeamento em `exit_code()`:

- `CompileFailed { .. }` → **40**
- `EngineNotInstalled { .. }` → **41**
- `EngineNotSupported { .. }` → **42**

Reuso do helper `format_accepted` já existente em `errors.rs`
(usado em `UnknownKey` da spec 001).

**Rationale**:

- Faixa 40+ mantém consistência com o padrão spec 001 (10-15) →
  spec 002 (20-22) → spec 003 (30-31) → spec 004 (40-42): cada
  spec sua janela.
- `log_tail` no Display é útil pro usuário ver imediatamente o
  problema sem consultar arquivo separado.

**Alternatives considered**:

- **Variante única `CompileError` com kind interno**: perde
  distinção pra scripts CI. Rejeitado.
- **Não incluir `log_tail` no Display, só via getter**: usuário
  precisaria de `--verbose` extra pra ver a mensagem essencial.
  Rejeitado.

---

## D-09. `--keep-tex` / `--no-keep-tex`: par de flags booleanas

**Decision**: Duas flags booleanas explícitas, mutualmente
exclusivas via `clap` (`conflicts_with`):

```rust
#[arg(long, conflicts_with = "no_keep_tex")]
pub keep_tex: bool,
#[arg(long = "no-keep-tex", conflicts_with = "keep_tex")]
pub no_keep_tex: bool,
```

Resolução no handler:

- `keep_tex=true` → override `true`.
- `no_keep_tex=true` → override `false`.
- Nenhum → usa `config.compiler.keep_tex`.

Idêntico para `keep_logs` / `no_keep_logs`.

**Rationale**:

- clap oferece a conflict natively; conflitos disparam exit 2
  (arg error).
- Padrão semelhante ao `cargo build --release` / (implícito debug
  padrão).
- Explícito é melhor que valor tri-state customizado (Option<bool>
  no clap requer `--keep-tex=true` que fere convenção Unix).

**Alternatives considered**:

- **`clap::builder::BoolishValueParser` com um único flag**:
  `--keep-tex[=<bool>]` funciona mas menos convencional em CLI.
  Rejeitado por preferência ergonômica.

---

## D-10. Detecção de PDF gerado (sucesso vs. falha)

**Decision**: Após `Command::output()`, decidir sucesso apenas por
`ExitStatus::success()`. Se sucesso, procurar `<temp>/<basename>.pdf`;
se **existir**, considerar compilação bem-sucedida. Se **não existir**
apesar de exit 0 (raro; alguns engines exit 0 sem produzir PDF em
docs vazios), tratar como `CompileFailed` com log_tail e
`log_tail: "Engine retornou sucesso mas nenhum PDF foi produzido"`.

**Rationale**:

- Cinturão + suspensórios: o exit code do engine é primário, mas
  presença do PDF é a verdade real do artefato desejado.
- Cobre corner case de "engine compilou algo mas não gerou PDF"
  (ex.: erro pré-`\begin{document}` mas engine limpo).

**Alternatives considered**:

- **Apenas exit code**: perde a verificação de "produziu artefato".
  Rejeitado.
- **Verificar PDF magic bytes**: overkill em unit level;
  integração cobre isso via teste.

---

## D-11. Duração no `stdout` de sucesso

**Decision**: `Instant::now()` capturado antes de spawn do engine;
`elapsed()` medido após `output()` retornar. Formatado como
`X.Ys` (uma casa decimal) em stdout final:

```
PDF gerado em /home/u/tex/out/artigo.pdf. Compilação levou 1.4s.
```

**Rationale**:

- Feedback útil pro usuário — sabe quanto tempo tomou.
- Uma casa decimal é suficiente (millisegundos são ruído).
- Instant → Duration → `as_secs_f32()` faz a formatação natural.

**Alternatives considered**:

- **Sem duração**: menos informativo. Rejeitado.
- **Duração só no `-v` (INFO)**: dev quer saber sem verbose.
  Rejeitado.
