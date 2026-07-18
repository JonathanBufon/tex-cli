# Versioning policy

`tex-cli` follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).
This document defines what the version numbers mean **in practice** for
this project — what surface is covered by the stability guarantee, what
changes force each kind of bump, and how deprecations are handled.

## Table of contents

- [Public surface](#public-surface)
- [Not covered by the stability guarantee](#not-covered-by-the-stability-guarantee)
- [Version bumps](#version-bumps)
- [Deprecation policy](#deprecation-policy)
- [Pre-1.0 behavior](#pre-10-behavior)
- [Rust MSRV](#rust-msrv)
- [Release process](#release-process)

## Public surface

The following are **covered** by the stability guarantee once `1.0.0` is
released. A backwards-incompatible change to any of these requires a
major version bump.

- **Subcommands**: `init`, `config show`, `config set`, `templates list`,
  `templates show`, `templates add`, `templates remove`, `render`,
  `compile`, `build`.
- **Flags and short options** on every subcommand. Adding a new flag
  is a minor bump; renaming or removing a flag is a major bump.
- **Exit codes** documented in [`README.md`](README.md#exit-codes).
  Introducing a new code is a minor bump; changing what an existing
  code means is a major bump.
- **Stdout / stderr shape** documented in the "Important contracts"
  sections of `README.md`. Adding to a message is a minor bump;
  changing the format that scripts parse (path, duration, machine-
  readable JSON) is a major bump.
- **Config schema** in `~/.config/tex/config.toml`: the set of
  accepted dotted-path keys and their value types. Adding a key with
  a safe default is a minor bump; renaming or removing a key is a
  major bump.

## Not covered by the stability guarantee

The following can change in any release without a major bump.

- **Internal Rust library API** (`tex_cli::*` in `src/lib.rs`). The
  `[lib]` target exists so the integration tests can share code with
  the binary. It is **not a supported downstream API**.
- **Verbose log output** at `-v`, `-vv`, `-vvv`. These are for
  humans, not machines.
- **Interactive prompt wording**. The prompts in the `tex-cli` menu
  can change to improve UX.
- **LaTeX engine behavior**. Pluggable engines (`tectonic`,
  `latexmk`, `pdflatex`, `xelatex`, `lualatex`) are third-party
  binaries; their behavior is out of scope.
- **Default engine choice**. If a better engine appears, we may
  change the default.
- **Docker image contents** at `docker/Dockerfile`. Reproducibility
  is best-effort, not a contract.

## Version bumps

Given a version `MAJOR.MINOR.PATCH`:

- **MAJOR** — bumped for any breaking change to the public surface
  above. Examples: removing a subcommand, renaming a flag, changing
  an exit code's meaning, changing a stdout format that scripts
  parse, breaking the config schema.
- **MINOR** — bumped for backwards-compatible additions: new
  subcommand, new flag, new exit code, new config key with a safe
  default, new engine support.
- **PATCH** — bumped for bug fixes, doc fixes, and internal
  refactors that do not change the public surface. Includes
  performance improvements, dependency updates, and CI changes.

## Deprecation policy

Once a release goes out, features are not removed without warning.

1. **Announce**: A feature marked for removal gets a deprecation
   notice in `CHANGELOG.md` and, where feasible, a runtime warning
   to stderr when the feature is used.
2. **Keep working**: The deprecated feature keeps working through
   at least the next minor release cycle.
3. **Remove**: Removal happens only in a subsequent major release,
   at least one minor version after the deprecation was announced.

The intent: nobody upgrading within a major series ever gets their
scripts silently broken.

## Pre-1.0 behavior

While the version is `0.x.y`, the stability guarantee **does not
apply**. Any release may include breaking changes to any part of the
public surface. This is standard SemVer behavior for `0.x`.

In practice, we still try to be conservative — changelog entries
call out breaks explicitly and provide migration hints — but do not
rely on `0.x` stability for automation.

The move to `1.0` is documented in [`ROADMAP.md`](ROADMAP.md).

## Rust MSRV

`tex-cli` targets **Rust stable, edition 2021**. There is no minimum
supported Rust version (MSRV) commitment lower than "latest stable at
the time of release". Bumping the required Rust version is a **minor
bump**, not a major bump, because this project is a binary
(installed via `cargo install` or a prebuilt release, not linked as
a library).

## Release process

1. Land all changes to `main` via the normal PR flow.
2. Update `CHANGELOG.md`: move items from `[Unreleased]` into a new
   version section with the release date.
3. Bump `version` in `Cargo.toml`.
4. Land the changelog + version bump as a single "Prepare vX.Y.Z"
   PR.
5. Tag the merge commit: `git tag -a vX.Y.Z -m "…"`.
6. Push the tag: `git push origin vX.Y.Z`.
7. Publish a GitHub Release from the tag, with the changelog entry
   as the body.
8. (Optional) `cargo publish` to crates.io.

CI must be green before tagging.
