# Tasks: Auto PDF Pipeline

**Input**: Design documents from `/specs/007-auto-pdf-pipeline/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md).

**Tests**: Included — plan.md explicitly declares four integration test files as deliverables. Every user story gets its integration tests written before the implementation tasks for that story.

**Organization**: One phase per user story, ordered P1 → P2 → P2 → P3. Each phase ends at a Checkpoint where that user story is independently verifiable via the quickstart section that exercises it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different file, no dependency on incomplete task)
- **[Story]**: `[US1]` / `[US2]` / `[US3]` / `[US4]` on story tasks; omitted on Setup / Foundational / Polish
- Every task carries an exact file path

## Path Conventions

Single-crate Rust CLI. Source lives under `src/`, tests under `tests/` at the repository root — matches the layout declared in [plan.md § Source Code](./plan.md).

---

## Phase 1: Setup

**Purpose**: Land the empty module boundaries so subsequent tasks can `use` them without breaking `cargo build`.

- [X] T001 Declare new modules (`install`, `trust`, `discovery`) as `pub mod` entries in `src/lib.rs`
- [X] T002 [P] ~~Add empty error-enum stubs~~ **Adapted**: existing codebase uses a single `TexError` enum with grouped exit-code ranges (10-49). New variants will land inside `TexError` in T006, preserving convention. No stub file needed.

**Checkpoint**: `cargo build` passes with empty new modules; no user story work has started.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Deliver the shared value objects, path resolvers, and error variants every user story consumes. No user-visible behaviour yet.

**⚠️ CRITICAL**: US1 through US4 cannot begin until this phase is complete.

- [X] T003 Implement `Identifier` value type (parse, `Display`, `Eq`, regex validation `^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9._-]*$` for third-party and `^[a-z0-9][a-z0-9._-]*$` for bare built-in) in `src/discovery.rs` — manual char validation (no `regex` crate; constitution stack)
- [X] T004 [P] Implement `Version` value type (parse `major.minor.patch` with optional `-<pre>` and `+<build>`, `PartialOrd` on numeric components) in `src/templates.rs`
- [X] T005 [P] Implement `paths::templates_dir()` and `paths::trust_file()` (both via `dirs::data_local_dir().join("tex/…")`, no hardcoding) in `src/paths.rs`
- [X] T006 ~~Flesh out every variant of the six error enums~~ **Adapted**: added 23 new variants to the existing single `TexError` enum in `src/errors.rs`, grouped by concern (install 50-59, manifest 60-69, trust 70-79, resolve 80-89, sandbox 90-99). Preserves single-enum convention.
- [X] T007 Map every new error variant to a distinct exit code (**adapted**: ranges 50-99 instead of 0-6, per T002/T006 note) in `TexError::exit_code()` in `src/errors.rs`. `src/main.rs` unchanged — it already dispatches via `downcast_ref::<TexError>().map(|te| te.exit_code())`.

**Checkpoint**: Foundation ready; `cargo build` and `cargo test --all` still green (no new tests yet — foundational types are exercised via existing tests only).

---

## Phase 3: User Story 1 — First PDF from a built-in document type (Priority: P1) 🎯 MVP

**Goal**: `tex-cli build --json <path>` on a JSON with `document.type = "resume"` produces a PDF at a resolved output path with zero template authoring and zero install steps.

**Independent Test**: [quickstart.md § 1](./quickstart.md) — baseline compile of `/tmp/resume.json` using the built-in `resume` template. No trust prompt, no sandbox errors.

### Tests for User Story 1 (write first, ensure FAIL) ⚠️

- [X] T008 [P] [US1] Write integration test covering Case A of [contracts/json-document-fields.md](./contracts/json-document-fields.md) (built-in match by `document.type`) plus Case D (explicit override) and error paths (exit codes 80/81/82) in `tests/cli_pipeline_resolve.rs`. Uses `artigo`/`carta` as built-ins (per available fixtures — spec's `resume` example JSON needs a `resume.tex` template that's not yet shipped; deferred to a future polish task).

### Implementation for User Story 1

- [X] T009 [US1] Implement `templates::list_builtin_identifiers(dir)` in `src/templates.rs` — reuses existing `list_templates`, wraps each name in `Identifier::builtin`. Templates live in `cfg.paths.templates_dir` (the user's configured directory), which matches the constitution's "templates are static files" (II) and the existing spec-005 layout — no new directory needed for US1.
- [X] T010 [US1] Implement `discovery::resolve()` with `ResolveInputs { builtins, installed }`, `ResolvedTemplate { identifier, version }`, FR-018 precedence (explicit override → built-in → installed), NoMatch/Ambiguous variants with populated candidate lists — in `src/discovery.rs`. Includes 6 unit tests for the resolver flow.
- [X] T011 [US1] Extended `BuildOutcome` with `template_identifier: Identifier` and `template_version: Option<Version>` fields in `src/build.rs`; `build_pipeline` populates `template_identifier` from the passed `template_name` (built-in) so existing callers work unchanged.
- [X] T012 [US1] `build_pipeline_from_json()` in `src/build.rs` — loads JSON, enumerates built-ins, resolves, delegates to `build_pipeline`, overwrites the outcome's identifier/version with the resolver's typed values. No sandbox (no installed templates in US1).
- [X] T013 [US1] Added `--json <source>` to `BuildArgs` with `conflicts_with = "template_name"` in `src/cli.rs`.
- [X] T014 [US1] `handle_build` in `src/cli.rs` dispatches to `handle_build_from_json` when `--json` is set. `main.rs` unchanged — it already dispatches through `handle_build`.
- [X] T015 [US1] Interactive `handle_build_menu` in `src/cli.rs` now begins with a `Select` between "Compile a JSON to PDF (auto-resolve)" and "Pick a template and JSON explicitly", satisfying Constitution III (dual CLI + interactive on equal footing).

**Checkpoint**: US1 is fully functional and testable independently — the quickstart step 1 passes end-to-end. Ship-ready MVP for anyone using only built-in templates.

---

## Phase 4: User Story 2 — Install and use a third-party template (Priority: P2)

**Goal**: `tex-cli template install <git-url|path>` places a third-party package; the first compile against JSON that resolves to it triggers a one-time trust prompt; approved compiles run inside the FR-017 sandbox.

**Independent Test**: [quickstart.md § 2–5](./quickstart.md) — install `/tmp/acme-invoice`, hit the trust prompt, compile, re-run without prompt, upgrade to `1.1.0` and observe re-prompt, then verify `\write18` is blocked in step 5.

### Tests for User Story 2 (write first, ensure FAIL) ⚠️

- [X] T016 [P] [US2] `tests/cli_template_install.rs` — 10 tests covering local-path install, --force overwrite, missing manifest (exit 60), bare identifier rejection (exit 63), invalid version (exit 64), entrypoint escape (exit 65), non-.tex entrypoint (exit 67), install→list roundtrip, install→remove cleanup.
- [X] T017 [P] [US2] `tests/cli_template_trust.rs` — 4 tests: non-TTY build against un-trusted install exits 70; template remove drops trust records; version bump forces re-prompt (exit 70); `template trust --revoke --version` scoped revoke.
- [X] T018 [P] [US2] `tests/cli_pipeline_sandbox.rs` — 2 tests, both `requires_tectonic()`-gated: `\write18` in installed template is blocked (exit 40 or 92) AND the malicious side-effect file is absent; built-in compile still works without sandbox (regression check).

### Implementation for User Story 2

- [X] T019 [US2] `Manifest`, `Manifest::load()`, `require_str()` in `src/templates.rs`: TOML parse, required-field checks, `Identifier::from_str` + third-party enforcement, `Version::from_str`, absolute-path rejection, parent-dir escape rejection, `.tex` extension check, existence check.
- [X] T020 [US2] `install_from_local_path()` in `src/install.rs`: validate manifest → derive `<root>/<ns>/<name>/` → honor `--force` (or `InstallOverwrite` exit 53) → `copy_dir_recursive` (skips `.git/` and symlinks) → re-validate at destination. Plus `list_installed()`, `uninstall()` (best-effort prunes empty namespace dir).
- [X] T021 [US2] `install_from_git()` in `src/install.rs`: `which("git")` gate (else `GitBinaryMissing` exit 50) → `git clone --depth 1` into tempdir → delegate to local-path installer.
- [X] T022 [US2] `TrustFile` + `TrustRecord` + `TrustFile::load/save` in `src/trust.rs`. Save via `atomic::write_atomic` at mode 0o600. Missing file → default (empty). Corrupt TOML → `TrustFileCorrupted` (exit 71).
- [X] T023 [US2] `TrustFile::is_trusted / grant / revoke_all / revoke_version` in `src/trust.rs`. Grant is idempotent; `approved_at` stored as Unix seconds (no new crate — see commit note re: RFC 3339 deferral).
- [X] T024 [US2] `trust::ensure_trusted()` in `src/trust.rs`: skip if trusted; non-TTY → `TrustDenied` (exit 70); interactive → `inquire::Confirm` with FR-017 summary; refusal → `TrustDenied`; approval → grant + save.
- [X] T025 [US2] `discovery::resolve()` already accepted `InstalledCandidate` (T010); Phase 4 now populates it via `install::list_installed()` from `build_pipeline_from_json`. Ambiguity + NoMatch variants emit populated candidate lists.
- [X] T026 [US2] `SandboxDirective` struct + `for_builtin()` / `for_third_party()` constructors + `is_sandboxed()` in `src/compiler.rs`. (Adapted from the plan's `for_origin(&TemplateOrigin)` — the codebase has no `TemplateOrigin` enum; a boolean-shaped directive is cleaner.)
- [X] T027 [US2] Introduced `compile_and_write_sandboxed` (new signature with `SandboxDirective` + optional identifier-for-errors); `compile_and_write` now delegates to it with `SandboxDirective::for_builtin()`, preserving every pre-007 call site unchanged. `run_engine` composes `--only-cached` (tectonic), `openout_any=p` env var, and never opts into `-shell-escape` when the directive is sandboxed. Log-tail signature detection maps to `SandboxShellEscapeAttempted` / `SandboxBundleMissing` when applicable.
- [X] T028 [US2] `build_pipeline_from_json` in `src/build.rs` now enumerates installed via `install::list_installed`, resolves, and — for third-party matches — calls `trust::ensure_trusted` before dispatching to a new `build_pipeline_installed` helper that reads the manifest entrypoint, renders, and calls `compile_and_write_sandboxed` with `SandboxDirective::for_third_party()`.
- [X] T029 [US2] `TemplateCmd::Install { source, force }` + `handle_template_install` in `src/cli.rs`. Dispatches on `install::looks_like_git_url(&source)`.
- [X] T030 [US2] `TemplateCmd::List { json, include_builtin }` + `handle_template_list` — human table (IDENTIFIER/VERSION/TRUSTED/PATH) and structured `--json` output with a `"kind": "installed"|"builtin"` discriminator.
- [X] T031 [US2] `TemplateCmd::Remove { identifier, yes }` + `handle_template_remove` — namespace validation, `inquire::Confirm` unless `--yes`, `install::uninstall`, then `TrustFile::revoke_all`.
- [X] T032 [US2] `TemplateCmd::Trust { identifier, version, revoke }` + `handle_template_trust` — enumerates installed versions, per-version `Confirm` prompt for grant, immediate revoke without prompt.
- [X] T033 [US2] Extended `TemplateMenuAction` in `src/interactive.rs` with `InstallThirdParty / ListInstalled / RemoveInstalled / ManageTrust`; the interactive `template_menu()` now surfaces built-in and third-party actions side-by-side, wired to the T029-T032 handlers from `handle_templates_menu` in `src/cli.rs`. Two new prompt helpers: `prompt_template_install_source`, `prompt_template_identifier`.

**Checkpoint**: US2 is fully functional and testable independently — quickstart steps 2 through 6 pass end-to-end, including the ambiguity error in step 6.

---

## Phase 5: User Story 3 — Safe rendering of arbitrary user data (Priority: P2)

**Goal**: Every JSON value interpolated into LaTeX is escaped or transformed so no LaTeX special character can silently corrupt output or hard-fail the compile, regardless of whether the template is built-in or installed.

**Independent Test**: [spec.md § US3 Acceptance Scenarios](./spec.md) — JSON containing every LaTeX special (`& % $ # _ { } \ ~ ^`) round-trips into the PDF; unrepresentable unicode substitutes with a reported warning; template/engine syntax collisions are detected at template-load time.

### Tests for User Story 3 (write first, ensure FAIL) ⚠️

- [X] T034 [P] [US3] `tests/cli_pipeline_safe_render.rs` — 5 integration tests using `tex-cli render --dry-run` (no tectonic): all 10 LaTeX specials escaped in the rendered stdout; em-dash and en-dash substituted; substitution reported via `tracing::warn!` to stderr (visible with `-vv`); collision detected at line 3 with a targeted diagnostic; plain template renders verbatim (regression guard).

### Implementation for User Story 3

- [X] T035 [US3] Universal LaTeX escaping now runs on every JSON string value BEFORE tera renders. `escape_latex_string` covers all 10 specials (`\ & % $ # _ { } ~ ^`), applied recursively through arrays and objects. `render_template` API unchanged; a new `render_template_safe` returns `(String, Vec<CharSubstitution>)` for callers that need the warnings list. 6 new unit tests in `src/render.rs`.
- [X] T036 [US3] Unicode substitution path: em-dash `—` → `---` and en-dash `–` → `--`, each recorded as a `CharSubstitution { original, replacement, json_path }` and logged via `tracing::warn!` on stderr with the JSON dot-path where it happened. Warnings do not yet propagate into `BuildOutcome` — deferred as a future ergonomic (they're already observable via `-vv` logs).
- [X] T037 [US3] `templates::check_template_collisions(name, src)` scans for `\<macro>{#` (Tera comment opener adjacent to a LaTeX macro argument — the canonical `\MakeUppercase{#1}` bug from spec 007 Context). Reports `TeraRenderError` with the macro name and line/col of the collision. Wired into `render_template_safe` before Tera parsing so users see the targeted diagnostic instead of an opaque parse error. 4 new unit tests.

**Checkpoint**: US3 is independently verifiable; the safe-render test passes for both built-in and installed templates without regressions in US1/US2.

---

## Phase 6: User Story 4 — Zero-config first run (Priority: P3)

**Goal**: `tex-cli build --json <path>` on a machine with no `~/.config/tex/config.toml` produces the PDF, creates a default config, and reports where it was written — no separate `init` invocation required.

**Independent Test**: [spec.md § US4 Acceptance Scenario 1](./spec.md) — nuke `~/.config/tex/` and `~/.local/share/tex/`, run the pipeline against a valid built-in-matching JSON, verify the PDF is produced and the config file is created at the documented location.

### Tests for User Story 4 (write first, ensure FAIL) ⚠️

- [X] T038 [P] [US4] `tests/cli_zero_config.rs` — 3 integration tests: (1) `build --json` in a bare home creates the config at the canonical XDG path and writes the "no config found" note to stderr; (2) second invocation does NOT re-announce (bootstrap only fires once); (3) explicit `build <template> <json>` still errors with `ConfigMissing` (exit 10) — US4 scope is intentionally limited to the `--json` path.
- [X] T039 [US4] `Config::default_bootstrap()` in `src/config.rs` (templates_dir=`~/.config/tex/templates`, output_dir=cwd, engine=`tectonic`). `handle_build` catches `TexError::ConfigMissing` on the `--json` route and calls `bootstrap_default_config` in `src/cli.rs` (creates parent dir, best-effort creates templates_dir, atomic save, prints the path + defaults to stderr as a "note"). PDF pipeline continues unchanged.
- [X] T040 [US4] `warm_tectonic_bundle_cache()` added to `src/cli.rs`; invoked at the end of `handle_init` when the chosen engine is tectonic. Uses an embedded (no external file) 4-line minimal LaTeX fixture, runs tectonic in a `tempfile::TempDir`, silences stdout/stderr, prints a one-line progress note. Never fails init — missing tectonic prints an informational note referring the user to `SandboxBundleMissing` for the follow-up. Adapted from tasks.md's `examples/templates/carta` recommendation: an embedded fixture avoids depending on the examples/ layout being present at runtime, and the file is smaller than any of the three A-04 templates.

**Checkpoint**: US4 is independently verifiable; a fresh-machine scenario produces a PDF with a single command.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, changelog, roadmap updates, and full quickstart validation. No user-facing behaviour change beyond copy.

- [X] T041 [P] `README.md` — new "JSON → PDF auto-pipeline (`build --json`) — spec 007" section, "Installable third-party templates (`template …`) — spec 007" section with security-model summary, links to spec 007 quickstart / manifest schema / JSON contract. Extended the exit-code table with all new codes 50-92.
- [X] T042 [P] `CHANGELOG.md` — added spec 007 block under `[Unreleased]` with `### Added — spec 007` (all new subcommands, trust model, sandbox, zero-config, bundle warm-up, value types) and `### Changed — spec 007 (BREAKING)` (universal LaTeX escaping migration note, load-time collision detection, BuildOutcome shape change).
- [X] T043 [P] `ROADMAP.md` — appended a 2026-08-03 Progress-log entry summarising the spec 007 delivery, with the Q1=C "library-only" note and a link back to `specs/007-auto-pdf-pipeline/`.
- [ ] T044 Docker quickstart validation — image build kicked off in the background at commit time; user to run the full 7-step walkthrough in the container and file follow-ups for any deviation. Instructions live in `specs/007-auto-pdf-pipeline/quickstart.md`.
- [X] T045 `cargo fmt --all` applied (13 files reformatted); `cargo clippy --all-targets -- -D warnings` returns **No issues found**.
- [X] T046 [P] `tests/cli_pipeline_deterministic.rs` — 2 integration tests, both `requires_tectonic()`-gated: repeat built-in compile byte-identical; repeat installed-template-at-fixed-version compile byte-identical. Closes the FR-009 / SC-007 coverage gap raised in the `/speckit-analyze` report.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: no dependencies — start immediately
- **Phase 2 Foundational**: depends on Phase 1 completion — BLOCKS all user stories
- **Phase 3 US1 (P1 MVP)**: depends on Phase 2
- **Phase 4 US2 (P2)**: depends on Phase 2 and Phase 3 (US2's build pipeline reuses the router landed in US1 tasks T012–T014)
- **Phase 5 US3 (P2)**: depends on Phase 2. Independent of US1/US2 for tests; runs on top of the render pipeline that US1 already exercises
- **Phase 6 US4 (P3)**: depends on Phase 3 (build pipeline entry point must exist to auto-create config before it)
- **Phase 7 Polish**: depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)** — no dependency on other stories
- **US2 (P2)** — layered on top of US1's build pipeline entry point; independently testable via the install → trust → sandbox flow
- **US3 (P2)** — parallel-safe with US2 (touches `render.rs` + `templates.rs` syntax-collision detection, not `install.rs` / `trust.rs` / `compiler.rs`); reuses US1's build pipeline
- **US4 (P3)** — parallel-safe with US2/US3; small delta in `build.rs` entry + `config.rs`

### Within Each User Story

- Test task(s) MUST be written first and MUST fail before the implementation tasks of that story land
- Data-model changes (T011 in US1) before pipeline changes (T012)
- CLI/dispatcher last within each story so the story's Checkpoint is a full end-to-end smoke test

### Parallel Opportunities

- **Phase 1**: T002 in parallel with T001
- **Phase 2**: T004 and T005 in parallel with T003; T006 must follow T002 (fills the stubs); T007 follows T006
- **Phase 3 (US1)**: T008 in parallel with T009 while implementation lands
- **Phase 4 (US2)**: T016, T017, T018 all in parallel; T019 and T022 can be in parallel (different files); install and trust modules parallel-safe until T028 wires them together
- **Phase 5 (US3)**: T034 in parallel with T035/T036/T037
- **Phase 7 Polish**: T041/T042/T043/T046 all in parallel

---

## Parallel Example: User Story 2

```bash
# Write all three failing integration tests first, in parallel:
Task: T016 install test  → tests/cli_template_install.rs
Task: T017 trust test    → tests/cli_template_trust.rs
Task: T018 sandbox test  → tests/cli_pipeline_sandbox.rs

# Implement leaf modules in parallel once tests are red:
Task: T019 Manifest loader → src/templates.rs
Task: T022 TrustFile I/O   → src/trust.rs
Task: T026 SandboxDirective → src/compiler.rs
```

---

## Implementation Strategy

### MVP first (US1 only)

1. Complete Phase 1 (Setup) — T001, T002
2. Complete Phase 2 (Foundational) — T003 through T007
3. Complete Phase 3 (US1) — T008 through T015
4. **STOP and VALIDATE**: quickstart § 1 passes; a resume JSON produces a PDF via `tex-cli build --json`
5. Deploy / demo — the MVP already replaces the manual pipeline for built-in document types

### Incremental delivery

1. Ship MVP (US1) — the "JSON in → built-in PDF out" promise
2. Add US2 — unlocks the SDK / community-template story; the biggest single value jump
3. Add US3 — hardens the render safety net for every template
4. Add US4 — closes the last first-run friction gate
5. Polish (Phase 7) — docs + changelog + quickstart-in-Docker run

### Parallel team strategy

Once Phase 2 lands, US1/US3/US4 can be worked in parallel by three implementers (US1's pipeline entry-point commit needs to land first for US4 to build on, but that is a single early task). US2's install + trust + sandbox modules are parallel-safe among themselves per the T019/T022/T026 grouping above.

---

## Notes

- Every task references a concrete file path from [plan.md § Source Code](./plan.md).
- Every user story ends at a Checkpoint that maps to a section of [quickstart.md](./quickstart.md), so an implementer can validate the story without reading the full spec.
- Constitution III interactive-menu tasks (T015, T033) are first-class implementation tasks, not polish — the constitution treats the two interfaces as equal-priority.
- Commit after each task or logical group; do not amend prior commits during implementation (Git safety per project convention).
