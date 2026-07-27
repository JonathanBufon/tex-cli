# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
