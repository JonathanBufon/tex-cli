# Feature Specification: Translate All User-Facing Strings to English

**Feature Branch**: `006-translate-to-english`

**Created**: 2026-07-18

**Status**: Draft

**Input**: User description: "Translate all user-facing strings from Portuguese to English (English-only, no i18n framework, no locale detection). Closes the last i18n blocker on the road to v1.0."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - International user reads `--help` and understands the CLI (Priority: P1)

A developer discovers `tex-cli` through crates.io or a GitHub search and runs `tex-cli --help` (or any subcommand's `--help`) to learn how to use it. They do not read Portuguese. Every subcommand description, flag description, argument description, and hint text they see is in English.

**Why this priority**: `--help` is the single most-read surface of any CLI. If a new user hits `tex-cli init --help` and sees Portuguese, they close the terminal. This blocks all downstream adoption.

**Independent Test**: Run `tex-cli --help` and `tex-cli <every-subcommand> --help` in a fresh environment. Every visible sentence is in English. No accented character (`ã`, `õ`, `ç`, `á`, `é`, `í`) appears except inside proper nouns like `LaTeX`. Verified by an automated test that captures `--help` output and asserts absence of a curated Portuguese-word denylist.

**Acceptance Scenarios**:

1. **Given** a fresh user opens a terminal, **When** they run `tex-cli --help`, **Then** every line of the help output is in English.
2. **Given** they inspect `tex-cli build --help`, **When** they read the description of each flag (`--keep-tex`, `--engine`, `--output`, `--force`, etc.), **Then** each description is in English.
3. **Given** they inspect `tex-cli config set --help`, **When** they read the key list, **Then** the surrounding prose is English (the key names themselves remain unchanged as they are user-facing config keys).

---

### User Story 2 - Error messages are actionable in English (Priority: P2)

A user hits an error condition (missing config, malformed JSON, compile failure, template not found, etc.) and reads the error message on stderr. Every error message and its remediation hint is in English. Exit codes are unchanged.

**Why this priority**: Error messages are the second-highest-friction surface. When things go wrong, users need to be told *what* is wrong and *how to fix it* in a language they can read. Portuguese errors leave English speakers stuck.

**Independent Test**: Trigger each of the 14 `TexError` variants (missing config → exit 10, unknown key → exit 12, template not found → exit 20, tera error → exit 30, malformed JSON → exit 31, compile failed → exit 40, engine not installed → exit 41, engine unsupported → exit 42, etc.) and assert that the stderr text is English and contains no Portuguese words from the denylist. Assert the exit code is unchanged.

**Acceptance Scenarios**:

1. **Given** no config exists, **When** the user runs `tex-cli config show`, **Then** stderr contains an English message pointing them to `tex-cli init` and the process exits with code 10.
2. **Given** an existing template with malformed JSON input, **When** the user runs `tex-cli build tpl bad.json`, **Then** stderr contains an English "Invalid JSON" message with the source path and exit code is 31.
3. **Given** a broken `.tex` file, **When** the user runs `tex-cli compile broken.tex`, **Then** stderr contains an English "Compile failed" message with the engine name and last ~30 lines of the log, and exit code is 40.
4. **Given** an unsupported engine name in config, **When** the user runs any compile-touching command, **Then** stderr contains an English "Engine not supported" message listing the accepted engine names, and exit code is 42.

---

### User Story 3 - Interactive prompts read as English (Priority: P3)

A user runs a subcommand without arguments in a TTY (e.g., `tex-cli init`, `tex-cli build`, `tex-cli templates`) and enters the interactive menu. Every prompt, every confirmation question, every selection list header is in English. Default suggestions and hints are in English.

**Why this priority**: Interactive mode is the friendliest onboarding surface but only reaches users who explicitly enter it. Lower daily reach than `--help` or errors, but still directly affects new-user experience.

**Independent Test**: Invoke each subcommand that has an interactive mode without arguments (`init`, `templates`, `render`, `compile`, `build`). Capture the prompt strings shown to the user via the `inquire` crate. Assert English text and denylist absence.

**Acceptance Scenarios**:

1. **Given** the user runs `tex-cli init` in a TTY, **When** the menu asks for the templates directory, **Then** the prompt text is in English (e.g., "LaTeX templates directory:").
2. **Given** the user runs `tex-cli build` in a TTY, **When** the menu asks whether to keep the intermediate `.tex`, **Then** the prompt text is in English.
3. **Given** an existing config, **When** the user runs `tex-cli init` and is asked to overwrite, **Then** the confirmation question is in English.

---

### User Story 4 - Success messages and default values are English (Priority: P3)

A user runs a command that succeeds, and the stdout summary (path of produced artifact, elapsed time) is in English. Any default flag value that used to be Portuguese (e.g., `--format humano`) is now English (`--format human`).

**Why this priority**: Success messages are small in volume but critical for scripts parsing stdout. The `--format` default value change is a pre-1.0 breaking change to the CLI surface; must ship before 1.0 per `SEMVER.md`.

**Independent Test**: Run each command that produces a stdout success line (`render`, `compile`, `build`) and assert the English message shape. Run `tex-cli templates list` and `tex-cli config show` with no `--format` flag and confirm the default output mode is `human`. Run with `--format human` and confirm it works. Run with `--format humano` and confirm the behavior matches the plan-phase decision (either rejected or accepted as deprecated alias).

**Acceptance Scenarios**:

1. **Given** a successful build, **When** the user runs `tex-cli build tpl data.json`, **Then** stdout contains an English message with the PDF path and total pipeline elapsed time in the format `PDF generated at <path>. Pipeline (render + compile) took X.Ys.` (or equivalent English phrasing).
2. **Given** the user runs `tex-cli templates list`, **When** no `--format` is passed, **Then** the output is in the human format (English rendering).
3. **Given** the user runs `tex-cli templates list --format human`, **Then** the command succeeds identically to the default.

---

### Edge Cases

- **What happens to scripts grepping stdout for Portuguese tokens?** They break. This is a documented pre-1.0 breaking change and is explicitly acceptable per `SEMVER.md`. Callers must update to match the new English strings after upgrading.
- **What happens to `--format humano`?** Decision deferred to plan phase — either hard-rejected (cleaner, breaks any script using it) or accepted as a deprecated alias for one minor cycle with a stderr warning (matches the SEMVER deprecation policy).
- **What happens to tests that assert Portuguese words in error messages?** They fail. As part of this feature, every affected test is updated to assert the new English tokens.
- **What happens to source-code comments (non-doc `//`) that are in Portuguese?** They are explicitly out of scope. They can be translated incrementally in future PRs; they are not user-facing.
- **What happens to identifiers, config keys, template names, and JSON keys?** They are unchanged. They are user-data or public-API surface, not translatable strings.
- **What happens to proper nouns (`LaTeX`, `tectonic`, `latexmk`, `xelatex`, `Tera`, `PDF`)?** Unchanged. They are proper names, not translatable.
- **What if a translated message becomes noticeably longer or shorter and breaks output alignment?** Accept the change. Output alignment is not part of the stability contract; the structural shape (path in first position, duration in second) is what scripts depend on.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every variant of `TexError` MUST render its `Display` implementation entirely in English. No Portuguese words remain in any variant's `#[error(...)]` message.
- **FR-002**: Every doc comment (`///`) on a clap subcommand, argument struct, or flag in `src/cli.rs` MUST be in English. These become the `--help` output.
- **FR-003**: Every `inquire::{Select,Text,Confirm}` prompt string in `src/interactive.rs` MUST be in English.
- **FR-004**: The stdout success message emitted by `build` MUST be in English and include the PDF path and total elapsed time in the same positional structure as before.
- **FR-005**: The stdout success message emitted by `compile` MUST be in English and include the PDF path and compile elapsed time in the same positional structure as before.
- **FR-006**: The stdout success message emitted by `render` (when writing to disk, not `--dry-run`) MUST be in English.
- **FR-007**: The default value of the `--format` flag on `templates list` and `config show` MUST be `human` (the previous default `humano` is replaced).
- **FR-008**: All existing exit codes MUST be preserved. Every `TexError` variant returns the same integer from `exit_code()` as before this feature.
- **FR-009**: Any stderr diagnostic message in `src/compiler.rs` or elsewhere (e.g., "Engine returned success but no PDF was produced" replacing the Portuguese equivalent) MUST be in English.
- **FR-010**: All unit and integration tests that assert on Portuguese tokens MUST be updated to assert on the corresponding English tokens.
- **FR-011**: The removed test `errors::tests::display_messages_are_in_portuguese` MUST be replaced by an equivalent English-assertion test.
- **FR-012**: Structural output shape (paths, durations, log tails, exit codes) MUST be preserved so that programmatic consumers depending on positional parsing remain compatible.
- **FR-013**: No new runtime dependencies MUST be introduced. No i18n framework (fluent, gettext, `rust-i18n`, etc.) is added.
- **FR-014**: No locale detection MUST be implemented. `LANG`, `LC_MESSAGES`, and other environment variables are ignored.
- **FR-015**: The plan phase MUST decide the fate of the string literal `humano` as a value for `--format`: either hard-rejected with an "Invalid format" error (`--format humano` fails at parse time) or accepted as a deprecated alias for one minor cycle (with a stderr deprecation warning). Whichever is chosen MUST be enforced with a test.

### Key Entities

*(This feature involves no new data entities. It is a pure string translation over existing surfaces.)*

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The full test suite (`cargo test --all` in the Docker container) passes with no regressions after translation. Baseline is ~197 tests; a small delta up or down is acceptable if driven by test-assertion updates.
- **SC-002**: `tex-cli --help` and `tex-cli <subcommand> --help` for every subcommand produce zero words from a curated Portuguese denylist (`Diretório`, `Sobrescrever`, `Nenhum`, `Compilação`, `renderizar`, `Manter`, `arquivo`, `caminho`, `padrão`, `usuário`, `Não`, `Últimas`, etc.).
- **SC-003**: Every exit code documented in `README.md` maps to exactly one `TexError` variant and vice versa. No orphans in either direction.
- **SC-004**: `grep -rnE "[À-ÿ]" src/ --include="*.rs" | grep -vE '^\s*//' | grep -v 'test'` returns zero matches inside `#[error]` messages, `///` doc comments, `println!`/`eprintln!`/`writeln!` string arguments, and `inquire` prompt strings. (Accented characters remaining in `//` comments or test bodies are out of scope.)
- **SC-005**: `tex-cli templates list --format human` and `tex-cli config show --format human` succeed. Behavior of `--format humano` matches the plan-phase decision (rejected OR accepted with a deprecation warning) and is tested.
- **SC-006**: Every user-visible message that previously included a `path` or `duration` value still includes it, in the same positional order, so programmatic parsing by `awk '{print $NF}'` or similar remains viable.

## Assumptions

- **Target users are English-literate.** No support for readers who can only read Portuguese. This is a deliberate pre-1.0 choice, aligned with the open-source-audience decision documented in `ROADMAP.md`.
- **Users currently grepping stdout for Portuguese tokens will update their scripts.** Documented as a breaking change in `CHANGELOG.md` under `[Unreleased] > Changed` or `[Unreleased] > Breaking`.
- **Source-code comments in Portuguese remain until translated incrementally.** Non-doc `//` comments are for maintainers, not users, and are not part of the `--help` or error output.
- **Proper nouns are not translated.** `LaTeX`, `tectonic`, `latexmk`, `xelatex`, `lualatex`, `Tera`, `PDF`, `JSON`, `TOML` all stay unchanged.
- **Config keys, engine names, template names, and JSON payload keys are user-facing but not natural language.** They are not translated. `paths.templates_dir` stays `paths.templates_dir`.
- **The plan phase will decide the `humano` deprecation strategy.** Spec deliberately leaves this open (FR-015) to be settled by architectural discussion in `plan.md`.
- **No new dependencies are added.** The change is text-only and does not require any new crate.
- **This feature is a prerequisite for `v1.0.0`.** Once merged, another pre-1.0 blocker (i18n decision) is cleared per `ROADMAP.md`.
