# Phase 1 — Quickstart: Translate All User-Facing Strings to English

**Feature**: `006-translate-to-english`
**Date**: 2026-07-18

Dev guide for landing spec 006. Covers the running order, migration
notes for callers who upgrade from `0.1.0`, and the smoke checks.

---

## Prerequisites

- Specs 001-005 shipped in `main` (they are — this is post-`v0.1.0`).
- Docker image `tex-cli` built (see the project `README.md` for the
  `docker build` command).
- Branch `006-translate-to-english` checked out and current with
  `main`.

---

## Implementation order

Follow the task order in `tasks.md`. In short:

1. **Translate `src/errors.rs`** first. Fix its inline tests
   immediately after — they must stay green as a self-check.
2. **Translate `src/cli.rs`** next (largest surface, biggest diff).
   Change `--format` default from `humano` to `human` here.
3. **Translate `src/interactive.rs`** — mechanical sweep.
4. **Translate `src/build.rs`, `src/render.rs`, `src/compiler.rs`**
   for the stdout/stderr strings not already in `cli.rs`.
5. **Update tests**, file by file, running `cargo test --lib` and
   the file's own `cargo test --test <name>` between passes.
6. **Update `README.md`** — sync the quoted stdout examples.
7. **Update `CHANGELOG.md`** under `[Unreleased] > Changed` with a
   `BREAKING:` prefix per D-07.
8. **Add the humano-rejection test** (`--format humano` → exit 2).
9. **Add the English-denylist smoke test** capturing `--help`.

Do **not** batch changes across files. One file at a time keeps the
diff reviewable and each `cargo test` blast-radius small.

---

## Local dev commands

```bash
# Full sanity check inside the container (canonical pre-commit gate).
docker run --rm -v $(pwd):/src -w /src tex-cli sh -lc \
  "cargo fmt --all -- --check && \
   cargo clippy --all-targets --all-features -- -D warnings && \
   cargo test --all"

# Fast library-only feedback loop (no tectonic invocation).
cargo test --lib

# Sweep audit — should return zero after each file's translation.
grep -rnE '"[^"]*[À-ÿ][^"]*"' src/<file>.rs

# --help visual check (rebuild first).
docker run --rm -v $(pwd):/src -w /src tex-cli \
  sh -lc "cargo build --release && ./target/release/tex-cli --help"

docker run --rm -v $(pwd):/src -w /src tex-cli \
  sh -lc "./target/release/tex-cli build --help"
```

---

## Smoke checks per file

After each file's translation task, run:

```bash
# 1. Format check.
cargo fmt --all -- --check

# 2. Lint check.
cargo clippy --all-targets --all-features -- -D warnings

# 3. Static Portuguese sweep on the file.
grep -nE '"[^"]*[À-ÿ][^"]*"' src/<file>.rs
# Expected: no matches inside string literals. Ignoring matches in
# `//` comments and inside test bodies is out of scope for this
# feature (see spec.md § Scope out).

# 4. Test the touched file (or the closest test binary).
cargo test --lib               # for src/*.rs changes
cargo test --test cli_<name>   # for tests/*.rs changes
```

---

## End-to-end smoke (post-implementation)

```bash
# Fresh temp home so the smoke doesn't touch a real config.
export HOME=/tmp/tex-006-smoke
mkdir -p $HOME/.config/tex /tmp/tex-006-smoke/{tpl,out}
export XDG_CONFIG_HOME=$HOME/.config

docker run --rm -v $(pwd):/src -w /src \
  -v /tmp/tex-006-smoke:/tmp/tex-006-smoke tex-cli \
  sh -lc "\
    cargo build --release && \
    export HOME=/tmp/tex-006-smoke && \
    export XDG_CONFIG_HOME=\$HOME/.config && \
    ./target/release/tex-cli init \
      --templates-dir /tmp/tex-006-smoke/tpl \
      --output-dir /tmp/tex-006-smoke/out \
      --engine tectonic --create-dirs --force && \
    ./target/release/tex-cli templates add examples/templates/artigo-basico.tex && \
    ./target/release/tex-cli build artigo-basico examples/data/artigo-basico.json"
```

Expected stdout (post-translation):

```text
PDF generated at /tmp/tex-006-smoke/out/artigo-basico.pdf. Pipeline (render + compile) took 2.3s.
```

Expected stderr banner (unchanged — banner is ASCII art, not natural
language):

```text
████████╗███████╗██╗  ██╗     ██████╗██╗     ██╗
...
```

---

## Migration notes for callers upgrading from `v0.1.0`

- **Scripts grepping stdout** for `PDF gerado` → change to `PDF generated`.
- **Scripts grepping stdout** for `Pipeline (render + compile) levou`
  → change to `Pipeline (render + compile) took`.
- **Scripts grepping stdout** for `Compilação levou` → change to
  `Compile took`.
- **Scripts passing `--format humano`** → change to `--format human`
  or omit the flag (`human` is the default).
- **Scripts asserting on error messages** — if pinning specific
  Portuguese tokens, re-pin to the English equivalent from
  [`contracts/cli.md`](./contracts/cli.md).
- **Exit codes are unchanged.** Scripts that check `$?` after
  `tex-cli` runs need no change.

---

## Rollback plan

If a critical issue is discovered post-merge:

1. **Cherry-pick revert** the merge commit of the spec-006 branch on
   `main`.
2. Users on `main` get the old Portuguese strings back.
3. Users who already installed the new binary can `cargo install`
   the previous version tag.

The refactor is text-only and isolated to string literals; a full
revert is safe and does not require data migration.

---

## Acceptance gate

Feature is done when:

- `docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all`
  → **all tests green** (~197 baseline; small delta acceptable per
  spec SC-001).
- `docker run --rm -v $(pwd):/src -w /src tex-cli sh -lc \
    "cargo fmt --all -- --check && cargo clippy --all-targets \
    --all-features -- -D warnings"` → **clean**.
- `tex-cli --help` and every subcommand's `--help` contain zero
  words from the Portuguese denylist (per SC-002).
- New `--format humano` rejection test passes.
- New `--help` English denylist smoke test passes.
- `README.md` no longer quotes any Portuguese success message.
- `CHANGELOG.md` has the `BREAKING:` entry under `[Unreleased]`.
- `ROADMAP.md` i18n blocker checkbox flipped to `[x]`.

---

## Cross-references

- [Spec](./spec.md)
- [Plan](./plan.md)
- [Research](./research.md)
- [Contracts](./contracts/cli.md)
- [Roadmap](../../ROADMAP.md)
