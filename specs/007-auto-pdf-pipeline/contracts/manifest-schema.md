# Contract — Template Package Manifest (`tex-template.toml`)

**Feature**: 007-auto-pdf-pipeline
**Source**: FR-019, research R4.

Every third-party template package MUST include a manifest at `<package-root>/tex-template.toml`. The manifest is the single source of truth for the identifier, version, and entrypoint the tool uses when it installs, trusts, or invokes the template.

## Required fields

| Field        | Type    | Format / regex                                                          | Example              |
|--------------|---------|-------------------------------------------------------------------------|----------------------|
| `identifier` | string  | `^[a-z0-9][a-z0-9-]*\/[a-z0-9][a-z0-9._-]*$`                            | `"acme/invoice"`     |
| `version`    | string  | `MAJOR.MINOR.PATCH` optionally followed by `-<pre>` and/or `+<build>`   | `"1.2.0"`, `"0.1.0-beta+ci"` |
| `entrypoint` | string  | POSIX relative path within the package root, must end in `.tex`         | `"template.tex"`     |

## Optional / unknown fields

Any additional top-level TOML key is **preserved but ignored** in v1. The install command MUST NOT reject a manifest solely because it contains unknown keys — this is required so future manifest revisions do not break older tool versions.

## Reserved for future use (not enforced in v1)

The following field names are reserved; using them today has no effect but will be interpreted in a future spec revision:

- `description` (string, human-readable)
- `author` (string or table)
- `homepage` (URL)
- `min_tex_cli` (semver requirement string)

## Full example

```toml
# Third-party template published on GitHub.

identifier = "acme/invoice"
version    = "1.2.0"
entrypoint = "template.tex"

# Reserved fields — safe to include; ignored by v1.
description = "Consultancy-style invoice with per-line VAT"
author      = "Acme Templating Co."
```

## Validation errors (`ManifestError`)

| Case                                              | Error variant                       |
|---------------------------------------------------|-------------------------------------|
| No `tex-template.toml` at package root            | `NotFound`                          |
| Invalid TOML                                      | `Parse { source }`                  |
| Missing `identifier` / `version` / `entrypoint`   | `MissingField { field }`            |
| `identifier` fails regex                          | `InvalidIdentifier { value }`       |
| `version` fails parse                             | `InvalidVersion { value }`          |
| `entrypoint` escapes package root                 | `EntrypointEscape`                  |
| `entrypoint` file missing on disk                 | `EntrypointMissing { path }`        |
| `entrypoint` does not end in `.tex`               | `EntrypointNotTex { path }`         |

Every variant carries enough context to be printed directly to the user in FR-014-style messages (phase = install; corrective action explicit).
