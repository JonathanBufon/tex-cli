# Contributing to tex-cli

Thanks for your interest in contributing! `tex-cli` is a Rust command-line
tool that turns JSON data into LaTeX documents and compiles them to PDF.

This guide covers the dev setup, coding conventions, and the workflow we
use to ship changes.

## Table of contents

- [Ways to contribute](#ways-to-contribute)
- [Learn Rust before contributing](#learn-rust-before-contributing)
- [Development environment](#development-environment)
- [Running tests](#running-tests)
- [Code style](#code-style)
- [Commit and branch conventions](#commit-and-branch-conventions)
- [Why Spec Kit?](#why-spec-kit)
- [Feature workflow (Spec Kit)](#feature-workflow-spec-kit)
- [Contributions with AI assistance](#contributions-with-ai-assistance)
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

## Learn Rust before contributing

Code contributions to `tex-cli` assume a working knowledge of Rust. If
you are new to the language, please read
**[The Rust Programming Language](https://doc.rust-lang.org/stable/book/)**
(a.k.a. "the Rust book") before opening a code PR. It's free, official,
and covers everything you need to work in this codebase.

The chapters most relevant to `tex-cli` are:

- **Chapters 1-9** — syntax, ownership, structs, enums, and pattern
  matching. Foundational for reading any file in `src/`.
- **Chapter 9 (Error Handling)** — this project relies on `Result<T, E>`
  and a custom `TexError` enum with structured exit codes. Understanding
  `?`, `From` conversions, and `thiserror` is essential.
- **Chapter 11 (Testing)** — all changes ship with tests. We use
  `#[test]`, `assert_cmd`, and `assert_fs` extensively.
- **Chapter 17 (Fearless Concurrency)** — not currently used in the
  runtime code, but useful context for RAII patterns like `TempDir`.

You do not need to be an expert — reviewers are happy to explain
idiomatic patterns during review. But please do not open a PR asking
what a `Result` or a lifetime is; those are covered in the book, and
answering them line-by-line in review is not a good use of anyone's
time.

Non-code contributions (docs, templates, bug reports, feature
requests) do not require Rust knowledge.

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

## Why Spec Kit?

`tex-cli` uses [Spec Kit](https://github.com/github/spec-kit) as its
default feature-development workflow. This is an opinionated choice —
here's the rationale.

**It's currently the best automated way to pair-program with an LLM.**
Model-agnostic: Claude Code, Codex, GitHub Copilot, Cursor, and other
agents all handle the `spec → plan → tasks → implement` pipeline well
because it produces durable, structured artifacts (`spec.md`, `plan.md`,
`research.md`, `contracts/`, `tasks.md`) that the agent can re-read at
any point. The workflow forces the model to write its own context down
before writing code.

**It mitigates the AI amnesia problem.** LLMs lose context across
sessions and, on longer tasks, drift within a single session. Spec Kit
does not eliminate this — nothing does today — but the artifact chain
resolves the majority of it: when a session gets compacted, cleared, or
resumed weeks later, the agent (or a different agent, or a human) can
pick up from the specs directory instead of guessing intent from code.

**It's also useful without any AI.** The same discipline — write down
what and why, then how, then a task list — catches bad architecture
before it becomes code. Even a solo human contributor gets clearer,
more reviewable PRs out of it.

**When you shouldn't use it.** Bug fixes, small refactors, docs
changes, and typo hunts don't need a spec. Use judgment: if the change
touches multiple modules or introduces new user-facing behavior, it's
worth the ~15 minutes to draft the spec first.

## Feature workflow (Spec Kit)

Non-trivial features go through the [Spec Kit](https://github.com/github/spec-kit)
workflow, which lives under `specs/NNN-feature-name/`. See the
[Spec Kit documentation](https://github.com/github/spec-kit#readme) for
a full reference — the summary below is enough to get started.

1. **`speckit-specify`** — write `spec.md` (what and why, no
   implementation details).
2. **`speckit-plan`** — write `plan.md`, `research.md`, `data-model.md`,
   `contracts/`, `quickstart.md` (how you'll build it).
3. **`speckit-tasks`** — generate `tasks.md` (an ordered, testable
   task list, one PR-sized chunk each).
4. **`speckit-implement`** — execute the tasks, committing incrementally.

See [Why Spec Kit?](#why-spec-kit) above for when this workflow is
worth the setup cost and when to skip it.

### Example: end-to-end flow

Adding a hypothetical `--watch` mode to `build`, with an AI coding
assistant (Claude Code, Codex, Copilot, or similar):

```bash
# 1. Draft the spec (what + why, not how)
/speckit-specify Add --watch mode to `build` that rebuilds the PDF
                 whenever the template or JSON changes on disk.
# → creates specs/006-watch-mode/spec.md
#   review it, tighten the language, commit

# 2. Turn the spec into a plan
/speckit-plan
# → creates plan.md, research.md, data-model.md, contracts/, quickstart.md
#   review each artifact — this is where you catch bad architecture
#   before it becomes code

# 3. Break the plan into tasks
/speckit-tasks
# → creates tasks.md with T001..TNN, ordered by dependency

# 4. Execute — the agent works through tasks, committing per task
/speckit-implement
# → each task becomes 1-2 commits; you review the diff between tasks

# 5. Run the full suite locally before opening the PR
docker run --rm -v $(pwd):/src -w /src tex-cli \
  sh -lc "cargo fmt --all -- --check && \
          cargo clippy --all-targets --all-features -- -D warnings && \
          cargo test --all"
```

Every past feature in `specs/001-…/` through `specs/005-…/` was built
this way — read those directories if you want a concrete reference.

## Contributions with AI assistance

`tex-cli` welcomes contributions written with AI coding assistants
(Claude Code, Codex, GitHub Copilot, Cursor, etc.). AI-generated code is
not treated differently at review time, but the failure modes are
different from human-written code, and contributors are responsible for
catching them before the PR lands.

### Recommended workflow

- **Use Spec Kit for anything non-trivial.** The `spec → plan → tasks →
  implement` chain gives the agent structured context that reduces
  hallucination and keeps the change reviewable. Feeding "add a watch
  mode" straight into `/speckit-implement` without a spec produces
  worse output than the four-step pipeline.
- **Read every diff.** Do not commit code you have not read. LLMs
  produce confident-looking output that can be subtly wrong.
- **Commit incrementally.** One PR = one feature; one commit = one
  coherent change. Do not push a single 2000-line "AI made it work"
  commit — it is not reviewable and will be sent back.

### What to verify before opening the PR

- **Tests actually test the behavior they claim.** A common LLM
  failure mode is a test that passes vacuously (e.g. asserts on a
  hard-coded string it just wrote) without exercising the code path.
  Read each assertion.
- **APIs and crate features exist.** LLMs hallucinate function
  signatures, feature flags, and trait bounds. Cross-check anything
  unfamiliar against [docs.rs](https://docs.rs/) or `cargo doc --open`.
- **Error paths propagate `TexError`** with the correct exit code
  (see `src/error.rs`). LLMs sometimes invent new error types or
  swallow errors silently.
- **No new dependencies** slipped in. Run `git diff Cargo.toml
  Cargo.lock` before pushing.
- **License-clean.** If the assistant produced a large block that
  looks copied from a known source, do not commit it. Refactor from
  first principles or leave it out.

### Disclosure in the PR

You do not have to disclose that an assistant was involved, but if it
did substantial work on the change, we appreciate a short note in the
PR body (e.g., "Drafted with Claude Code, all commits reviewed and
tests re-run manually"). This helps reviewers weigh what to look at
more carefully.

For commit attribution, the assistants that support it typically add
their own `Co-Authored-By:` trailer — that is fine and welcomed, but
never required.

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

`tex-cli` is dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([`LICENSE-MIT`](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.
