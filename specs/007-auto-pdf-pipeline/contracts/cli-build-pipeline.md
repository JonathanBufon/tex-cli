# CLI contract — `tex-cli build --json <source>`

**Feature**: 007-auto-pdf-pipeline
**Constitution alignment**: III (scriptable CLI + interactive), V (safe file ops), IV (compiler isolation).

The `build` subcommand from spec 005 gains a new mode: accept a JSON source directly and run the full JSON → template resolution → render → compile → PDF pipeline in one invocation (FR-001).

## Invocation

```text
tex-cli build --json <source> [--output <path>] [--engine <name>] [--force] [--yes]
```

- **`--json <source>`** (required in JSON mode): file path or `-` for stdin.
- **`--output <path>`** (optional): override the resolved output path.
- **`--engine <name>`** (optional): engine override (existing spec 005 flag).
- **`--force`** (optional): overwrite an existing PDF at the output path.
- **`--yes`** (optional): skip the trust prompt for third-party templates (assumes trust is already granted; if not, `--yes` alone MUST NOT auto-approve — approval requires an interactive confirmation OR an explicit prior `tex-cli template trust` call).

Existing `tex-cli build` behaviour (rendering a pre-selected template + JSON) is unchanged; adding `--json` selects the pipeline mode where discovery runs.

## Behaviour

1. **Load JSON** — `serde_json::from_reader`; on parse error → exit **2** (`InputError::JsonParse`).
2. **Resolve template** — `discovery::resolve(&json)`:
   - Explicit `document.template` → forced choice.
   - Else `document.type` matched against built-in first, then installed (per FR-018).
   - On `NoMatch` → exit **2** with the FR-004 error listing near-match candidates and an install hint.
   - On `Ambiguous` → exit **2** listing every candidate identifier.
3. **Trust check** (installed templates only) — `trust::is_trusted(id, ver)`:
   - Trusted → proceed silently.
   - Not trusted:
     - If stdin is a TTY → `inquire::Confirm` "Trust `<id>@<ver>`? This template will run inside the sandbox described in FR-017.".
     - If stdin is not a TTY (script context) → exit **6** (`TrustError::DeniedByUser`, new exit code specifically for this case) with instruction to run `tex-cli template trust <id>` first.
     - On approval → `trust::grant(id, ver)`.
     - On denial → exit **6**.
4. **Render** — existing render pipeline (`render.rs`), unchanged.
5. **Compile** — `compiler::compile(engine, tex_path, output_dir, SandboxDirective::for_origin(&descriptor.origin))`.
6. **Emit report** — see FR-010 shape; print to stdout as human-readable text by default, or as JSON if `--format=json` (existing spec 005 flag).

## Exit codes (extends spec 005)

| Code | Meaning                                                                 |
|------|-------------------------------------------------------------------------|
| 0    | Success                                                                 |
| 2    | Input error: bad JSON, no template resolution, ambiguous resolution     |
| 3    | Environment error: engine missing, bundle cache missing, git missing    |
| 4    | Render error                                                            |
| 5    | Compile error (including sandbox violations `SandboxError::*`)          |
| 6    | **NEW** — Trust denied or trust cannot be prompted in a non-TTY context |

## Interactive counterpart

Constitution III mandates that every CLI subcommand be reachable from `tex-cli interactive`. The pipeline is added as a menu entry "Compile a JSON to PDF" which prompts for the JSON path via `inquire::Text` and then executes the same logic. Trust prompts appear inline as they would in the scripted flow.

## Output determinism (FR-009)

Given the same JSON, same installed template at the same version, and the same trust state, `tex-cli build --json` MUST produce byte-identical PDFs. Sandbox flags are deterministic; the discovery result is deterministic given a stable installed-templates directory; the trust check has no side effects on the compile output. This holds only when the compilation engine (tectonic) is at the same version — engine version bumps are documented as the source of variance in the Report.
