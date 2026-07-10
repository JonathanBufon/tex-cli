# Phase 1 — Data Model: Renderização JSON → `.tex` via Tera

**Feature**: `003-render-tera`
**Date**: 2026-07-09

Modelo de dados desta feature — todas as entidades são efêmeras
(não persistidas). Reaproveita `Config`, `Template`,
`resolve_template`/`read_template` da spec 002 sem modificações.

Cross-reference: extende `TexError` conforme D-06 do research.

---

## Entidade: `RenderArgs`

Argumentos que o handler `handle_render` consome. Vem de duas
fontes: clap (subcomando direto) ou modo interativo (`render_menu`).

```rust
#[derive(Debug, Clone)]
pub struct RenderArgs {
    /// Nome canônico do template — resolvido via resolve_template.
    pub template_name: String,

    /// Path do arquivo JSON no host OU o literal "-" (stdin).
    pub data_source: String,

    /// Path custom do .tex final. None → default
    /// (config.paths.output_dir/<template_name>.tex).
    pub output: Option<PathBuf>,

    /// true → imprime em stdout, não grava; ignora `output`.
    pub dry_run: bool,

    /// true → sobrescreve output existente sem confirmação.
    pub force: bool,
}
```

Regras de construção:

| Campo         | Vem de                          | Regra                          |
|---------------|---------------------------------|--------------------------------|
| template_name | positional arg OU Select do menu | não vazio, valida via resolve_template |
| data_source   | positional arg OU Text do menu   | não vazio, "-" ou path         |
| output        | `-o/--output` OU None            | expandido via expand_user_path se presente |
| dry_run       | `--dry-run` OU Confirm do menu   | bool                           |
| force         | `--force` OU sempre `false` no menu | bool                           |

---

## Entidade: `RenderOutcome`

Retornado por `render_and_write` para o handler compor a mensagem
final de stdout. Efêmero.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutcome {
    /// Path final do arquivo gravado. None se dry_run.
    pub output_path: Option<PathBuf>,

    /// Bytes escritos (no arquivo OU no stdout).
    pub bytes_written: u64,

    /// true se sobrescreveu arquivo pré-existente.
    /// Sempre false quando dry_run == true.
    pub overwrote_existing: bool,

    /// Espelha RenderArgs.dry_run para o handler decidir a mensagem.
    pub dry_run: bool,
}
```

---

## Entidade auxiliar: `RenderRequest` (interno de `render.rs`)

Encapsulamento do que `render_template` recebe. Não é público —
existe só para deixar a assinatura da função pura sem 5 args soltos.

```rust
struct RenderRequest<'a> {
    template_name: &'a str,
    template_src: &'a str,
    context: &'a tera::Context,
}
```

Usado por:

```rust
pub fn render_template(
    template_name: &str,
    template_src: &str,
    json_value: &serde_json::Value,
) -> Result<String, TexError>
```

Que faz:

1. Valida `json_value.is_object()` — senão `InvalidJson { source:
   "<template context>", detail: "esperado objeto no topo, recebido
   <tipo>" }`.
2. `tera::Context::from_value(json_value.clone())` → contexto.
3. `tera::Tera::one_off(template_src, &context, false)` →
   `Ok(string)` ou `Err(tera::Error)`.
4. Mapeia erro do tera para `TeraRenderError { template_name,
   detail: tera_error.to_string() }`.

---

## Extensão de `TexError`

Duas novas variantes conforme D-06 do research:

```rust
#[error("Falha ao renderizar template '{template_name}': {detail}")]
TeraRenderError {
    template_name: String,
    detail: String,
},

#[error("JSON inválido em {source}: {detail}")]
InvalidJson {
    source: String,   // "stdin" ou path como string
    detail: String,   // mensagem localizada (line/col se disponível)
},
```

`TexError::exit_code()` estendido:

- `TeraRenderError { .. }` → **30**
- `InvalidJson { .. }` → **31**

Todas as demais entradas mantidas (10..15, 20..22, 1).

---

## Regras de validação por operação

### `load_json_source(source: &str) -> Result<serde_json::Value, TexError>`

| Cenário                                    | Comportamento                                             |
|--------------------------------------------|-----------------------------------------------------------|
| `source == "-"`                            | Lê stdin em `String`; falha de I/O → `TexError::Io` (exit 1) |
| `source == "-"` e stdin vazio              | `InvalidJson { source: "stdin", detail: "sem conteúdo" }` → exit 31 |
| `source` path existente e legível          | `fs::read_to_string`                                       |
| `source` path inexistente                  | `TexError::Io(NotFound)` → exit 1                          |
| `source` path sem permissão                | `TexError::PermissionDenied { path }` → exit 14            |
| Conteúdo não é JSON válido                 | `InvalidJson { source, detail: erro.to_string() }` → exit 31 |
| JSON parseado mas top-level não é objeto   | `InvalidJson { source, detail: "esperado objeto..." }` → exit 31 |

### `render_template(template_name, template_src, json_value) -> Result<String, TexError>`

| Cenário                            | Comportamento                                          |
|-------------------------------------|--------------------------------------------------------|
| json_value não é objeto             | `InvalidJson` → exit 31 (defensivo, load já valida)   |
| tera::Error (var indefinida, sintaxe, filtro) | `TeraRenderError { template_name, detail }` → exit 30 |
| render OK                           | `Ok(string)`                                           |

### `resolve_output_path(cfg: &Config, args: &RenderArgs) -> Result<PathBuf, TexError>`

| Cenário                      | Comportamento                                                        |
|------------------------------|----------------------------------------------------------------------|
| `args.output = Some(path)`   | `paths::expand_user_path(path)`                                      |
| `args.output = None`         | `cfg.paths.output_dir.join(format!("{}.tex", args.template_name))`   |
| `args.dry_run = true`        | Path irrelevante — não é usado                                       |

### `render_and_write(args, cfg) -> Result<RenderOutcome, TexError>`

Fluxo:

1. `template_src = read_template(&cfg.paths.templates_dir,
   &args.template_name)?` — reuso spec 002, exit 20 se ausente.
2. `json = load_json_source(&args.data_source)?`.
3. `rendered = render_template(&args.template_name,
   &template_src_str, &json)?`.
4. Se `args.dry_run`:
   - `io::stdout().write_all(rendered.as_bytes())?`.
   - `Ok(RenderOutcome { output_path: None, bytes_written:
     rendered.len(), overwrote_existing: false, dry_run: true })`.
5. Senão:
   - `output_path = resolve_output_path(cfg, args)?`.
   - `overwrote_existing = output_path.exists()`.
   - Se overwrite + `!args.force` + TTY: prompt confirm; "não"
     → `UserAborted` (exit 15).
   - Se overwrite + `!args.force` + não-TTY: `UserAborted`.
   - `atomic::write_atomic(&output_path, rendered.as_bytes(),
     0o644)?`.
   - `Ok(RenderOutcome { output_path: Some(output_path),
     bytes_written, overwrote_existing, dry_run: false })`.

---

## Extração de `atomic::write_atomic` (D-03/D-10 do research)

Novo módulo `src/atomic.rs`:

```rust
pub fn write_atomic(
    target: &Path,
    bytes: &[u8],
    mode: u32,
) -> Result<(), TexError>
```

Move a lógica hoje em `src/templates.rs::write_atomic_0644` para
esse módulo, agora parametrizando `mode`. Call sites:

- `src/templates.rs::add_template`: `atomic::write_atomic(&dest_path,
  &bytes, 0o644)`.
- `src/render.rs::render_and_write`: idem, com o `.tex` renderizado.

`src/config.rs::save_atomic` **não** é migrado nesta spec — mantém
implementação inline com mode 0o600 e serialização toml no meio da
função.

---

## Não é entidade

- **`tera::Context`**: construído dentro de `render_template` a partir
  do `serde_json::Value`. Não vaza para fora.
- **`inquire` responses no menu**: strings/paths consumidos
  imediatamente na construção do `RenderArgs`.
- **Stdout bytes no dry-run**: streamed direto de `render_template`
  → `io::stdout()`. Sem struct intermediária.
