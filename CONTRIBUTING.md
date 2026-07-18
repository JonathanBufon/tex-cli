# Contributing to tex-cli

Thanks for your interest in contributing! `tex-cli` is a Rust command-line
tool that turns JSON data into LaTeX documents and compiles them to PDF.

This guide covers the dev setup, coding conventions, and the workflow we
use to ship changes.

## Table of contents

- [Ways to contribute](#ways-to-contribute)
- [Development environment](#development-environment)
- [Running tests](#running-tests)
- [Code style](#code-style)
- [Commit and branch conventions](#commit-and-branch-conventions)
- [Feature workflow (Spec Kit)](#feature-workflow-spec-kit)
- [Pull request process](#pull-request-process)
- [Reporting bugs and requesting features](#reporting-bugs-and-requesting-features)

## Ways to contribute

- **Bugs**: open an issue using the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml).
- **Features**: open a [feature request](.github/ISSUE_TEMPLATE/feature_request.yml)
  and let's align on scope before you write code.
- **Documentation**: PRs that fix typos, clarify examples, or improve the
  README are always welcome and don't need prior discussion.
- **Templates**: contributions to `examples/templates/` are appreciated —
  they help new users see what the tool can do.

## Development environment

`tex-cli` uses **tectonic** as its default LaTeX engine. The canonical
dev environment is Docker, which pins the LaTeX toolchain and avoids
"works on my machine" problems.

### Requirements

- **Rust stable** (edition 2021) — install via [rustup](https://rustup.rs)
- **Docker** — for running the LaTeX toolchain in a reproducible container
- A POSIX shell (Linux, macOS, or WSL2 on Windows)

### Build the container

```bash
docker build -t tex-cli -f docker/Dockerfile .
```

This installs Rust + tectonic + the required system libraries. The
resulting image is around 1.6 GB and can be reused across sessions.

### Cargo check without Docker

For fast iteration on non-LaTeX code (config parsing, CLI wiring, error
handling), you can run `cargo check` and `cargo test --lib` directly on
the host — no Docker needed. Integration tests that produce a PDF
require tectonic and should be run inside the container.

## Running tests

Run the full suite (unit + integration) inside the container:

```bash
docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all
```

Expected: ~200 tests passing. The suite takes roughly 2 minutes because
several integration tests compile real `.tex` documents with tectonic.

Fast feedback loop for library code only:

```bash
cargo test --lib
```

To pull a generated PDF out of the container, use `docker cp` rather
than installing tectonic on the host.

## Code style

We enforce three checks on every PR:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Formatting is `rustfmt` defaults. Clippy runs with `-D warnings`, so
any lint is a blocker — fix it or add a targeted `#[allow(...)]` with a
short justification.

Additional conventions:

- **No new dependencies** without discussion. The project deliberately
  keeps its dependency tree small.
- **Comments explain *why*, not *what***. Names should carry meaning;
  comments cover hidden constraints or non-obvious tradeoffs.
- **Errors are propagated, not swallowed**. Use `TexError` and existing
  exit code contracts (see `src/error.rs`) rather than inventing new ones.

## Commit and branch conventions

**Branches** follow the Spec Kit pattern: `NNN-short-name` for feature
work (e.g., `005-build-pipeline-impl`), and `chore/…`, `fix/…`, or
`docs/…` for standalone changes.

**Commits** use conventional-commit prefixes and reference the task ID
when applicable:

```text
feat(T005-T011): build module + handle_build MVP
test(T004): tests/cli_build.rs — 13 integration tests for US1
chore(T001): verify baseline after spec 004 merge
docs: clarify --keep-tex behavior in README
```

Keep commits focused. Prefer several small commits over one large one
when the changes are logically distinct.

## Feature workflow (Spec Kit)

Non-trivial features go through the [Spec Kit](https://github.com/github/spec-kit)
workflow, which lives under `specs/NNN-feature-name/`:

1. **`speckit-specify`** — write `spec.md` (what and why, no
   implementation details).
2. **`speckit-plan`** — write `plan.md`, `research.md`, `data-model.md`,
   `contracts/`, `quickstart.md` (how you'll build it).
3. **`speckit-tasks`** — generate `tasks.md` (an ordered, testable
   task list, one PR-sized chunk each).
4. **`speckit-implement`** — execute the tasks, committing incrementally.

You don't need to use Spec Kit for bug fixes, small refactors, or docs
changes. Use judgment: if the change touches multiple modules or
introduces a new user-facing behavior, a spec is worth writing.

## Pull request process

1. **Fork** the repo and create a branch from `main`.
2. Make your changes and ensure all three checks pass locally
   (`fmt`, `clippy`, `cargo test --all` in Docker).
3. **Open a PR** using the [PR template](.github/PULL_REQUEST_TEMPLATE.md).
   Fill in the summary, test plan, and constitution alignment sections.
4. Link to any related issue or spec (e.g., `Closes #12`,
   `Implements specs/005-build-pipeline`).
5. A maintainer will review. Address feedback with additional commits
   rather than force-pushing — this keeps review threads coherent.
6. Once approved, the maintainer will **rebase-merge** the PR to keep
   `main`'s history linear.

### What we look for in reviews

- Tests cover the new behavior — including error paths and exit codes.
- Public API surface stays small; if you add a new function, it needs
  a clear purpose and doc comment.
- Changes are constitution-aligned (see `.specify/memory/constitution.md`).
  The five principles are:
  1. Personal scope — not a LaTeX replacement.
  2. Separation of data, template, and PDF layers.
  3. Dual interface (CLI flags + interactive prompts).
  4. Pluggable compiler engine.
  5. Safe file handling (atomic writes, explicit overwrite consent).

## Reporting bugs and requesting features

- **Bugs**: use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml)
  and include your OS, `tex-cli --version`, the exact command that
  failed, and the full error output.
- **Security issues**: **do not open a public issue**. See
  [SECURITY.md](SECURITY.md) for the private disclosure process.
- **Feature requests**: use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.yml).
  Explain the use case, not just the mechanism.

## Code of conduct

Participation in this project is governed by the
[Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By
contributing, you agree to abide by its terms.

## License

By contributing, you agree that your contributions will be licensed
under the [MIT License](LICENSE).
