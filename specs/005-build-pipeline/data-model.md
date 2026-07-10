# Phase 1 — Data Model: Pipeline JSON → PDF (`build`)

**Feature**: `005-build-pipeline`
**Date**: 2026-07-10

Modelo de dados desta feature — todas as entidades são efêmeras
(não persistidas). **Zero mudança no `TexError`**. Toda propagação
de erro reusa variantes das specs 001-004.

Cross-reference: reusa `Config`, `SupportedEngine`,
`render::render_template`, `render::load_json_source`,
`compiler::compile_and_write`, `atomic::write_atomic`,
`templates::read_template` sem modificação.

---

## Entidade: `BuildArgs` (público em `src/cli.rs`)

Parseado por clap; espelha o contrato observável.

```rust
#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Nome do template (opcional para modo interativo).
    pub template_name: Option<String>,

    /// Path do JSON no host OU `-` para stdin (opcional para menu).
    pub data_source: Option<String>,

    /// Path custom do PDF final.
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Sobrescreve compiler.engine só nesta invocação.
    #[arg(short = 'e', long)]
    pub engine: Option<String>,

    /// Copia .tex intermediário para output_dir.
    #[arg(long, conflicts_with = "no_keep_tex")]
    pub keep_tex: bool,

    /// Descarta .tex intermediário mesmo se config diz true.
    #[arg(long = "no-keep-tex", conflicts_with = "keep_tex")]
    pub no_keep_tex: bool,

    /// Copia .log do engine para output_dir.
    #[arg(long, conflicts_with = "no_keep_logs")]
    pub keep_logs: bool,

    /// Descarta .log mesmo se config diz true.
    #[arg(long = "no-keep-logs", conflicts_with = "keep_logs")]
    pub no_keep_logs: bool,

    /// Sobrescreve PDF existente sem prompt.
    #[arg(long)]
    pub force: bool,
}
```

Regras de construção no handler:

| Campo         | Resolução                                              |
|---------------|--------------------------------------------------------|
| template_name | arg positional OU Select do menu                       |
| data_source   | arg positional OU Text do menu                         |
| output_pdf    | `--output` (expandido) OU default `output_dir/<name>.pdf` |
| engine        | `--engine` OR `cfg.compiler.engine` (parseado)         |
| keep_tex      | flag → override; senão `cfg.compiler.keep_tex`         |
| keep_logs     | flag → override; senão `cfg.compiler.keep_logs`        |
| force         | valor direto do flag                                   |

---

## Entidade: `BuildOutcome`

Retornado por `build_pipeline` para o handler compor stdout.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    /// Path final do PDF gravado.
    pub pdf_path: PathBuf,

    /// Soma render + compile + I/O de escrita.
    pub total_duration: Duration,

    /// Só o render (sem compile, sem I/O).
    pub render_duration: Duration,

    /// Só o compile (sem render, sem I/O do intermediate).
    pub compile_duration: Duration,

    /// Tamanho do PDF em bytes.
    pub bytes_written: u64,

    /// true sse sobrescreveu PDF pré-existente.
    pub overwrote_existing: bool,

    /// Espelha o valor efetivo de keep_tex.
    pub kept_tex: bool,

    /// Espelha o valor efetivo de keep_logs.
    pub kept_logs: bool,

    /// Path do .tex intermediário no output_dir, se kept_tex=true.
    pub intermediate_tex_path: Option<PathBuf>,
}
```

---

## Contrato de `build_pipeline` (função principal do módulo)

Assinatura pública:

```rust
pub fn build_pipeline(
    cfg: &Config,
    template_name: &str,
    data_source: &str,
    output_pdf: &Path,
    engine: SupportedEngine,
    keep_tex: bool,
    keep_logs: bool,
    force: bool,
    verbose: u8,
) -> Result<BuildOutcome, TexError>
```

Fluxo (research D-02):

1. `templates::read_template(&cfg.paths.templates_dir, template_name)?`
   → bytes.
2. `let template_src = std::str::from_utf8(&bytes).map_err(|e|
   TexError::InvalidUtf8 { … })?` (defensivo — templates add já
   validou, mas cinturão-suspensórios).
3. `let value = render::load_json_source(data_source)?`.
4. `let start_render = Instant::now();`
5. `let rendered = render::render_template(template_name,
   template_src, &value)?`.
6. `let render_duration = start_render.elapsed();`
7. **Se `keep_tex`**:
   `atomic::write_atomic(<output_dir>/<name>.tex, rendered.as_bytes(),
   0o644)?`. Guarda o path retornado em `intermediate_tex_path`.
8. Cria `TempDir::new()?`.
9. `let tex_filename = format!("{name}.tex");`
10. `fs::write(temp.path().join(&tex_filename), &rendered)?`.
11. `let start_compile = Instant::now();`
12. **Chama `compiler::compile_and_write(cfg, &tex_in_temp, engine,
    output_pdf, /* keep_tex= */ false, keep_logs, force, verbose)?`**
    (passamos `keep_tex=false` porque já lidamos com o .tex na
    etapa 7 — research D-06).
13. `let compile_duration = start_compile.elapsed();`
14. Constrói `BuildOutcome { pdf_path: output_pdf.clone(),
    total_duration: start_render.elapsed(),
    render_duration, compile_duration,
    bytes_written: compile_outcome.bytes_written,
    overwrote_existing: compile_outcome.overwrote_existing,
    kept_tex: keep_tex, kept_logs: keep_logs,
    intermediate_tex_path: (if keep_tex { Some(...) } else { None }) }`.

Erros propagados via `?` (todos existem nas specs 001-004):

- `read_template` → `TemplateNotFound` (20), `TemplatesDirMissing` (22),
  `PermissionDenied` (14), `Io` (1).
- `load_json_source` → `InvalidJson` (31), `PermissionDenied` (14),
  `Io` (1).
- `render_template` → `TeraRenderError` (30), `InvalidJson` (31).
- `atomic::write_atomic` (etapa 7) → `PermissionDenied` (14), `Io` (1).
- `compile_and_write` → `EngineNotInstalled` (41), `CompileFailed`
  (40), `UserAborted` (15), `PermissionDenied` (14), `Io` (1).

Externas ao pipeline (resolvidas no handler antes de chamar
`build_pipeline`):

- `resolve_engine` → `EngineNotSupported` (42).
- `Config::load` → `ConfigMissing` (10), `ConfigCorrupted` (11).

---

## Regras de validação por operação

### `resolve_output_pdf(cfg, template_name, output: Option<&Path>) -> PathBuf`

| Cenário             | Retorno                                                        |
|---------------------|----------------------------------------------------------------|
| `output = Some(p)`  | `paths::expand_user_path(p)`                                   |
| `output = None`     | `cfg.paths.output_dir.join(format!("{}.pdf", template_name))`  |

### `resolve_keep_flags(args, cfg) -> (bool, bool)`

| Cenário               | keep_tex                       | keep_logs                       |
|-----------------------|--------------------------------|---------------------------------|
| `--keep-tex`          | `true`                         | (via keep_logs)                 |
| `--no-keep-tex`       | `false`                        | (via keep_logs)                 |
| Nenhum flag           | `cfg.compiler.keep_tex`        | (via keep_logs)                 |
| `--keep-logs`         | (via keep_tex)                 | `true`                          |
| `--no-keep-logs`      | (via keep_tex)                 | `false`                         |
| Nenhum flag           | (via keep_tex)                 | `cfg.compiler.keep_logs`        |

### Regras do `intermediate_tex_path`

- `keep_tex=true` **antes** de compile → grava em
  `<output_dir>/<name>.tex` (mesmo se compile falhar → FR-015).
- `keep_tex=false` → `.tex` fica no TempDir e é apagado no drop.

---

## Não é entidade

- **Buffer `rendered: String`**: gerenciado pelo `build_pipeline`,
  não escapa para o caller.
- **`TempDir`**: RAII local do `build_pipeline` — dropado no fim
  do escopo.
- **Args exatos passados pro `compile_and_write`**: derivados de
  `BuildArgs` + defaults, sem struct intermediária pública.
