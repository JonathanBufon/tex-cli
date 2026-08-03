# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — spec 007 (auto PDF pipeline)

- **`tex-cli build --json <path>`** — one-command JSON → PDF pipeline.
  Resolves the template from the JSON's `document.type` (matched against
  built-in library first, then installed) or explicit `document.template`
  (`namespace/name` for installed, bare name for built-in). Prints
  `Template: <identifier>` on success (FR-010). See
  [`specs/007-auto-pdf-pipeline/spec.md`](specs/007-auto-pdf-pipeline/spec.md).
- **`tex-cli template install|list|remove|trust`** — new subcommand
  tree for third-party installable templates. `install` accepts a Git
  URL (shelled out to `git`, no new crate) or a local filesystem path.
  Packages ship a `tex-template.toml` manifest with `identifier`,
  `version` (semver), and `entrypoint` fields. Layout at
  `~/.local/share/tex/templates/<namespace>/<name>/`.
- **Trust model** — installed third-party templates require explicit
  per-`(identifier, version)` approval before their first compile
  (FR-016). Approvals persist to `~/.local/share/tex/trust.toml`.
  Version bumps re-prompt. Non-TTY invocations exit `70`
  (`TrustDenied`) rather than blocking.
- **Sandbox** — installed third-party template compiles run with
  `\write18` disabled, `--only-cached` (no bundle-network fetch), and
  `openout_any=p` (KPathsea paranoid mode) — FR-017. Built-in
  templates keep the compiler's default. Sandbox violations map to
  exit codes `90..=92`.
- **Zero-config first run** — `build --json` in a bare environment
  auto-creates the config at `~/.config/tex/config.toml` with sensible
  defaults (templates_dir under `~/.config/tex/templates`, output_dir
  = cwd, engine = tectonic) and prints a "note" to stderr. FR-007.
- **Tectonic bundle-cache warm-up** — `tex-cli init` runs a minimal
  no-op compile after writing the config so subsequent third-party
  compiles (which require `--only-cached`) don't need a separate
  priming step. Skipped when the engine is not tectonic.
- New exit codes `50..=92` grouped by concern (install, manifest,
  trust, resolve, sandbox). See the README exit-code table.
- Extended interactive menu — `template_menu` now surfaces both
  built-in and third-party actions on equal footing (Constitution III).
- New value objects `Identifier` (bare or `namespace/name`) and
  `Version` (semver-lite) — manual char/regex validation, no new deps.

### Changed — spec 007 (BREAKING)

- **Universal LaTeX escaping of JSON string values (FR-005)** — every
  string in the user JSON is now escaped for the ten LaTeX specials
  (`\ & % $ # _ { } ~ ^`) before Tera sees it. Em-dash `—` → `---` and
  en-dash `–` → `--` are substituted with the substitution logged via
  `tracing::warn!` (A-06). **Migration**: JSON values that previously
  embedded raw LaTeX (e.g. `"date": "\\today"`) will now render as
  literal text. Move LaTeX macros into the template body and keep JSON
  as pure data (Constitution II). The pre-007 test that documented
  unsafe passthrough (`render_template_no_autoescape_for_latex`) has
  been replaced with `render_template_escapes_latex_specials_universally`.
- **Load-time template↔LaTeX collision detection (FR-006)** — templates
  containing `\macro{#N}` patterns (the canonical `\MakeUppercase{#1}`
  bug) now fail with a targeted diagnostic naming the macro and line
  number instead of an opaque Tera parse error.
- `BuildOutcome` gained `template_identifier: Identifier` and
  `template_version: Option<Version>` fields (populated by the new
  `build --json` pipeline; the pre-007 explicit `build TEMPLATE JSON`
  path fills identifier from the given name and leaves version None).

### Added — pre-spec-007

- Collaboration documentation: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, `CHANGELOG.md`.
- GitHub issue and pull-request templates under `.github/`.
- GitHub Actions CI running `fmt --check`, `clippy -D warnings`, and
  the full test suite.
- Dual license under **MIT OR Apache-2.0**, matching the Rust
  ecosystem convention. Apache-2.0 adds an explicit patent grant that
  MIT alone did not provide.

- `ROADMAP.md` — tracks blockers, recommended items, and explicitly
  deferred scope on the road to `v1.0.0`.
- `SEMVER.md` — versioning policy defining the public surface,
  version-bump rules, deprecation policy, pre-1.0 behavior, MSRV
  stance, and release process.

### Fixed

- Removed dead `TexError::EngineBinaryMissing` variant. It duplicated
  `EngineNotInstalled` semantically but had no call sites and would
  have produced exit code `1` (generic) instead of `41` (engine not
  on PATH). Discovered during the pre-1.0 API audit.

### Changed

- Renamed `LICENSE` to `LICENSE-MIT` and added `LICENSE-APACHE`.
- `Cargo.toml`: replaced `license-file = "LICENSE"` with
  `license = "MIT OR Apache-2.0"` (SPDX expression).
- **BREAKING**: All user-visible strings translated from Portuguese to
  English. This affects `--help` output, `TexError` messages,
  interactive prompts, stdout success messages, and stderr diagnostics.
  Scripts that grep stdout or stderr for specific Portuguese tokens
  must be updated. Exit codes are unchanged; structural output shape
  (path position, duration position) is unchanged. See
  [`specs/006-translate-to-english/`](specs/006-translate-to-english/)
  for the full contract mapping.
- **BREAKING**: The `--format` flag default value renamed from `humano`
  to `human` on `templates list` and `config show`. Passing
  `--format humano` now fails with a clap parse error (exit code 2).

## [0.1.0] — 2026-07-18

Initial release. Closes the JSON → PDF pipeline end-to-end.

### Added

- **`init`** — bootstrap `~/.config/tex/config.toml` with templates
  directory, output directory, and default engine (spec 001).
- **`config show` / `config set`** — inspect and update config keys
  with atomic writes and validation (spec 001).
- **`templates list` / `show` / `add` / `remove`** — manage the LaTeX
  template catalog with human and JSON output modes (spec 002).
- **`render`** — apply JSON data to a Tera template and emit `.tex`
  to stdout, a file, or a temp directory (spec 003).
- **`compile`** — turn a `.tex` file into a PDF via the configured
  engine (tectonic by default, latexmk/xelatex supported) with
  atomic writes and log tail on failure (spec 004).
- **`build`** — orchestrate `render` + `compile` in a single
  invocation. Supports JSON via file or stdin, `--keep-tex` /
  `--keep-logs` for debug artifacts, `--engine` override, `--output`
  path override, and `--force` for overwrite consent (spec 005).
- **Interactive menu** on all major commands when invoked without
  arguments in a TTY.
- **`.tex` preservation on compile failure** when `--keep-tex` is set
  (FR-15), enabling debug of broken renders.
- **Structured exit codes** — 10 (config missing), 15 (overwrite
  needs `--force`), 20 (template not found), 22 (templates dir
  missing), 30 (Tera error), 31 (malformed JSON), 40 (compile
  failed), 41 (engine not installed), 42 (engine binary missing).
- **`-v` / `-vv` verbosity flags** with per-stage timing.
- **Docker-based dev environment** at `docker/Dockerfile` pinning
  tectonic and Rust toolchain.
- Example templates and JSON data under `examples/`.

### Testing

- 208 tests across 12 integration test binaries plus library unit
  tests, exercising the full pipeline end-to-end inside the Docker
  container.

[Unreleased]: https://github.com/JonathanBufon/tex-cli/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/JonathanBufon/tex-cli/releases/tag/v0.1.0
