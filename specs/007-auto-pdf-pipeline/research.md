# Phase 0 Research — Auto PDF Pipeline

**Feature**: 007-auto-pdf-pipeline
**Date**: 2026-08-03
**Purpose**: Resolve every open technical question raised by [plan.md](./plan.md) so Phase 1 design has no unknowns.

## R1 — Tectonic sandbox composition for third-party templates

**Question**: FR-017 requires that third-party template compiles run with `\write18` disabled, no network access, and no filesystem writes outside the resolved output directory. What is the concrete flag/env combination inside `compiler.rs` that delivers those three guarantees on top of Tectonic?

**Decision**: Compose three enforcement layers for third-party compiles:

1. **`\write18` off**: never pass `--shell-escape` (or any equivalent flag) when invoking Tectonic for a third-party template. Tectonic's default is `\write18` disabled, so this is an explicit *do-not-opt-in* rule inside `compiler.rs`.
2. **Filesystem write scope**: set the environment variable `openout_any=p` (KPathsea paranoid mode) on the Tectonic child process. In paranoid mode LaTeX may only `\openout` files under the current working directory; attempts to escape emit a KPathsea error surfaced to the user as `SandboxError::FilesystemEscape`.
3. **No network**: point Tectonic at a locally cached bundle via `--only-cached` (Tectonic downloads a bundle on first run; after that `--only-cached` prevents any fetch). If the bundle cache is absent when a third-party compile is requested, `compiler.rs` refuses to compile and returns `SandboxError::BundleMissing` telling the user to run `tex-cli init` or a first built-in compile to populate the cache.

Built-in library templates skip all three layers (they can rely on Tectonic's ambient defaults). The distinction is a single boolean parameter (`sandboxed: bool`) threaded through `compiler.rs::compile()`.

**Rationale**:
- Tectonic already uses KPathsea for path resolution, so `openout_any=p` is honored without any Tectonic-specific patch.
- `--only-cached` is a first-class Tectonic flag; no wrapper needed.
- No new crate, no `unshare`/`firejail`/namespace call — the constitution's fixed stack is preserved. Docker container (per Assumption A-01) provides one outer layer of isolation; these three flags provide the inner layer for template-specific mistrust.

**Alternatives considered**:
- **Namespace-based sandbox (unshare, seccomp)**: portable to Linux only, requires unsafe FFI or a new crate (`nix`), and duplicates protection Docker already gives. Rejected as over-engineering for v1.
- **Per-compile Docker container**: latency cost of spinning a container per compile violates SC-001 (< 60 s first-run including setup) for realistic multi-invocation workflows. Rejected.
- **`--shell-escape=restricted`**: still allows a whitelist of shell commands; too permissive for the "assume adversarial template" posture. Rejected.

## R2 — Git URL install: shell-out vs `git2` crate

**Question**: FR-015 says the install command must accept a Git URL. How does `install.rs` actually clone the repo without violating the constitution's fixed dependency stack?

**Decision**: Shell out to the system `git` binary via `which::which("git")` + `std::process::Command::new(git).args(["clone", url, dest]).status()`. If `git` is absent, produce `InstallError::GitBinaryMissing` with an install instruction (Debian: `apt install git`; macOS: `brew install git`).

**Rationale**:
- **No new crate**: the constitution fixes the dependency stack; `git2` is not in the list.
- **Consistency with existing pattern**: the tool already resolves `tectonic` via `which` (see `compiler.rs`). Using the same pattern for `git` keeps the two external tools symmetric — same error class, same discovery.
- **Runtime dep is acceptable**: `git` is a near-universal presence on developer machines; the failure mode (missing binary) is transparent and actionable.

**Alternatives considered**:
- **`git2` crate**: libgit2 bindings, no runtime `git` dependency. Rejected because it violates the fixed stack and because libgit2 has a larger CVE surface than delegating to `git`.
- **Support only tarball URLs (`.tar.gz`)**: no git needed; user could package a template. Rejected because most template authors will publish on GitHub/GitLab, and requiring a tarball step is friction with no upside.
- **Pure HTTP fetch of a single manifest URL**: too restrictive for multi-file templates. Rejected.

## R3 — Trust record storage: file location and format

**Question**: FR-016 requires per-(template, version) approval to persist across runs. Where does the trust record live and in what format?

**Decision**: Single TOML file at `~/.local/share/tex/trust.toml` (resolved via `dirs::data_local_dir().join("tex/trust.toml")`). One array-of-tables entry per approved identifier@version.

Example:

```toml
schema_version = 1

[[trust]]
identifier = "acme/invoice"
version    = "1.0.0"
approved_at = "2026-08-03T14:30:00Z"

[[trust]]
identifier = "acme/invoice"
version    = "1.1.0"
approved_at = "2026-08-04T09:12:00Z"
```

Writes go through the existing `atomic.rs` (write-temp + rename) so a crashed install cannot leave the file half-written.

**Rationale**:
- **TOML** is in the fixed stack (`toml` crate); no new dependency.
- **Single file** keeps concurrent-safe rewriting trivial (whole-file atomic replace) — installed templates rarely exceed hundreds of entries, so file size is bounded.
- **`~/.local/share/tex/`** is the XDG-standard data location on Linux (via `dirs`); separates trust state from user-editable config in `~/.config/tex/`.
- **`schema_version`** field allows future migration without ambiguity.
- **`approved_at`** ISO-8601 timestamp gives future audit tooling something to hang on, at zero implementation cost.

**Alternatives considered**:
- **Sqlite via `rusqlite`**: overkill for expected corpus size (< 1000 entries); adds C dependency; violates fixed stack. Rejected.
- **One file per trust record** (`~/.local/share/tex/trust/<hash>.toml`): more parallel-safe but adds directory-scan cost to every compile. Rejected.
- **Store trust inside `config.toml`**: mixes user preferences with security-critical state. Rejected on principle-V grounds.

## R4 — Template package manifest: filename and schema

**Question**: FR-019 requires three fields (`identifier`, `version`, `entrypoint`). What is the manifest filename, format, and precise field validation?

**Decision**: Manifest lives at `<package-root>/tex-template.toml` in the template package. Schema:

```toml
identifier = "acme/invoice"   # required — must match /^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9._-]*$/
version    = "1.0.0"           # required — must parse as semver (major.minor.patch, optional -pre / +build)
entrypoint = "template.tex"    # required — relative path within the package, must exist, must end in .tex
```

Install-time validation rejects the package on:
- Missing `tex-template.toml`.
- Missing any required field.
- `identifier` failing the regex (must be `namespace/name` with allowed charset; namespace non-empty).
- `version` failing semver parse.
- `entrypoint` pointing outside the package (must remain within `<package-root>` after path normalization), missing on disk, or not ending in `.tex`.

Unknown TOML keys are preserved but ignored (per FR-019 "unknown fields MUST be preserved but ignored").

**Rationale**:
- **`tex-template.toml`** filename is explicit and avoids collision with `tectonic.toml` (Tectonic's own project file), `Cargo.toml`, or a generic `template.toml`.
- **TOML** is in the fixed stack.
- **Semver** for `version` gives FR-016's re-prompt-on-upgrade logic a clean comparison rule (a hand-rolled ordering would be a footgun); we can validate with `serde` + a lightweight version parser without adding a `semver` crate — a small internal parser is trivial for the three-segment case and does not require a new dep.
- **Identifier regex** matches common package-naming conventions (crates.io, npm scoped packages) and forces lowercase / URL-safe characters, avoiding filesystem-portability issues.

**Alternatives considered**:
- **`Cargo.toml`-inspired `[package]` section**: extra nesting for no value at three fields. Rejected.
- **JSON manifest**: the tool already uses TOML for its own config; consistency wins. Rejected.
- **Adding the `semver` crate**: would violate the fixed stack for a 30-line parser. Rejected.
- **Allowing `entrypoint` outside `<package-root>`**: expands attack surface (symlink escape). Rejected.

## R5 — Installed templates on-disk layout

**Question**: Where and how are installed template packages laid out on disk?

**Decision**:

```text
~/.local/share/tex/
├── trust.toml
└── templates/
    ├── acme/
    │   └── invoice/
    │       ├── tex-template.toml
    │       ├── template.tex          # entrypoint per manifest
    │       └── (any supporting files)
    └── myorg/
        └── report/
            ├── tex-template.toml
            └── template.tex
```

Rules:
- Root resolved once per invocation as `dirs::data_local_dir().join("tex/templates")`.
- Install destination for `identifier = "namespace/name"` is `<root>/<namespace>/<name>/`.
- Directory MUST match the manifest identifier's `namespace/name` after install; a mismatch (e.g., user renamed the checkout) triggers `InstallError::PathMismatch`.
- Existing `<root>/<namespace>/<name>/` requires `--force` to overwrite (Constitution V compliance).
- Removal (`tex-cli template remove <identifier>`) deletes the whole `<namespace>/<name>/` directory and drops all matching trust records from `trust.toml`.

**Rationale**:
- **Two-level directory** (namespace, name) matches the identifier grammar exactly, so `discovery.rs` can `readdir` twice to enumerate installed templates — no index file needed.
- **Sibling to `trust.toml`** keeps all runtime state under one XDG data root.
- **`--force` requirement** aligns with Constitution V's "never overwrite without explicit consent".

**Alternatives considered**:
- **Flat directory keyed by `namespace_name`**: readable but hides the namespace hierarchy from `ls`; loses grouping. Rejected.
- **Content-addressed layout** (`<root>/<hash-of-manifest>/`): breaks `tex-cli template list` UX and complicates re-install. Rejected.
- **Templates directory next to config** (`~/.config/tex/templates/`): mixes user-authored config with tool-managed state. Rejected on XDG grounds.

## Cross-cutting notes

- **First-run of a third-party compile requires a warm Tectonic bundle cache** (R1). The natural fix is: on `tex-cli init`, warm the cache by running a no-op compile against a bundled fixture. That decision belongs in Phase 2 (task granularity), noted here so it isn't lost.
- **All new modules use `thiserror`-based error enums**; boundary errors bubble up as `anyhow::Error` in `main.rs` per Constitution V. No changes to the existing error-handling pattern.
- **Windows portability**: `openout_any=p` behaviour is KPathsea-provided and works cross-platform; `dirs::data_local_dir()` returns the correct platform-native path; `which` and `Command` are cross-platform. No Linux-specific syscalls introduced.

## Summary — no unresolved items

Every question raised by plan.md's Technical Context is resolved. Phase 1 (data-model + contracts + quickstart) can proceed with no `NEEDS CLARIFICATION` markers remaining.
