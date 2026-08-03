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

- [ ] T001 Declare new modules (`install`, `trust`, `discovery`) as `pub mod` entries in `src/lib.rs`
- [ ] T002 [P] Add empty error-enum stubs (`InstallError`, `TrustError`, `ManifestError`, `ResolveError`, `SandboxError`, `DiscoveryError`) marked `#[derive(Debug, thiserror::Error)]` in `src/errors.rs`

**Checkpoint**: `cargo build` passes with empty new modules; no user story work has started.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Deliver the shared value objects, path resolvers, and error variants every user story consumes. No user-visible behaviour yet.

**⚠️ CRITICAL**: US1 through US4 cannot begin until this phase is complete.

- [ ] T003 Implement `Identifier` value type (parse, `Display`, `Eq`, regex validation `^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9._-]*$` for third-party and `^[a-z0-9][a-z0-9._-]*$` for bare built-in) in `src/discovery.rs`
- [ ] T004 [P] Implement `Version` value type (parse `major.minor.patch` with optional `-<pre>` and `+<build>`, `PartialOrd` on numeric components) in `src/templates.rs`
- [ ] T005 [P] Implement `paths::templates_dir()` and `paths::trust_file()` (both via `dirs::data_local_dir().join("tex/…")`, no hardcoding) in `src/paths.rs`
- [ ] T006 Flesh out every variant of the six error enums to match [data-model.md § Errors](./data-model.md) in `src/errors.rs`
- [ ] T007 Map every new error variant to a distinct exit code (0/2/3/4/5/6 per [contracts/cli-build-pipeline.md](./contracts/cli-build-pipeline.md)) in the existing exit-code function in `src/errors.rs` and wire it in `src/main.rs`

**Checkpoint**: Foundation ready; `cargo build` and `cargo test --all` still green (no new tests yet — foundational types are exercised via existing tests only).

---

## Phase 3: User Story 1 — First PDF from a built-in document type (Priority: P1) 🎯 MVP

**Goal**: `tex-cli build --json <path>` on a JSON with `document.type = "resume"` produces a PDF at a resolved output path with zero template authoring and zero install steps.

**Independent Test**: [quickstart.md § 1](./quickstart.md) — baseline compile of `/tmp/resume.json` using the built-in `resume` template. No trust prompt, no sandbox errors.

### Tests for User Story 1 (write first, ensure FAIL) ⚠️

- [ ] T008 [P] [US1] Write integration test covering Case A of [contracts/json-document-fields.md](./contracts/json-document-fields.md) (built-in match by `document.type = "resume"`) in `tests/cli_pipeline_resolve.rs`

### Implementation for User Story 1

- [ ] T009 [US1] Implement built-in template enumeration returning `Identifier` + `TemplateOrigin::BuiltIn { path }` for every template under `examples/templates/` in `src/templates.rs`
- [ ] T010 [US1] Implement `discovery::resolve()` step 2 (built-in match on `document.type`) with the FR-018 precedence rule in `src/discovery.rs`
- [ ] T011 [US1] Extend `Report` with `template_identifier: Identifier` and `template_version: Option<Version>` fields (per FR-010 rewrite) in `src/build.rs`
- [ ] T012 [US1] Route `build --json <source>` through discovery → existing render → existing `compiler::compile` (no sandbox for built-in) in `src/build.rs`
- [ ] T013 [US1] Add `--json <source>` argument (file path or `-` for stdin) to the existing `build` subcommand's `clap` definition in `src/cli.rs`
- [ ] T014 [US1] Wire the `--json` branch to the new build pipeline in `src/main.rs`
- [ ] T015 [US1] Add interactive menu entry "Compile a JSON to PDF" (prompts for JSON path via `inquire::Text`) in `src/interactive.rs`

**Checkpoint**: US1 is fully functional and testable independently — the quickstart step 1 passes end-to-end. Ship-ready MVP for anyone using only built-in templates.

---

## Phase 4: User Story 2 — Install and use a third-party template (Priority: P2)

**Goal**: `tex-cli template install <git-url|path>` places a third-party package; the first compile against JSON that resolves to it triggers a one-time trust prompt; approved compiles run inside the FR-017 sandbox.

**Independent Test**: [quickstart.md § 2–5](./quickstart.md) — install `/tmp/acme-invoice`, hit the trust prompt, compile, re-run without prompt, upgrade to `1.1.0` and observe re-prompt, then verify `\write18` is blocked in step 5.

### Tests for User Story 2 (write first, ensure FAIL) ⚠️

- [ ] T016 [P] [US2] Write integration test covering install from local path and from a Git URL fixture (per [contracts/cli-template-subcommands.md](./contracts/cli-template-subcommands.md) install section) in `tests/cli_template_install.rs`
- [ ] T017 [P] [US2] Write integration test covering first-use prompt, re-prompt on version bump (`1.0.0` → `1.1.0`), and denial path (per FR-016) in `tests/cli_template_trust.rs`
- [ ] T018 [P] [US2] Write integration test covering third-party template attempting `\write18` (FR-017 sandbox violation returns exit 5 with `SandboxError::ShellEscapeAttempted`) in `tests/cli_pipeline_sandbox.rs`

### Implementation for User Story 2

- [ ] T019 [US2] Implement `Manifest::load(path)` per [contracts/manifest-schema.md](./contracts/manifest-schema.md) — TOML parse, all three required-field checks, identifier regex, `Version` parse, entrypoint escape / existence / `.tex` extension checks — in `src/templates.rs`
- [ ] T020 [US2] Implement local-path install path in `src/install.rs` — copy source into `templates_dir()/<ns>/<name>/`, verify manifest post-copy, honor `--force` overwrite guard per Constitution V
- [ ] T021 [US2] Implement Git-URL install path in `src/install.rs` — resolve `which("git")` (returns `InstallError::GitBinaryMissing` if absent), `git clone` into `tempfile::TempDir`, then delegate to the local-path installer
- [ ] T022 [US2] Implement `TrustFile` load/save via `atomic::write_atomic` in `src/trust.rs`
- [ ] T023 [US2] Implement `TrustFile::is_trusted(id, ver)` and `TrustFile::grant(id, ver)` in `src/trust.rs`
- [ ] T024 [US2] Implement first-use trust prompt using `inquire::Confirm` (with the FR-017 sandbox summary as the prompt text) plus non-TTY refusal path (exit code 6) in `src/trust.rs`
- [ ] T025 [US2] Extend `discovery::resolve()` with installed-template enumeration and the ambiguity error listing every candidate (per FR-018) in `src/discovery.rs`
- [ ] T026 [US2] Implement `SandboxDirective::for_origin()` and the flag/env composition (`openout_any=p` env var, `--only-cached` arg, refuse `--shell-escape`) inside `src/compiler.rs`
- [ ] T027 [US2] Grow `compiler::compile()` signature to accept a `SandboxDirective` and thread it through in `src/compiler.rs`; update the built-in call site in `src/build.rs` to pass the no-op directive
- [ ] T028 [US2] Wire the trust check into the build pipeline (after discovery, before compile, only when `TemplateOrigin::Installed`) in `src/build.rs`
- [ ] T029 [US2] Add `tex-cli template install <source> [--force]` subcommand definition and dispatcher in `src/cli.rs`
- [ ] T030 [US2] Add `tex-cli template list [--json] [--include-builtin]` subcommand in `src/cli.rs`
- [ ] T031 [US2] Add `tex-cli template remove <identifier> [--yes]` subcommand — deletes package dir and drops matching trust records — in `src/cli.rs`
- [ ] T032 [US2] Add `tex-cli template trust <identifier> [--version <ver>] [--revoke]` subcommand in `src/cli.rs`
- [ ] T033 [US2] Add interactive menu entries "Install a template", "List installed templates", "Remove a template", "Manage trust" wiring to the same dispatchers in `src/interactive.rs`

**Checkpoint**: US2 is fully functional and testable independently — quickstart steps 2 through 6 pass end-to-end, including the ambiguity error in step 6.

---

## Phase 5: User Story 3 — Safe rendering of arbitrary user data (Priority: P2)

**Goal**: Every JSON value interpolated into LaTeX is escaped or transformed so no LaTeX special character can silently corrupt output or hard-fail the compile, regardless of whether the template is built-in or installed.

**Independent Test**: [spec.md § US3 Acceptance Scenarios](./spec.md) — JSON containing every LaTeX special (`& % $ # _ { } \ ~ ^`) round-trips into the PDF; unrepresentable unicode substitutes with a reported warning; template/engine syntax collisions are detected at template-load time.

### Tests for User Story 3 (write first, ensure FAIL) ⚠️

- [ ] T034 [P] [US3] Write integration test covering all 10 LaTeX specials round-trip + em-dash substitution reported in the warnings list + syntax-collision detection at load time in `tests/cli_pipeline_safe_render.rs`

### Implementation for User Story 3

- [ ] T035 [US3] Audit and extend the escape table in `src/render.rs` so every character listed in FR-005 acceptance scenario is covered; add unit tests inline in `src/render.rs`
- [ ] T036 [US3] Implement unicode-substitution path with `Warning` emission (per A-06 and FR-010's warnings field) in `src/render.rs`
- [ ] T037 [US3] Implement load-time detection of template-engine ↔ LaTeX macro collisions (FR-006) — scan the template for the known conflict patterns from spec 007 Context and emit a `RenderError::TemplateSyntaxCollision` naming the exact character sequence and line — in `src/templates.rs`

**Checkpoint**: US3 is independently verifiable; the safe-render test passes for both built-in and installed templates without regressions in US1/US2.

---

## Phase 6: User Story 4 — Zero-config first run (Priority: P3)

**Goal**: `tex-cli build --json <path>` on a machine with no `~/.config/tex/config.toml` produces the PDF, creates a default config, and reports where it was written — no separate `init` invocation required.

**Independent Test**: [spec.md § US4 Acceptance Scenario 1](./spec.md) — nuke `~/.config/tex/` and `~/.local/share/tex/`, run the pipeline against a valid built-in-matching JSON, verify the PDF is produced and the config file is created at the documented location.

### Tests for User Story 4 (write first, ensure FAIL) ⚠️

- [ ] T038 [P] [US4] Write integration test that removes any pre-existing `~/.config/tex/config.toml` fixture, invokes `tex-cli build --json`, asserts PDF produced + config auto-created + stdout mentions the config path in `tests/cli_zero_config.rs`

### Implementation for User Story 4

- [ ] T039 [US4] Add missing-config detection at the entry of the build pipeline; on absent config, delegate to `config::create_default()` and log the path (FR-007) in `src/build.rs`
- [ ] T040 [US4] Warm the Tectonic bundle cache during `tex-cli init` (existing subcommand) by running a no-op compile against `examples/templates/carta` (the smallest built-in — smallest bundle-cache footprint of the three A-04 templates) — enables all third-party compiles per research R1 cross-cutting note — in `src/config.rs` (or the init subcommand handler)

**Checkpoint**: US4 is independently verifiable; a fresh-machine scenario produces a PDF with a single command.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, changelog, roadmap updates, and full quickstart validation. No user-facing behaviour change beyond copy.

- [ ] T041 [P] Update `README.md` with the new `tex-cli build --json` and `tex-cli template …` subcommands, and add a link to `specs/007-auto-pdf-pipeline/quickstart.md`
- [ ] T042 [P] Add `CHANGELOG.md` entry documenting the new subcommand tree and the new `document.template` JSON field
- [ ] T043 [P] Update `ROADMAP.md` marking spec 007 as delivered
- [ ] T044 Run [quickstart.md](./quickstart.md) end-to-end inside the project Docker container per project convention; capture any deviations as follow-up issues
- [ ] T045 Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`; fix or justify every diagnostic
- [ ] T046 [P] Regression test asserting FR-009 / SC-007 — compile the same JSON twice against the same built-in template and assert byte-identical PDFs; then repeat against an installed template with a fixed version — in `tests/cli_pipeline_deterministic.rs`

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
