# Phase 0 — Research: Translate All User-Facing Strings to English

**Feature**: `006-translate-to-english`
**Date**: 2026-07-18

Resolves the architectural decisions raised in `spec.md` (FR-015 in
particular) and locks in the style guide + test-refactor strategy
before implementation begins.

---

## D-01. `--format humano` fate (resolves FR-015)

**Decision**: **Hard-reject `humano`**. `--format humano` fails at
`clap` parse time with the same error clap gives for any unknown
value (exit 2).

**Rationale**:

- The project is at `0.1.x`. Per [`SEMVER.md`](../../SEMVER.md),
  breaking changes to the CLI surface are explicitly allowed
  pre-1.0. This is the last window to break things cleanly.
- Keeping `humano` as a deprecated alias contradicts the English-
  only stance the whole feature is establishing. The alias would
  advertise "Portuguese is a first-class option," which is exactly
  the message we want to retire.
- Deprecation alias adds branching in clap value parsing (`enum
  Format { Human, Humano }` or `#[value(alias = "humano")]`) and
  needs a stderr warning path. More code, more branches, more
  tests. Not worth it for a pre-1.0 alias.
- Users on scripts using `--format humano` need a one-line change:
  `s/humano/human/`. Documented in `CHANGELOG.md` under
  `[Unreleased] > Breaking`.

**Alternatives considered**:

- **Deprecated alias with stderr warning** — clap supports `#[value(alias = "humano")]`.
  Rejected because it prolongs a legacy we are trying to remove and
  the migration cost for callers is trivially small.
- **Silent alias (no warning)** — worst option. Hides the breaking
  intent, blocks future removal without another deprecation cycle.

**Test coverage**: One integration test asserts `--format humano`
exits with code 2 (clap parse error) and stderr mentions the accepted
values. One asserts `--format human` succeeds. One asserts the
default (no `--format` flag) matches `--format human`.

---

## D-02. Translation approach: literals in place, no i18n framework

**Decision**: All strings become **plain English `&str` literals in
place**. No `t!()` macro, no fluent bundles, no `LANG` sniffing, no
`match locale` blocks anywhere.

**Rationale**:

- Explicitly required by FR-013 and FR-014.
- The project has one target audience (English-literate developers).
  A framework for one language is over-engineering.
- Adding an i18n crate adds a compile-time dependency, a runtime
  dependency, and a maintenance surface. All three violate the
  project's minimal-dep philosophy.
- If future demand for multi-language ever appears, the migration
  path is well-known (extract literals to constants, then to a
  bundle). Doing it now speculatively is a violation of the "no
  design for hypothetical futures" rule in CLAUDE.md.

**Alternatives considered**:

- `rust-i18n` — compile-time macro that keys strings to YAML
  locale files. Nice DX but adds a build dep for zero present
  benefit.
- `fluent-rs` (Mozilla's Fluent) — powerful but heavy for a CLI.
  Rejected on complexity.
- Constants module (`src/messages.rs`) with `pub const MSG_FOO:
  &str`. Rejected: extra indirection without benefit; translators
  and reviewers prefer the message next to the code that emits it.

---

## D-03. Message style guide

**Decision**: Adopt a consistent English style across all user-facing
messages. Locked in below.

**Errors** (`TexError` `#[error("...")]`):

- **Sentence case** (`No config found. Run 'tex-cli init' first.`)
- **Full stops** at the end of every sentence.
- **Active voice**, **present or past tense** as appropriate to the
  verb. Avoid future tense.
- **Actionable remediation** in the second sentence when a fix is
  obvious. Example: `Template '{name}' not found in {templates_dir}.`
  followed on the same line by `Run 'tex-cli templates list' to see
  what is available.` Only include the remediation when it clarifies
  next steps; do not pad.
- **File paths, engine names, key names** are wrapped in
  single-quotes (`'tectonic'`, `'compiler.engine'`) or backticks in
  Markdown docs (in messages, single quotes are used since terminals
  don't render Markdown).
- **Interpolated values** stay in the same positions they occupied
  in the Portuguese version to preserve structural output shape
  (see FR-012).

**Prompts** (`inquire::{Select,Text,Confirm}`):

- **Imperative or question form**. Examples: `LaTeX templates
  directory:`, `Overwrite existing config?`, `Choose a template:`.
- **End with a colon** for Text/Select, a **question mark** for
  Confirm. Never both.
- **No trailing space** after the punctuation — `inquire` adds
  spacing.

**Success stdout messages**:

- **Past tense** for confirmations (`PDF generated at ...`).
- **Include the artifact path first**, elapsed time second.
- Format: `PDF generated at <path>. Pipeline (render + compile)
  took X.Ys.` for `build`; `PDF generated at <path>. Compile took
  X.Ys.` for `compile`; `.tex written to <path>.` for `render`
  (when not `--dry-run`).

**Help text** (clap `///`):

- **Descriptive fragments**, not full sentences unless clarifying
  a behavior. Terse but complete.
- Match clap's own tone: short first line summary, longer detail
  below on a second `///` line only when needed.

**Style anti-patterns to avoid**:

- Redundant "please" or "kindly" — CLI tools do not beg.
- Emoji — banned by CLAUDE.md.
- Capitalized `LATEX` or `PDF` inside prose sentences — use
  `LaTeX`, `PDF` (already correct capitalization).
- Passive voice in error messages (`The file could not be found`
  is worse than `File '{path}' not found.`).

---

## D-04. Test-refactor strategy

**Decision**: **Systematic sweep, file by file**, using the
following process:

1. **Enumerate PT test assertions** before starting via
   `grep -rnE 'assert!\(.*contains.*"[À-ÿ]"' tests/ src/` and
   `grep -rn '"[^"]*[À-ÿ][^"]*"' src/ tests/`.
2. For each hit:
   - If the assertion pins the **structural shape** (path
     position, exit code, duration position), update the token to
     English but keep the structural check.
   - If the assertion pins a **specific Portuguese word**
     (`renderizar`, `cancelada`, `Últimas linhas`), replace with
     the corresponding English word from the style guide (see
     [contracts/cli.md](./contracts/cli.md)).
   - If the test **name** contains Portuguese (`display_messages_are_in_portuguese`),
     rename to the English equivalent (`display_messages_are_in_english`).
3. **Do NOT change test behavior**. Only assertion tokens.
4. Run `cargo test --all` after each file's sweep to catch
   regressions early. Do NOT bulk-edit and hope.

**Enumeration output** (from grep during planning):

- `src/errors.rs::tests` — 14 assertions on PT tokens across 12
  tests + 1 name change (`display_messages_are_in_portuguese`).
- `tests/cli_build.rs` — ~5 assertions on PT tokens (stdout format,
  log-tail delimiter).
- `tests/cli_compile.rs` — ~5 assertions on PT tokens (stdout
  format, engine-not-supported list).
- `tests/cli_config_set.rs` — ~3 assertions on PT tokens (unknown
  key, invalid bool).
- `tests/cli_config_show.rs` — 1 assertion on `humano`.
- `tests/cli_init.rs` — ~2 assertions on prompt regex.
- `tests/cli_render.rs` — ~4 assertions on PT tokens (tera error,
  invalid JSON).
- `tests/cli_templates_*` — ~3 assertions each on PT tokens (list
  header, add/remove errors).

**Total**: ~25 assertion updates + 1 test rename.

---

## D-05. Rollout structure: per-file, not per-user-story

**Decision**: Task granularity in `tasks.md` groups tasks by **file
touched**, not by user story from `spec.md`.

**Rationale**:

- The four user stories (`--help`, errors, prompts, success + default)
  are conceptually distinct but code-wise **overlap heavily** in
  each file. `src/cli.rs` alone touches US1 (help text) and US4
  (`--format` default). `src/errors.rs` covers US2 exclusively but
  every test asserting on errors lives elsewhere.
- Reviewers benefit from seeing all changes to `src/errors.rs` in
  one task, then all to `src/cli.rs` in the next, rather than
  jumping across files per user story.
- Test discipline: run `cargo test` after each file's change to
  isolate regressions to a single blast radius.

**Task shape** (rendered in full in `tasks.md` after
`/speckit-tasks`):

- **T001** [P1] — Translate `src/errors.rs` (14 `TexError` variants
  + inline test assertions + rename `display_messages_are_in_portuguese`).
- **T002** [P1] — Translate `src/cli.rs` (all `///` doc comments +
  `--format` default `humano` → `human`).
- **T003** [P3] — Translate `src/interactive.rs` (all `inquire`
  prompts).
- **T004** [P3] — Translate stdout success in `src/build.rs`,
  `src/compile.rs` (via `cli.rs`), `src/render.rs`. (`cli.rs`
  handles compile/build stdout, so this is small.)
- **T005** [P4] — Translate remaining stderr diagnostics in
  `src/compiler.rs`.
- **T006-T014** — Update PT test assertions per test file
  (`tests/cli_*.rs`).
- **T015** — Update `README.md` sections quoting stdout (search:
  "Contratos importantes" text blocks and success-message
  examples).
- **T016** — Add `CHANGELOG.md` entry under `[Unreleased] > Breaking`.
- **T017** — Add `--format humano` rejection test.
- **T018** — Add English-denylist smoke test capturing
  `--help` output.
- **T019** — Full-suite verification in Docker; fmt + clippy + tests
  green.

**Not doing per-story tasks**. Justification: file-shaped review is
strictly easier for this feature; the story shape is preserved
only in `spec.md` for user-outcome tracking.

---

## D-06. README updates in sync

**Decision**: Update the following `README.md` sections in the same
PR as the code changes (or a preceding one, but not later — docs
must stay accurate).

**Sections that quote actual command output**:

- Line ~187: `stdout` reports the PDF path and compile time:
  `PDF gerado em <path>. Compilação levou X.Ys.` → English.
- Line ~219: `stdout` reports the PDF path and total pipeline time:
  `PDF gerado em <path>. Pipeline (render + compile) levou X.Ys.` →
  English.
- Any other "PDF gerado" / "Compilação" quotes.

**Sections that show default values**:

- Line ~68: `tex-cli config show                    # default: human`
  — already English, no change.
- Line ~96: `tex-cli templates list                     # default: human`
  — already English, no change.
- (Both were rendered as English in the earlier README translation
  PR; only the underlying default value in `cli.rs` needs updating.)

**Not touched**: Everything else in `README.md` is already English
after the earlier translation PR (see git history: `docs(readme): translate to English`).

---

## D-07. CHANGELOG classification

**Decision**: Land the changelog entry under `[Unreleased] > Breaking`
(a new subheading, since `Keep a Changelog` uses `Changed` for
non-breaking behavior changes and doesn't specify a `Breaking`
subheading — but Keep-a-Changelog allows custom subheadings).

Actually, on re-check: the Keep-a-Changelog canonical categories are
`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`. To
stay canonical, use **`Changed`** with a **`BREAKING:` prefix** on
each bullet.

Example entry:

```markdown
### Changed

- **BREAKING**: All user-visible strings translated from Portuguese
  to English. This affects `--help` output, error messages,
  interactive prompts, and stdout success messages. Scripts that
  grep stdout or stderr for specific Portuguese tokens must be
  updated. Exit codes are unchanged; structural output shape (path
  position, duration position) is unchanged. See `specs/006-translate-to-english/`
  for full detail.
- **BREAKING**: The `--format` flag default value renamed from
  `humano` to `human` on `templates list` and `config show`. Passing
  `--format humano` now fails with a clap parse error (exit 2).
```

---

## Cross-references

- Spec: [`spec.md`](./spec.md)
- Plan: [`plan.md`](./plan.md)
- Contracts: [`contracts/cli.md`](./contracts/cli.md)
- Quickstart: [`quickstart.md`](./quickstart.md)
- Versioning policy: [`../../SEMVER.md`](../../SEMVER.md)
- Roadmap: [`../../ROADMAP.md`](../../ROADMAP.md)
