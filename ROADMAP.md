# Road to 1.0

This document tracks the work between the current state (post-spec-005,
pre-release) and a responsible **`v1.0.0`** cut.

Cutting `1.0` is a public commitment: from that point on, breaking
changes to the CLI surface (subcommand names, flag names, exit codes,
stdout/stderr contracts) require a major version bump. Everything on
this list either hardens that contract or clears blockers that would
otherwise force a `2.0` shortly after `1.0`.

## What "stable" means for `tex-cli`

The **stability contract at 1.0** covers:

- Every documented subcommand and flag in `README.md`.
- The full **exit-code table** in `README.md`.
- The stdout / stderr shapes documented in the `"Important contracts"`
  sections of each subcommand.
- The `~/.config/tex/config.toml` schema (key names and value types).

Not covered by the contract:

- Internal Rust library API (`tex_cli::…`) — the library is a public
  side-effect of `cargo` layout, not a supported surface. Unstable
  until further notice.
- Log output at `-v`, `-vv`, `-vvv` — human-readable, not machine-parsable.
- LaTeX engine behavior — pluggable and out of scope.

## Pre-1.0 blockers

These gate any responsible 1.0 cut. Order is roughly the recommended
sequence.

### Release hygiene

- [x] ~~Tag **`v0.1.0`**~~ — done 2026-07-18. Annotated tag at
      `d53839a` (last commit of spec 005), GitHub Release published:
      <https://github.com/JonathanBufon/tex-cli/releases/tag/v0.1.0>.
- [x] ~~Add a **versioning policy** section~~ — done 2026-07-18 as
      [`SEMVER.md`](SEMVER.md). Defines public surface, bump rules,
      deprecation policy, pre-1.0 behavior, MSRV stance, and the
      release process.
- [ ] Publish the crate to **crates.io** once — validates that
      `Cargo.toml` metadata is complete and reserves the name.

### API stability audit

- [ ] Freeze the CLI surface: review every subcommand, flag, and
      short-option letter. Rename anything ergonomic now, before users
      lock in muscle memory.
- [ ] Audit `pub` items in `src/lib.rs` and its submodules. Decide per
      item: **exposed on purpose** (stays public) or **incidental**
      (move behind `pub(crate)`).
- [ ] Confirm every exit code in `README.md` matches a `TexError`
      variant, and vice versa. No orphans in either direction.

### Internationalization decision

- [ ] Decide the i18n stance for stdout/stderr strings currently
      hardcoded in Portuguese (`PDF gerado em …`, `Pipeline (render +
      compile) levou X.Ys.`, `Compilação levou X.Ys.`). Three viable
      options:
  - **English-only** — translate the strings, keep it simple.
  - **PT + EN via `LANG`/`LC_MESSAGES`** — locale detection, two
    string tables.
  - **Full i18n** via `fluent` or similar — over-engineered for a
    single binary; defer.
- [ ] Whichever option wins, ship it **before 1.0**. Changing user-
      visible strings after 1.0 is a breaking change for anyone
      grepping stdout in a script.

### Real-world validation

- [ ] Run **5+ real documents** through the pipeline yourself
      (articles, resumés, letters, forms) and log any friction.
- [ ] Get at least **3 external testers** to run through the
      `quickstart.md` of each spec. Their bug reports likely surface
      the last-mile UX issues.
- [ ] Address findings via 0.x patch releases (`0.1.1`, `0.1.2`, …)
      until zero blockers remain.

## Recommended but not blocking

Nice-to-haves that make the 1.0 experience noticeably better. Ship
them if the calendar allows; skip them if not.

### Distribution

- [ ] **Homebrew** formula (`brew install tex-cli`)
- [ ] **Nix** flake / package
- [ ] **Pre-built binaries** in GitHub releases via
      [`cargo-dist`](https://github.com/axodotdev/cargo-dist) for
      Linux/macOS x86_64 + aarch64
- [ ] Debian/RPM packages — only if there's demand

### Testing & CI

- [ ] Cross-platform CI matrix: `ubuntu-latest`, `macos-latest`. Skip
      Windows unless there's demand — WSL2 is the documented path.
- [ ] Coverage report via `cargo-tarpaulin` or `cargo-llvm-cov`,
      published to Codecov or as an artifact.
- [ ] Wire each spec's `quickstart.md` into CI as an integration test
      so the docs cannot drift from behavior.

### Documentation

- [ ] Convert per-command sections in `README.md` into a small
      **mdBook** at `docs/book/` — better navigation for the long
      exit-code table and per-command contracts.
- [ ] Rustdoc for the (frozen) public library API.
- [ ] Screencast / GIF of the interactive menu flow.

### OSS hygiene

- [ ] `cargo-deny` in CI: license compatibility, RustSec advisories,
      duplicate deps.
- [ ] SBOM generation on release (via `cargo-sbom` or similar).

## Explicitly deferred (out of 1.0 scope)

Not blocking. Listed here so contributors know they don't need to
build these to reach 1.0.

- **`--watch` mode** — rebuild on template/JSON change. Useful, but
  additive; can ship in 1.1 without breaking anything.
- **Template scaffolding** (`tex-cli templates new`) — generate a
  starter `.tex` from a prompt.
- **Remote template catalog** — install templates from a URL.
- **PDF post-processing** — pdftk-style merge, split, watermark.
- **Non-LaTeX backends** — Typst, ConTeXt, etc. Explicitly out of
  scope per Constitution Principle I.

## Progress log

- **2026-07-18** — Roadmap created. All spec 001-005 shipped; docs
  ready for external contributors (CONTRIBUTING, CoC, SECURITY,
  CHANGELOG, PR/issue templates, CI, dual license).
- **2026-07-18** — `v0.1.0` tagged at `d53839a` and published as a
  GitHub Release.
