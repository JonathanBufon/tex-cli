# Implementation Plan: Translate All User-Facing Strings to English

**Branch**: `006-translate-to-english` | **Date**: 2026-07-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-translate-to-english/spec.md`

## Summary

Replace every Portuguese user-visible string in `src/` (error messages,
`--help` doc comments, interactive prompts, stdout success lines, the
`--format` default value) with its English equivalent. No i18n
framework, no locale detection, no macros — plain English string
literals in place. Preserve every exit code, every structural output
shape (path position, duration position, log-tail format). Update all
tests that assert on Portuguese tokens. Update `README.md` sections
that quote actual command output. Document as a breaking change in
`CHANGELOG.md`.

This is a **text-only refactor**. Zero behavior change beyond the
`humano → human` default flip on `--format` (see FR-015 decision in
[research.md](./research.md)).

Reference: closes the last i18n blocker on the road to `v1.0.0` per
[`ROADMAP.md`](../../ROADMAP.md).

## Technical Context

**Language/Version**: Rust stable, edition 2021 (unchanged).

**Primary Dependencies**: `clap`, `inquire`, `thiserror`, `tera`,
`tectonic`, `serde`, `serde_json`, `toml` (all unchanged). **No new
dependencies.**

**Storage**: N/A — config file at `~/.config/tex/config.toml`
unchanged, schema unchanged.

**Testing**: `cargo test --all` in Docker container per project
convention. Baseline ~197 tests. Delta expected: 0 net tests added,
but ~25 test assertions rewritten to English tokens.

**Target Platform**: Linux (canonical), macOS / WSL2 (supported).
Unchanged.

**Project Type**: Rust CLI binary with library facade for integration
tests. Unchanged.

**Performance Goals**: No performance impact. String literal size
delta is negligible.

**Constraints**: Preserve every existing exit code
(`TexError::variant.exit_code()` returns identical `i32`). Preserve
structural output shape (path in same word position, duration in
same word position, log tail delimited identically).

**Scale/Scope**: 14 `TexError` variants, ~50 clap doc comments,
~20 interactive prompt strings, 3 stdout success messages, 1
`--format` default value, ~25 test assertions. Estimated ~300 line
changes across 8 files.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Text-only translation. Every principle is preserved.

| Principle                                              | Status | Notes                                                                                    |
|--------------------------------------------------------|--------|------------------------------------------------------------------------------------------|
| I. Escopo pessoal e não-substituição do LaTeX          | ✅ PASS | No scope change. Feature keeps `tex-cli` as a JSON→PDF automation layer.                 |
| II. Separação Dados/Template/PDF                       | ✅ PASS | Layers untouched. No new code paths cross boundaries.                                    |
| III. Interface Dupla CLI + Interativo                  | ✅ PASS | Both CLI and interactive surfaces are translated identically. Contract preserved.        |
| IV. Compilador Plugável                                | ✅ PASS | `compiler.rs` engine dispatch unchanged. Only string literals inside error messages change. |
| V. Segurança na Manipulação de Arquivos                | ✅ PASS | Atomic writes, overwrite consent, `.tex` preservation on failure — all untouched.        |

**Stack lock check**: Zero new dependencies. All translations use
Rust string literals only.

**Result**: **PASS**. No Complexity Tracking entries required.

## Project Structure

### Documentation (this feature)

```text
specs/006-translate-to-english/
├── plan.md              # This file (/speckit-plan output)
├── research.md          # Phase 0 output — FR-015 decision, style guide, test-refactor strategy
├── data-model.md        # Phase 1 output — N/A (no entities); documents that fact
├── quickstart.md        # Phase 1 output — dev flow + migration guide for callers
├── contracts/
│   └── cli.md           # Phase 1 output — new English strings per subcommand
├── checklists/
│   └── requirements.md  # Quality checklist (already present from /speckit-specify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── build.rs        # 1 stdout success message → English
├── cli.rs          # ~50 /// doc comments + `--format` default → English
├── compiler.rs     # 1 stderr message → English
├── errors.rs       # 14 TexError variants #[error("...")] → English + test block
├── interactive.rs  # ~20 inquire prompts → English
├── render.rs       # (may have 1 stdout message) → English
└── (other files)   # Untouched. Grep-verified during audit.

tests/
├── cli_banner.rs           # No PT assertions found — likely untouched
├── cli_build.rs            # Several `assert!(msg.contains("PT"))` → English
├── cli_compile.rs          # Several `assert!(msg.contains("PT"))` → English
├── cli_config_set.rs       # Config-error assertions → English
├── cli_config_show.rs      # Format flag test → --format human
├── cli_init.rs             # Prompt regex assertions → English
├── cli_render.rs           # Render error assertions → English
├── cli_templates_add.rs    # Add error assertions → English
├── cli_templates_list.rs   # Format flag test + list-header → English
├── cli_templates_remove.rs # Remove error/prompt assertions → English
└── cli_templates_show.rs   # Show error assertions → English
```

**Structure Decision**: **Per-file translation**, not per-user-story.
Justification in [research.md § D-05](./research.md). One PR at the
end contains the full sweep. Task granularity in `tasks.md` will
group by file to make review tractable.

## Complexity Tracking

*(No Constitution violations. Table left empty per template guidance.)*
