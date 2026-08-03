# Implementation Plan: Auto PDF Pipeline

**Branch**: `007-auto-pdf-pipeline` | **Date**: 2026-08-03 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-auto-pdf-pipeline/spec.md`

## Summary

Deliver the "JSON in → PDF out" one-command pipeline on top of the render + compile subsystems from specs 003/004/005. Scope is fixed by the five clarifications recorded in [spec.md `## Clarifications` § 2026-08-03](./spec.md#session-2026-08-03):

1. Third-party templates are distributed via Git URL or local filesystem path (no registry) — [FR-015](./spec.md).
2. Trust is granted per template + version and re-prompted on version upgrade — [FR-016](./spec.md).
3. Third-party templates compile inside a hard sandbox (`\write18` off, no network, no writes outside the output dir); built-in templates are exempt — [FR-017](./spec.md).
4. Built-in templates use bare names; third-party templates require `namespace/name` identifiers; `document.template` on the JSON overrides `document.type`; built-in wins ties — [FR-018](./spec.md).
5. Third-party template packages must ship a manifest with `identifier`, `version`, `entrypoint` — [FR-019](./spec.md).

Auto-generation of templates is explicitly out of v1 (per Clarification Q1 = library-only). Unknown JSON produces a structured error listing near-match candidates and install instructions.

Concrete technical approach:

- New CLI subcommand tree `tex-cli template {install|list|remove|trust}` and a new pipeline entrypoint (`tex-cli build --json <path>` reuses the spec 005 build pipeline behind a new resolution layer).
- Template resolution lives in a new `discovery.rs` module that walks: `document.template` override → built-in library → installed templates.
- Install lives in a new `install.rs` that shells out to `git` (via `which` + `std::process::Command`) for URLs, or does a validated file copy for local paths.
- Trust records live in a new `trust.rs` backed by `~/.local/share/tex/trust.toml` (per [research.md](./research.md) — decision R3).
- Sandbox lives as new flags composed inside `compiler.rs`; tectonic sandbox capabilities are the linchpin research question — see [research.md](./research.md) R1.
- Manifest parsing extends `templates.rs`; format is `tex-template.toml` (per [research.md](./research.md) R4).

## Technical Context

**Language/Version**: Rust stable, edition 2021 (unchanged).

**Primary Dependencies**: `clap`, `inquire`, `serde` + `serde_json`, `tera`, `toml`, `dirs`, `tempfile`, `anyhow`, `thiserror`, `tracing`, `which`. **No new dependencies proposed** — Git cloning is handled by shelling out to the `git` binary via `which` (justification in [research.md](./research.md) R2), preserving the constitution's fixed stack.

**Storage**: Filesystem only.
- Installed templates: `~/.local/share/tex/templates/<namespace>/<name>/` (resolved via `dirs::data_local_dir()`).
- Trust records: `~/.local/share/tex/trust.toml` (single file, one entry per approved `identifier@version`).
- Config path unchanged: `~/.config/tex/config.toml`.

**Testing**: `cargo test --all` in Docker container per project convention. New integration tests under `tests/`:
- `cli_template_install.rs` — install from local path and from Git URL fixture.
- `cli_template_trust.rs` — trust prompt on first use, re-prompt on version bump, denial path.
- `cli_pipeline_sandbox.rs` — third-party template attempting `\write18` fails with the FR-017 message; built-in template unaffected.
- `cli_pipeline_resolve.rs` — resolution order (explicit override > built-in > installed) and ambiguous-installed error.

**Target Platform**: Linux (primary, per constitution). macOS/Windows should compile but sandbox-level guarantees are Linux-first in v1 (see [research.md](./research.md) R1 for portability caveats).

**Project Type**: CLI tool (single binary, single Rust crate — matches existing layout).

**Performance Goals**: SC-001 mandates < 60 s first-run time-to-PDF for a JSON matching a built-in template, including any first-run environment setup. No throughput target — this is an interactive single-invocation tool.

**Constraints**:
- Constitution II (data/template/PDF separation) — installed templates are static files at rest; the tool NEVER rewrites them at runtime.
- Constitution V (safe file handling) — install must not overwrite an existing installed template of the same `identifier@version` without an explicit `--force` flag; trust prompt uses `inquire`.
- FR-009 (byte-identical repeat runs) — install must be idempotent; sandbox flags must be deterministic.

**Scale/Scope**: Single-user local tool. Expected corpus: ≤ 100 installed templates, ≤ 1000 trust records. Nothing warrants a database.

## Constitution Check

*GATE evaluated against `.specify/memory/constitution.md` v1.0.0. Re-check after Phase 1.*

| Principle | Verdict | Justification |
|-----------|---------|---------------|
| I — Escopo pessoal, camada de automação | ✅ PASS | Feature strictly automates the JSON → template → PDF path; installs templates authored by users, does not reimplement any LaTeX internal. |
| II — Separação estrita JSON / .tex / PDF | ✅ PASS | Installed templates are `.tex` source files stored statically on disk (never rewritten by the tool). Q1=C forbids runtime template generation, aligning perfectly with this principle. |
| III — Interface dupla CLI + interactive | ✅ PASS (with follow-through) | All new subcommands (`template install/list/remove/trust`, `build --json`) exposed as direct CLI. Interactive menu MUST gain matching entries — tracked as an explicit task in Phase 2. Trust prompt itself is inquire-based, satisfying the interactive contract at the approval step. |
| IV — Compilador isolado, engine plugável | ✅ PASS | Sandbox flags (FR-017) composed inside `compiler.rs` behind the existing engine trait/enum; no engine-specific logic leaks to callers. Tectonic is the v1 sandbox target; alternate engines can implement or refuse sandbox mode without changing callers. |
| V — Segurança na manipulação de arquivos | ✅ PASS | Install requires `--force` to overwrite an existing `identifier@version`. Trust prompt gates all third-party compiles. Manifest is parsed and validated before any file movement. Sandbox violations produce human-readable errors, never raw tectonic logs. |
| Stack fixa | ✅ PASS | No new crate proposed. Git shell-out via `which` + `Command` (already-canonical pattern). |

**No violations to track in Complexity Tracking.**

## Project Structure

### Documentation (this feature)

```text
specs/007-auto-pdf-pipeline/
├── plan.md              # This file (/speckit-plan output)
├── research.md          # Phase 0 output — resolves NEEDS CLARIFICATION
├── data-model.md        # Phase 1 output — entities + on-disk layout
├── quickstart.md        # Phase 1 output — end-to-end walkthrough
├── contracts/           # Phase 1 output — CLI + manifest schemas
│   ├── cli-template-subcommands.md
│   ├── cli-build-pipeline.md
│   ├── manifest-schema.md
│   └── json-document-fields.md
├── spec.md              # Feature specification (already exists)
└── tasks.md             # Phase 2 output (/speckit-tasks — not created here)
```

### Source Code (repository root)

```text
src/
├── main.rs              # unchanged entrypoint
├── lib.rs               # add new module declarations
├── cli.rs               # extend with `template` subcommand tree and `build --json`
├── interactive.rs       # extend menu with template install / trust flows
├── config.rs            # unchanged
├── paths.rs             # add resolvers for templates_dir(), trust_file()
├── errors.rs            # add InstallError, TrustError, ManifestError, SandboxError variants
├── templates.rs         # extend: parse manifest, list built-in + installed
├── render.rs            # unchanged (spec 003)
├── compiler.rs          # extend: sandbox flag composition for third-party templates
├── build.rs             # extend: pipeline resolves via discovery.rs, threads sandbox flag
├── atomic.rs            # unchanged (spec 005)
├── install.rs           # NEW — git clone / path copy + manifest validation
├── trust.rs             # NEW — trust record load/save + inquire prompt
└── discovery.rs         # NEW — resolve JSON → template identifier (Q4 policy)

tests/
├── cli_template_install.rs    # NEW
├── cli_template_trust.rs      # NEW
├── cli_pipeline_sandbox.rs    # NEW
├── cli_pipeline_resolve.rs    # NEW
└── (existing tests untouched)

examples/templates/
└── (existing built-in library — unchanged in v1: resume, artigo-basico, carta)
```

**Structure Decision**: Single-crate Rust CLI (Option 1 from the template — this project's baseline). No sub-crate split needed; the three new modules are cohesive with the existing set and share error types via `errors.rs`.

## Complexity Tracking

> No constitution violations. Table intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| _(none)_  | _(n/a)_    | _(n/a)_                             |

## Phase Roadmap

- **Phase 0 — Research** (this command): resolve five R-items — [research.md](./research.md).
- **Phase 1 — Design & Contracts** (this command): [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md).
- **Phase 2 — Task generation**: `/speckit-tasks` splits the design into a dependency-ordered TDD-style task list. Not produced here.
- **Phase 3 — Implementation**: `/speckit-implement`.
