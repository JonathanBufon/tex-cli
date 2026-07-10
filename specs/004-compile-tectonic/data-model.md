# Phase 1 — Data Model: Compilação `.tex` → PDF (Compilador Plugável)

**Feature**: `004-compile-tectonic`
**Date**: 2026-07-10

Modelo de dados desta feature — todas as entidades são efêmeras
(não persistidas). Reaproveita `Config`, `atomic::write_atomic`,
`resolve_template`/`read_template` das specs anteriores sem
modificações.

Cross-reference: extende `TexError` conforme D-08 do research.

---

## Entidade: `SupportedEngine`

Enum fechado representando as engines LaTeX que a v1 aceita. Não é
persistido — usado internamente pra dispatch e validação em tempo
de argumento CLI.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedEngine {
    Tectonic,
    Latexmk,
    Pdflatex,
    Xelatex,
    Lualatex,
}

impl SupportedEngine {
    pub const ALL: &'static [SupportedEngine] = &[
        SupportedEngine::Tectonic,
        SupportedEngine::Latexmk,
        SupportedEngine::Pdflatex,
        SupportedEngine::Xelatex,
        SupportedEngine::Lualatex,
    ];

    pub fn as_str(&self) -> &'static str { /* nome canônico */ }
    pub fn binary_name(&self) -> &'static str { /* alias do binário */ }
    pub fn args_for(&self, tex_filename: &str) -> Vec<String> { /* args D-03 */ }
}

impl FromStr for SupportedEngine {
    type Err = TexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { /* exact match */ }
}

impl Display for SupportedEngine {
    fn fmt(...) { /* delega a as_str */ }
}
```

Mapeamento canônico:

| Variante  | `as_str()`   | `binary_name()` | Args (D-03)                                                       |
|-----------|--------------|-----------------|-------------------------------------------------------------------|
| Tectonic  | `"tectonic"` | `"tectonic"`    | `["--outdir=.", "--keep-logs", "--keep-intermediates", <tex>]`   |
| Latexmk   | `"latexmk"`  | `"latexmk"`     | `["-pdf", "-interaction=nonstopmode", "-halt-on-error", <tex>]`  |
| Pdflatex  | `"pdflatex"` | `"pdflatex"`    | `["-interaction=nonstopmode", "-halt-on-error", <tex>]`          |
| Xelatex   | `"xelatex"`  | `"xelatex"`     | `["-interaction=nonstopmode", "-halt-on-error", <tex>]`          |
| Lualatex  | `"lualatex"` | `"lualatex"`    | `["-interaction=nonstopmode", "-halt-on-error", <tex>]`          |

Nota: `as_str()` e `binary_name()` retornam a mesma coisa hoje,
mas mantemos APIs separadas para o caso futuro onde um alias seja
introduzido (ex.: uma variante `LatexmkFast` mapeando pro binário
`latexmk` com args diferentes).

---

## Entidade: `CompileRequest` (interno)

Estrutura efêmera construída no handler consolidando args CLI +
config resolvidos. Não é público — é o input do
`compile_and_write`.

```rust
pub(crate) struct CompileRequest {
    pub tex_path: PathBuf,      // path do .tex fonte (absoluto ou relativo)
    pub engine: SupportedEngine,
    pub output_pdf: PathBuf,    // path final do PDF (default ou --output)
    pub keep_tex: bool,         // resolvido: --keep-tex/--no-keep-tex/config
    pub keep_logs: bool,        // idem
    pub force: bool,
}
```

Regras de construção (todas no handler `handle_compile`):

- `tex_path` = arg positional (canonicalizado via
  `paths::expand_user_path` se relativo).
- `engine`: `--engine` presente → parse via `FromStr`; ausente →
  `config.compiler.engine` (também parseado; se falhar,
  `EngineNotSupported`).
- `output_pdf`: `--output` presente → `expand_user_path`; ausente
  → `config.paths.output_dir.join(format!("{}.pdf", basename))`.
- `keep_tex`: `--keep-tex` → true; `--no-keep-tex` → false; nenhum
  → `config.compiler.keep_tex`.
- `keep_logs`: idem.
- `force`: valor direto do flag.

---

## Entidade: `CompileOutcome`

Retornado por `compile_and_write` para o handler compor stdout.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutcome {
    pub pdf_path: PathBuf,
    pub duration: std::time::Duration,
    pub bytes_written: u64,
    pub overwrote_existing: bool,
    pub kept_tex: bool,          // reflete o path final; true se copiou
    pub kept_logs: bool,
}
```

---

## Extensão de `TexError`

Três novas variantes conforme D-08 do research:

```rust
#[error(
    "Falha ao compilar '{}': engine '{engine}' retornou erro.\n\
     Últimas linhas do log:\n{log_tail}\n",
    tex_path.display()
)]
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

`TexError::exit_code()` estendido:

- `CompileFailed { .. }` → **40**
- `EngineNotInstalled { .. }` → **41**
- `EngineNotSupported { .. }` → **42**

Todas as demais entradas mantidas (10..15, 20..22, 30..31, 1).

---

## Regras de validação por operação

### `resolve_engine(cli_flag: Option<&str>, config_engine: &str) -> Result<SupportedEngine, TexError>`

| Cenário                                       | Comportamento                                   |
|-----------------------------------------------|-------------------------------------------------|
| `cli_flag = Some(s)` e `s` na lista canônica  | `Ok(SupportedEngine::from_s)`                   |
| `cli_flag = Some(s)` e `s` fora da lista      | `Err(EngineNotSupported { engine: s, accepted })` → exit 42 |
| `cli_flag = None`                             | Delega a `config_engine` (mesma lógica; se config tem engine inválido, exit 42) |

### `validate_binary(engine: SupportedEngine) -> Result<PathBuf, TexError>`

Chama `which::which(engine.binary_name())`. Se falhar (não está no
PATH): `EngineNotInstalled { engine: engine.to_string() }` → exit 41.
Se sucesso: retorna o path do binário (para logging e uso opcional).

### `run_engine(engine, temp_dir, tex_filename) -> Result<(), (i32, String)>`

- Cwd = `temp_dir`.
- Args = `engine.args_for(tex_filename)`.
- Verbose≥DEBUG: `Stdio::inherit()` pra stdout+stderr.
- Verbose<DEBUG: `Command::output()`, captura tudo.
- Retorno:
  - `Ok(())` se `ExitStatus::success()` **E** PDF esperado existe
    no temp_dir.
  - `Err((exit_code, log_tail))` senão. `log_tail` construído
    lendo `<temp>/<basename>.log` (últimas 30 linhas) ou fallback
    pro stderr capturado.

### `compile_and_write(cfg: &Config, args: &CompileRequest) -> Result<CompileOutcome, TexError>`

Fluxo:

1. Verifica `args.tex_path.exists()`; senão erro I/O NotFound →
   exit 1.
2. `validate_binary(args.engine)?` → exit 41 possível.
3. `let temp = TempDir::new()?;`
4. `fs::copy(&args.tex_path, temp.path().join(basename))?`.
5. `start = Instant::now();`
6. `run_engine(args.engine, temp.path(), &basename)?` → em erro,
   monta `TexError::CompileFailed { engine, tex_path, log_tail }`
   → exit 40.
7. `let duration = start.elapsed();`
8. Se `output_pdf` já existe e `!args.force`:
   - TTY → `confirm_compile_overwrite` prompt.
   - Não-TTY → `UserAborted` (exit 15).
9. Lê PDF bytes do temp; `atomic::write_atomic(&output_pdf, &bytes,
   0o644)?`.
10. Se `args.keep_tex`: `atomic::write_atomic(<output_dir>/<basename>.tex,
    &fs::read(&args.tex_path)?, 0o644)?`.
11. Se `args.keep_logs`: mesmo pattern pro `.log`.
12. TempDir vai fora de scope aqui → Drop apaga tudo.
13. Retorna `CompileOutcome { pdf_path, duration, bytes_written,
    overwrote_existing, kept_tex, kept_logs }`.

---

## Não é entidade

- **Args exatos passados pro child** (`Vec<String>`): efêmero,
  computado em `run_engine`.
- **Buffers de stdout/stderr do child**: descartados em sucesso,
  usados só para o `log_tail` em falha.
- **TempDir**: gerenciado por RAII (Drop), não vaza pra fora.
