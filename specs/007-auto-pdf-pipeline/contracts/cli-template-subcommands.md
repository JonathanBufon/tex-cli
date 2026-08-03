# CLI contract — `tex-cli template` subcommand tree

**Feature**: 007-auto-pdf-pipeline
**Constitution alignment**: III (dual CLI + interactive interface), V (safe file ops).

All subcommands MUST be exposed under `clap` in the exact shape below. Exit codes are drawn from `errors.rs`'s existing scheme (0 = success, 2 = input error, 3 = env error, 4 = render error, 5 = compile error) with new additions annotated.

## `tex-cli template install <source> [--force]`

Install a third-party template package.

- **`<source>`** (positional, required):
  - A **Git URL** (recognized by `://` or `git@` prefix, or `.git` suffix) — cloned via `git` binary.
  - A **local filesystem path** — copied recursively.
- **`--force`** (flag, optional): overwrite an existing installed template at the same `identifier@version`. Without it, an existing install → exit code **2** (`InstallError::Overwrite`).

**Behaviour**:
1. Resolve `<source>` to a temp directory (`tempfile::TempDir`).
2. Load `tex-template.toml`; validate all three required fields (see [manifest-schema.md](./manifest-schema.md)).
3. Derive destination `~/.local/share/tex/templates/<namespace>/<name>/`.
4. If dest exists and `--force` absent → error.
5. Move (rename or copy+remove) into destination atomically.
6. Print, on `stdout`: `installed <identifier>@<version> at <path>` (matches Constitution III scriptable-output rule).

**Exit codes**:
- 0 — success.
- 2 — invalid source, invalid manifest, or overwrite without `--force`.
- 3 — `git` binary missing, filesystem I/O failure.

**Interactive counterpart** (Constitution III): menu entry "Install a template" prompts for source via `inquire::Text`, then runs the same logic.

## `tex-cli template list [--json]`

Enumerate installed templates.

- **`--json`** (flag, optional): emit structured JSON to stdout for scripting.

**Default output** (human):

```text
IDENTIFIER          VERSION   PATH
acme/invoice        1.1.0     /home/user/.local/share/tex/templates/acme/invoice
myorg/report        0.2.3     /home/user/.local/share/tex/templates/myorg/report
```

**`--json` output**:

```json
[
  {"identifier": "acme/invoice", "version": "1.1.0", "path": "/home/…/acme/invoice", "trusted": true},
  {"identifier": "myorg/report", "version": "0.2.3", "path": "/home/…/myorg/report", "trusted": false}
]
```

Built-in library templates MAY be included with `--include-builtin` (flag). By default `list` shows only installed third-party packages — built-ins are enumerable via `tex-cli templates` (existing command, unchanged).

**Exit codes**: 0 on success, 3 on I/O failure.

## `tex-cli template remove <identifier> [--yes]`

Uninstall a template by identifier.

- **`<identifier>`** (positional, required): full `namespace/name` of an installed template.
- **`--yes`** (flag, optional): skip the interactive confirmation.

**Behaviour**:
1. Look up `<identifier>` in installed directory.
2. Print the destination path and prompt via `inquire::Confirm` unless `--yes` is set (Constitution V: never delete without explicit consent).
3. Delete the directory recursively.
4. Drop every matching trust record from `trust.toml` via `TrustFile::revoke`.
5. Print `removed <identifier>` on `stdout`.

**Exit codes**: 0 on success, 2 on unknown identifier, 3 on I/O failure.

## `tex-cli template trust <identifier> [--version <ver>] [--revoke]`

Manage trust records without going through a compile.

- **`<identifier>`** (positional, required).
- **`--version <ver>`** (optional): a specific version to trust or revoke. If omitted, all installed versions of that identifier are affected.
- **`--revoke`** (flag): revoke instead of grant.

**Behaviour**:
- Grant: prompt the user with `inquire::Confirm` describing what is about to be trusted; on yes, append record(s) and write `trust.toml` atomically. On `--force` (future) or `--yes`, skip prompt.
- Revoke: remove matching records; no prompt (revocation is safe).

**Exit codes**: 0 on success, 2 on unknown identifier.

## Interaction with existing commands

- `tex-cli templates` (existing) continues to list only built-in library templates. Deprecating or extending it is out of scope in v1; the new `template list` covers installed packages.
- `tex-cli build --json <path>` (see [cli-build-pipeline.md](./cli-build-pipeline.md)) reuses the trust prompt path — running `build` on JSON that resolves to an un-trusted installed template will trigger the same `inquire::Confirm` flow as `template trust`.
