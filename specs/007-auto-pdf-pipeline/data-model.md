# Phase 1 Data Model — Auto PDF Pipeline

**Feature**: 007-auto-pdf-pipeline
**Date**: 2026-08-03

Entities are grouped by lifecycle. Every entity below maps to a Rust type living in the module identified in its **Home** line. Field lists show the on-disk / in-memory schema; validation rules mirror the FRs in [spec.md](./spec.md).

## Identifier (value object)

**Home**: `discovery.rs` (module-local `pub` type).

```rust
pub struct Identifier {
    pub namespace: Option<String>, // None for built-in, Some for third-party
    pub name: String,
}
```

**Validation**:
- `name` must match `^[a-z0-9][a-z0-9._-]*$`.
- `namespace` (if present) must match `^[a-z0-9][a-z0-9-]*$`; empty namespace is rejected at parse time.
- `Display` renders as `name` for built-in or `namespace/name` for third-party.

**Where used**: threaded through Installed Template, Trust Record, Manifest, and Pipeline Report.

**Source**: FR-018.

## Template Package Manifest

**Home**: `templates.rs`.

```rust
#[derive(Deserialize)]
pub struct Manifest {
    pub identifier: String,      // parsed into Identifier after load
    pub version: Version,        // custom parsed type (see below)
    pub entrypoint: PathBuf,     // relative; validated
    #[serde(flatten)]
    pub extra: toml::Table,      // preserved but ignored
}

pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<String>,
    pub build: Option<String>,
}
```

**Validation** (all enforced by `Manifest::load(path)`):
- File name must be exactly `tex-template.toml` at the package root.
- Missing file → `ManifestError::NotFound`.
- Missing required field → `ManifestError::MissingField { field }`.
- `identifier` must parse as a namespaced `Identifier` (namespace required, not None).
- `version` must parse per R4 (`major.minor.patch`, optional `-pre`, optional `+build`).
- `entrypoint` must:
  - Be relative (no leading `/`, no `..` after normalization).
  - Resolve to a file that exists inside the package root.
  - End with `.tex`.
- Unknown keys are stored in `extra`, never surfaced to callers by default.

**Source**: FR-019, R4.

## Installed Template

**Home**: represented by directory layout (no in-memory record persisted); enumerated on demand.

```text
~/.local/share/tex/templates/<namespace>/<name>/
├── tex-template.toml
├── <entrypoint>.tex
└── (supporting files)
```

**Enumeration** (`templates::installed()`):
1. `readdir(root)` → each subdir is a namespace candidate.
2. `readdir(namespace)` → each subdir is a name candidate.
3. Load `tex-template.toml`; if invalid, log a warning and skip (never crash the pipeline for a corrupt sibling package).
4. Verify manifest identifier matches directory path; on mismatch → skip with warning (aligns with `InstallError::PathMismatch` produced by install).

**Source**: FR-015, R5.

## Trust Record

**Home**: `trust.rs`, persisted in `~/.local/share/tex/trust.toml`.

```rust
#[derive(Serialize, Deserialize)]
pub struct TrustFile {
    pub schema_version: u32, // = 1 for v1
    pub trust: Vec<TrustRecord>,
}

#[derive(Serialize, Deserialize)]
pub struct TrustRecord {
    pub identifier: String,   // namespace/name, canonical form
    pub version: String,      // canonical semver string
    pub approved_at: String,  // RFC 3339 timestamp
}
```

**Operations**:
- `TrustFile::load()` — reads via `fs::read_to_string`; missing file → default (empty vec, schema_version = 1).
- `TrustFile::is_trusted(id: &Identifier, ver: &Version) -> bool`.
- `TrustFile::grant(id, ver)` — appends record, writes via `atomic::write_atomic` (spec 005 helper).
- `TrustFile::revoke(id)` — removes all records for that identifier (used by `template remove`).
- `TrustFile::revoke_version(id, ver)` — removes a single record (future use; not exposed on CLI in v1).

**Validation**:
- On load, records with unparseable version or identifier are dropped with a warning (never crash).
- Duplicate `(identifier, version)` pairs are deduplicated on write (last-write-wins on `approved_at`).

**Source**: FR-016, R3.

## Document Type Descriptor

**Home**: `discovery.rs`.

```rust
pub enum TemplateOrigin {
    BuiltIn { path: PathBuf },                          // resolved under examples/templates/
    Installed { manifest: Manifest, package_dir: PathBuf }, // package_dir is the on-disk root
}

pub struct DocumentTypeDescriptor {
    pub identifier: Identifier,
    pub origin: TemplateOrigin,
}
```

**Resolution** (`discovery::resolve(json_doc: &Value) -> Result<DocumentTypeDescriptor, ResolveError>`):
1. If `json_doc["document"]["template"]` is present and parses as an `Identifier`, use it as an explicit override:
   - If the identifier is namespaced → require an Installed match with matching identifier; else `ResolveError::ExplicitMissing`.
   - If bare → require a BuiltIn match; else `ResolveError::ExplicitMissing`.
2. Else if `json_doc["document"]["type"]` is a string:
   - First check built-in list; if a BuiltIn's bare identifier equals this string → return it.
   - Else check installed list; collect every Installed whose identifier `name` (after slash) equals this string OR whose full `namespace/name` equals it.
     - Zero matches → `ResolveError::NoMatch { candidates: [...] }` (`candidates` = near-name built-in and installed identifiers to include in FR-004 error output).
     - One match → return it.
     - Two or more matches → `ResolveError::Ambiguous { candidates }` naming every candidate identifier for FR-018 compliance.
3. Else `ResolveError::MissingTypeField`.

**Source**: FR-002, FR-003, FR-004, FR-018.

## Sandbox Directive

**Home**: `compiler.rs`.

```rust
pub struct SandboxDirective {
    pub disable_shell_escape: bool, // always true for third-party
    pub paranoid_openout: bool,     // sets openout_any=p env var
    pub only_cached_bundle: bool,   // adds --only-cached
}

impl SandboxDirective {
    pub fn for_origin(origin: &TemplateOrigin) -> Self {
        match origin {
            TemplateOrigin::BuiltIn { .. } => Self { disable_shell_escape: false, paranoid_openout: false, only_cached_bundle: false },
            TemplateOrigin::Installed { .. } => Self { disable_shell_escape: true, paranoid_openout: true, only_cached_bundle: true },
        }
    }
}
```

**Application**: `compiler::compile(engine, tex_path, output_dir, sandbox)` — the existing signature grows one parameter; call sites in `build.rs` derive it via `SandboxDirective::for_origin(&descriptor.origin)`.

**Source**: FR-017, R1.

## Pipeline Invocation & Pipeline Report

Unchanged in shape from spec 005; two new fields added.

**Home**: `build.rs`.

```rust
pub struct Invocation {
    pub json_source: JsonSource,           // File(PathBuf) | Stdin — unchanged
    pub output_override: Option<PathBuf>,
    pub engine_override: Option<Engine>,
    // no new user-facing input fields in v1
}

pub struct Report {
    pub output_path: PathBuf,
    pub template_identifier: Identifier,   // NEW — replaces "template source: library|generated"
    pub template_version: Option<Version>, // NEW — Some for installed, None for built-in
    pub engine: Engine,
    pub elapsed: Duration,
    pub warnings: Vec<Warning>,
}
```

**Source**: FR-010, spec 005 report extended.

## Errors

**Home**: `errors.rs` (existing crate module). Six new enums, each `#[derive(Debug, thiserror::Error)]`:

- `InstallError` — `GitBinaryMissing`, `GitCloneFailed { url, exit }`, `PathMismatch`, `Overwrite { path }`, `ManifestInvalid(#[from] ManifestError)`, `Io(#[from] std::io::Error)`.
- `ManifestError` — `NotFound`, `Parse { source }`, `MissingField { field }`, `InvalidIdentifier { value }`, `InvalidVersion { value }`, `EntrypointEscape`, `EntrypointMissing { path }`, `EntrypointNotTex { path }`.
- `TrustError` — `Io`, `Parse`, `DeniedByUser { identifier, version }`.
- `ResolveError` — `MissingTypeField`, `ExplicitMissing { requested }`, `NoMatch { requested, candidates }`, `Ambiguous { requested, candidates }`.
- `SandboxError` — `FilesystemEscape { path }`, `BundleMissing`, `ShellEscapeAttempted`.
- `DiscoveryError` — wrapper for corrupted install layout during enumeration; non-fatal (logged only).

All map to distinct exit codes via `main.rs`'s existing exit-code mapping (Constitution III scriptability + FR-014).

**Source**: FR-014, spec 005 error taxonomy.

## Relationship diagram

```text
Invocation ──resolve──> DocumentTypeDescriptor ──origin──> TemplateOrigin
                                                          ├─ BuiltIn ────► compile(no sandbox)
                                                          └─ Installed ─┬─ Manifest ─► identifier + version
                                                                        └─ SandboxDirective (paranoid + no-net + no-shell)
                                                                                    │
                                                                             consult TrustFile
                                                                                    │
                                                                             prompt if unknown → grant()
                                                                                    │
                                                                             compile ─► Report
```

## Field ownership summary

| Entity                    | Persisted?            | Location                                          | Reader          | Writer            |
|---------------------------|-----------------------|---------------------------------------------------|-----------------|-------------------|
| Identifier                | No (derived)          | in-memory                                         | discovery       | parsers           |
| Manifest                  | Yes (per package)     | `<pkg>/tex-template.toml`                         | templates       | install           |
| Installed Template        | Yes (dir layout)      | `~/.local/share/tex/templates/<ns>/<name>/`       | templates       | install / remove  |
| Trust Record              | Yes (single file)     | `~/.local/share/tex/trust.toml`                   | trust           | trust             |
| Document Type Descriptor  | No (derived)          | in-memory                                         | discovery       | discovery         |
| Sandbox Directive         | No (derived)          | in-memory                                         | compiler        | compiler          |
| Report                    | No (returned)         | stdout via `main.rs`                              | build           | build             |
