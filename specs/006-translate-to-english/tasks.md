---
description: "Task list for feature 006-translate-to-english"
---

# Tasks: Translate All User-Facing Strings to English

**Input**: Design documents from `/specs/006-translate-to-english/`

**Prerequisites**: plan.md (required), spec.md (required for user
stories), research.md, contracts/cli.md, quickstart.md.

**Tests**: Included where applicable — the feature intentionally
touches ~25 existing test assertions plus adds 2 new tests
(humano-rejection, `--help` denylist smoke).

**Organization**: Per-file rollout as locked in
[research.md § D-05](./research.md). User-story labels
([US1]-[US4]) are kept on each task for traceability back to
`spec.md`, but the execution order groups by file rather than by
story because the files change disjointly and file-shaped review is
strictly easier for a translation refactor.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on
  incomplete tasks)
- **[Story]**: Which user story this task satisfies (see
  [spec.md § User Scenarios](./spec.md))
- File paths are relative to the repo root.

## Path Conventions

- Rust source: `src/`
- Integration tests: `tests/`
- Docs: repo root (`README.md`, `CHANGELOG.md`, `ROADMAP.md`)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the branch baseline is clean before any
translation begins.

- [x] T001 Confirm baseline is green by running
  `docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all`
  on the current `006-translate-to-english` branch. Expected: **~197
  tests pass, zero regressions vs. `main`**. If any test fails
  before the feature starts, STOP and fix the environment before
  proceeding.

**Checkpoint**: Green baseline. No tasks in Phase 2 (feature is a
text refactor, no scaffolding to lay).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: N/A for this feature.

This is a text-only refactor. There are no new modules, no
migrations, no scaffolding. Every subsequent task edits files that
already exist. Skip to Phase 3.

---

## Phase 3: Source Translation (per-file)

**Purpose**: Translate every user-visible Rust string literal from
Portuguese to English, one file per task, per the style guide in
[research.md § D-03](./research.md) and the string-by-string
mapping in [contracts/cli.md](./contracts/cli.md).

- [x] T002 [US2] Translate every `#[error("...")]` message in
  `src/errors.rs` (14 `TexError` variants) per
  [contracts/cli.md § 1](./contracts/cli.md#1-texerror-display-messages-srcerrorsrs).
  In the same file, update the inline `#[cfg(test)] mod tests`
  block: (a) rewrite every `assert!(msg.contains("<PT-token>"))`
  to the corresponding English token from the contract, (b) rename
  `display_messages_are_in_portuguese` → `display_messages_are_in_english`
  and update its two body assertions to English tokens
  (`config` → `config`, `cancelada` → `cancelled`). Preserve every
  exit-code integer (`10, 11, 12, 13, 14, 15, 20, 21, 22, 30, 31,
  40, 41, 42, 1`). Run `cargo test --lib` in Docker; must be green
  before moving on. **Satisfies FR-001, FR-008, FR-010, FR-011.**

- [x] T003 [US1] [US4] Translate every `///` doc comment in
  `src/cli.rs` per
  [contracts/cli.md § 2](./contracts/cli.md#2---help-doc-comments-srclirs) —
  top-level `Cli`, every subcommand descriptor, every arg-struct
  descriptor, every flag. Also change the two `--format`
  `default_value` literals from `"humano"` to `"human"` per
  [contracts/cli.md § 3](./contracts/cli.md#3---format-default-value)
  (one on `templates list`, one on `config show`). Also translate
  the three stdout success message string literals in the
  `handle_build`, `handle_compile`, `handle_render` handlers per
  [contracts/cli.md § 5](./contracts/cli.md#5-stdout-success-messages).
  Run `cargo build --release` in Docker to confirm the file
  compiles. **Satisfies FR-002, FR-004, FR-005, FR-006, FR-007.**

- [x] T004 [P] [US3] Translate every `inquire::{Select,Text,Confirm}`
  prompt string in `src/interactive.rs` per
  [contracts/cli.md § 4](./contracts/cli.md#4-interactive-prompts-srcinteractiveresponsers)
  (16 prompts). Preserve interpolation positions (`{}` for `path`,
  `name`, etc.). Run `cargo build` to confirm. **Satisfies FR-003.**

- [x] T005 [P] Translate the stderr diagnostic message in
  `src/compiler.rs` at line ~167
  (`"Engine retornou sucesso mas nenhum PDF foi produzido."`) to
  its English equivalent per
  [contracts/cli.md § 6](./contracts/cli.md#6-stderr-diagnostics-srccompilerrs).
  Run `cargo build` to confirm. **Satisfies FR-009.**

- [x] T006 [P] Sweep `src/build.rs` and `src/render.rs` for any
  remaining user-facing Portuguese in `println!`/`eprintln!`/error
  format strings that were not caught by T003. Any hit is
  translated in-place per the style guide in
  [research.md § D-03](./research.md#d-03-message-style-guide).
  If the grep returns zero, this task is a confirmation-only
  no-op — record the empty grep output in the task's commit
  message. **Reinforces FR-004, FR-006, FR-012.**

**Checkpoint**: All source-code Portuguese strings translated.
`grep -rnE '"[^"]*[À-ÿ][^"]*"' src/ --include="*.rs" | grep -vE '^\s*[[:print:]]*//'`
should return zero string-literal matches (accented characters in
`//` comments and in test-body assertions are still permitted —
those get swept in Phase 4). **Satisfies SC-004** (partial).

---

## Phase 4: Test Assertion Updates (per test file)

**Purpose**: Update every integration test that pins a Portuguese
token in `assert!(...)` or `predicates::str::contains(...)`. Task
per test file; all fully parallelizable because each touches a
distinct file.

- [x] T007 [P] [US2] Update `tests/cli_build.rs`: replace every
  `assert!(msg.contains("<PT-token>"))` and every
  `predicate::str::contains("<PT-token>")` referencing tokens from
  [contracts/cli.md § 1](./contracts/cli.md#1-texerror-display-messages-srcerrorsrs)
  or [§ 5](./contracts/cli.md#5-stdout-success-messages) with the
  English equivalent. Preserve exit-code assertions unchanged. Run
  `cargo test --test cli_build` in Docker; green before merging.

- [x] T008 [P] [US2] Update `tests/cli_compile.rs` assertions per
  the same procedure as T007. Focus areas: `Compile failed` +
  `Engine not supported` + `Compile took` + PDF path shape.
  Run `cargo test --test cli_compile` in Docker; green.

- [x] T009 [P] [US2] Update `tests/cli_config_set.rs` assertions
  (unknown key list, invalid bool value error). Run
  `cargo test --test cli_config_set`; green.

- [x] T010 [P] [US4] Update `tests/cli_config_show.rs`: (a) any
  existing assertion that referenced the `humano` default output
  now expects the `human` header, (b) any test named or asserting
  around the `humano` string value is either renamed or moved to
  the humano-rejection test in T016. Run
  `cargo test --test cli_config_show`; green.

- [x] T011 [P] [US3] Update `tests/cli_init.rs`: any regex
  assertion that pinned a Portuguese word from the interactive
  init flow (`Diretório`, `Compilador`, `Sobrescrever`) is
  replaced with the English token from
  [contracts/cli.md § 4](./contracts/cli.md#4-interactive-prompts-srcinteractiveresponsers).
  Run `cargo test --test cli_init`; green.

- [x] T012 [P] [US2] Update `tests/cli_render.rs` assertions
  (tera error message, invalid JSON message). Run
  `cargo test --test cli_render`; green.

- [x] T013 [P] [US2] Update `tests/cli_templates_add.rs`
  assertions (invalid UTF-8, overwrite prompt). Run
  `cargo test --test cli_templates_add`; green.

- [x] T014 [P] [US4] Update `tests/cli_templates_list.rs`: (a)
  `--format humano` test is either renamed to the humano-rejection
  case in T016 or moved there, (b) any header/label PT strings in
  the human-format output are updated, (c) `--format json` tests
  stay unchanged. Run `cargo test --test cli_templates_list`;
  green.

- [x] T015 [P] [US2] Update `tests/cli_templates_remove.rs` and
  `tests/cli_templates_show.rs` assertions (remove-confirm prompt,
  template-not-found error). Run
  `cargo test --test cli_templates_remove --test cli_templates_show`;
  green.

**Checkpoint**: All existing tests pass with English assertions.
`cargo test --all` green in Docker.

---

## Phase 5: New Tests (contract enforcement)

**Purpose**: Add tests for behaviors introduced by this feature.

- [x] T016 [US4] Add a new integration test asserting **`--format
  humano` is rejected**: run `tex-cli templates list --format humano`
  and `tex-cli config show --format humano`, assert both exit with
  code **2** (clap parse error), assert stderr contains the string
  `human` in the accepted-values list. Place in either
  `tests/cli_templates_list.rs` or a new dedicated file
  `tests/cli_humano_rejected.rs`. Verifies D-01 decision and
  **satisfies FR-015, SC-005.**

- [x] T017 [US1] Add a new integration test capturing the output
  of `--help` and asserting absence of every word in a curated
  Portuguese denylist. Place in a new file
  `tests/cli_help_english.rs`. Denylist entries (minimum):
  `Diretório`, `Sobrescrever`, `Nenhum`, `Compilação`, `renderizar`,
  `Manter`, `arquivo`, `caminho`, `padrão`, `usuário`, `Não`,
  `Últimas`, `humano`, `Gerencia`, `Descarta`, `Copia`, `Cria`.
  The test iterates over the top-level `tex-cli --help` plus every
  subcommand's `--help` output. Any hit fails the test with the
  offending word and subcommand. **Satisfies SC-002.**

**Checkpoint**: New contract behaviors covered by tests.
`cargo test --all` green.

---

## Phase 6: Documentation Sync

**Purpose**: Bring user-facing docs in sync with the new strings so
copy-paste examples and quoted output blocks in `README.md` remain
accurate.

- [x] T018 Update `README.md` sections that quote actual command
  output per [research.md § D-06](./research.md#d-06-readme-updates-in-sync):
  (a) the `compile` "Important contracts" section's stdout example
  now reads `PDF generated at <path>. Compile took X.Ys.`,
  (b) the `build` "Important contracts" section's stdout example
  now reads `PDF generated at <path>. Pipeline (render + compile)
  took X.Ys.`, (c) any other `PDF gerado` / `Compilação` /
  `renderizar` quotes replaced with their English equivalents.
  Also verify no lingering `humano` mentions in README.

- [x] T019 Add a `[Unreleased] > Changed` entry to `CHANGELOG.md`
  with the **`BREAKING:`** prefix per
  [research.md § D-07](./research.md#d-07-changelog-classification).
  Two bullets: (1) user-visible strings translated PT → EN
  (scripts grepping stdout must update), (2) `--format` default
  renamed `humano` → `human` and `--format humano` now fails with
  exit code 2.

**Checkpoint**: README and CHANGELOG accurate. External-facing
docs done.

---

## Phase 7: Polish & Verification

**Purpose**: Full-suite gate before opening the PR.

- [x] T020 **Full verification** in the Docker container:
  1. `cargo fmt --all -- --check` — clean.
  2. `cargo clippy --all-targets --all-features -- -D warnings` —
     clean.
  3. `cargo test --all` — all tests green (~197 baseline + 2 new
     from Phase 5 = ~199 expected).
  4. **Manual `--help` sweep**: run `tex-cli --help` plus each of
     the 6 subcommand `--help` calls and eyeball the output for
     any remaining Portuguese. This catches errors the denylist
     in T017 might miss.
  5. **Static grep gate**:
     `grep -rnE '"[^"]*[À-ÿ][^"]*"' src/ --include="*.rs" \
      | grep -vE '^\s*[[:print:]]*//' | grep -v '#\[cfg(test)\]' | grep -v 'mod tests'` —
     zero hits inside `#[error]`, `///`, `println!`/`eprintln!`
     string args, and `inquire` prompts. **Satisfies SC-004
     (final).**
  6. Update `ROADMAP.md` — flip the i18n blocker checkbox to `[x]`
     with a "done 2026-07-18, spec 006" note.

  If all six steps pass, the feature is ready to PR against `main`.
  **Satisfies SC-001, SC-003, SC-006.**

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: N/A (skipped).
- **Source translation (Phase 3)**: depends on Phase 1.
- **Test updates (Phase 4)**: depends on Phase 3 (source strings must
  be their English versions before test assertions are rewritten to
  match).
- **New tests (Phase 5)**: depends on Phase 3 (T016 needs the
  `humano→human` default flip in place; T017 needs the translated
  `--help` output to actually be English).
- **Docs (Phase 6)**: depends on Phase 3 (README quotes must match
  the actual strings). Independent of Phases 4 and 5.
- **Verification (Phase 7)**: depends on **everything above**.

### Task-level dependencies

- T003 depends on T002 (errors first so any cli.rs error path in
  the doc comments references the right English forms).
- T004, T005, T006 can each start after T003 lands (they touch
  disjoint files).
- All of T007-T015 can start once T002+T003+T004+T005+T006 are
  complete (they read the new English strings and mirror them
  into assertions). **All are parallel** across each other.
- T016, T017 can start once Phase 3 is complete. Parallel across
  each other.
- T018, T019 can start once Phase 3 is complete. Parallel across
  each other and across T016/T017.
- T020 is the final gate.

### Parallel opportunities

- **Phase 3**: T004, T005, T006 can run in parallel with each other
  once T003 lands.
- **Phase 4**: T007-T015 are all `[P]` — all 9 can be executed in
  parallel (different files, no shared state).
- **Phase 5**: T016 and T017 are independent.
- **Phase 6**: T018 and T019 are independent.

### Solo-developer strategy

If executed serially by one person, the natural order is
T001 → T002 → T003 → T004 → T005 → T006 → T007…T015
(in any order) → T016, T017 → T018, T019 → T020. Estimated
end-to-end: **1-2 focused days**.

---

## User story ↔ task traceability

| User story                                             | Tasks that satisfy it                                   |
|--------------------------------------------------------|---------------------------------------------------------|
| US1 (`--help` in English)                              | T003, T017                                              |
| US2 (Error messages in English)                        | T002, T007, T008, T009, T012, T013, T015                |
| US3 (Interactive prompts in English)                   | T004, T011                                              |
| US4 (Success stdout + `--format human` default)        | T003, T010, T014, T016                                  |

Every user story from `spec.md` has at least one task; every FR
from `spec.md` is referenced by at least one task; every SC from
`spec.md` is verifiable by the checkpoint at the end of the
task's phase.

---

## Notes

- `[P]` tasks touch different files with no shared state.
- Test-file tasks in Phase 4 are all `[P]` — a single reviewer can
  batch-review the PR because the diffs are file-scoped.
- The two new tests in Phase 5 are the only *behavior* additions;
  everything else is a text substitution.
- The `humano` deprecation strategy is **hard-reject** per
  [research.md § D-01](./research.md#d-01---format-humano-fate-resolves-fr-015).
  No alias, no warning path, no code branches. Simpler.
- Commit granularity: prefer one commit per task. This keeps the
  final PR reviewable (~19 focused commits) and matches the
  project's spec-kit conventions from specs 001-005.
