# Feature Specification: Auto PDF Pipeline

**Feature Branch**: `007-auto-pdf-pipeline`

**Created**: 2026-07-27

**Status**: Draft

**Input**: User description: "Automate the full pipeline we just did manually: receive a JSON file describing a document (structure/data), auto-develop or select a matching LaTeX template, compile it via the existing tex-cli engine (tectonic in Docker), and deliver the finished PDF."

## Context and Motivation

The first end-to-end test of `tex-cli` (using the resume JSON at `/home/jonathan/Downloads/resume_jonathan_bufon_en-US.json`) exposed how much manual work a new user faces between "I have a JSON" and "I have a PDF":

1. No template ships that matches an arbitrary JSON shape — the user must hand-author LaTeX + template-engine syntax from scratch.
2. Data is never escaped for LaTeX — special characters in JSON strings can silently corrupt the PDF (observed: em-dash `—` dropped without any warning; potential: `&`, `#`, `%`, `_`, `$` breaking compilation).
3. Template-engine syntax collides with common LaTeX macros (e.g. `\MakeUppercase{#1}` was parsed as a template comment start), forcing manual escape hatches that only an expert would discover.
4. Config setup and container plumbing (persistent `HOME`, file ownership) are undocumented for the first-run path.

The result: even a well-prepared user needs ~90 minutes of research, template writing, and three failed compile iterations to get a first PDF. The goal of this feature is to reduce that to one command with no manual template authoring.

## Clarifications

### Session 2026-08-03

- Q: How are third-party templates distributed and discovered by `tex-cli`? → A: Git URL or local filesystem path only for v1 (no central registry).
- Q: What is the trust granularity for installed third-party templates? → A: Per template + version; re-prompt only when the local install advances to a new version.
- Q: What is the execution security posture for third-party templates? → A: Hard sandbox — `\write18` disabled, no network, no filesystem writes outside the output directory. Built-in library templates are exempt (retain Tectonic's default sandbox).
- Q: How are templates identified and how are collisions between built-in and installed templates resolved? → A: Built-in templates use a bare name (e.g. `invoice`); third-party templates MUST declare a namespace (e.g. `acme/invoice`). JSON may set `document.template` for an explicit selection; otherwise `document.type` matches built-in first. If no built-in matches and multiple installed templates match, the pipeline errors with the list of candidates.
- Q: What MUST a third-party template package contain to be installable? → A: A manifest with three required fields — `identifier` (`namespace/name`), `version` (semver), `entrypoint` (relative path to the main `.tex` template) — plus the template body. No other fields are mandatory in v1.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — First PDF from a supported document type (Priority: P1)

A user has a JSON file whose `document.type` (or equivalent identifying field) matches a document type already known to `tex-cli` (e.g. `resume`, `article`, `letter`). They run a single command pointing at the JSON, and receive a compiled PDF at a predictable path.

**Why this priority**: This is the MVP. It delivers "JSON in → PDF out" for the built-in document types without any template authoring or install step, which is the largest usability leap over today.

**Independent Test**: Can be fully tested by piping any JSON matching a library template (starting with `resume`, extracted from the spec 006 test artifact) into `tex-cli` and verifying a well-formed PDF is produced without the user creating, editing, or naming a template file.

**Acceptance Scenarios**:

1. **Given** a JSON with `document.type = "resume"` and a resume library template exists, **When** the user runs the pipeline command against that JSON, **Then** a PDF is produced containing all sections and items from the JSON, at the configured output path, with no template files touched by the user.
2. **Given** the same JSON but the user overrides the output path, **When** the pipeline runs, **Then** the PDF appears at the specified path, and no other file is written to the default location.
3. **Given** a JSON with special characters in visible text (`&`, `%`, `#`, `_`, em-dash), **When** the pipeline runs, **Then** every character in the source JSON appears correctly in the PDF (no silent drops, no compile failure).

---

### User Story 2 — Install and use a third-party template (Priority: P2)

A user has a JSON whose shape is not covered by the built-in library. Instead of hand-authoring a template, they install a community-contributed template package (Git URL or local path) using the tool's install command, approve it once, and then run the pipeline against their JSON to get a PDF — no template files touched by hand.

**Why this priority**: This is what makes the "open-ended library + contribution SDK" (Clarification Q3) actually useful. Without it, users outside the three built-in types hit a dead end.

**Independent Test**: Can be tested by installing a template package from a Git URL that provides a namespaced identifier matching the user's `document.type` (or referenced explicitly by `document.template`), approving it at the trust prompt, and verifying the pipeline compiles a PDF from a JSON that had no built-in match.

**Acceptance Scenarios**:

1. **Given** a JSON whose `document.type` matches no built-in template, and a third-party template with a valid manifest is available at a Git URL, **When** the user runs the install command against that URL and then runs the pipeline against the JSON, **Then** the user is prompted once to approve the template at its declared version, and after approval a PDF is produced.
2. **Given** a previously approved installed template at version `1.0.0`, **When** the user re-installs the same template at version `1.1.0` and runs the pipeline, **Then** the pipeline halts with a re-approval prompt and does not compile until the user approves the new version.
3. **Given** an installed third-party template that attempts a sandbox-restricted operation during compile (e.g. `\write18`, network fetch, write outside the output directory), **When** the pipeline compiles, **Then** the compile fails with a message naming the restricted capability and the template identifier that requested it.

---

### User Story 3 — Safe rendering of arbitrary user data (Priority: P2)

Regardless of whether the template is built-in or installed via the SDK, every value interpolated from the JSON into the LaTeX source is escaped or transformed so that it can never silently corrupt the output or cause a compile error rooted in LaTeX special characters.

**Why this priority**: Silent corruption (the em-dash case) is worse than a hard failure — the user ships a broken PDF without noticing. This must hold for any pipeline invocation, including manual templates.

**Independent Test**: Can be tested by supplying a JSON with every LaTeX special character in string values and verifying (a) the pipeline succeeds, (b) the PDF preserves every character correctly, and (c) if any character cannot be represented, the pipeline reports it explicitly and does not silently drop it.

**Acceptance Scenarios**:

1. **Given** a JSON string field containing `&`, `%`, `$`, `#`, `_`, `{`, `}`, `\`, `~`, `^`, **When** the pipeline compiles the PDF, **Then** every character is visible in the rendered PDF at the correct position.
2. **Given** a Unicode character in the JSON that the compilation environment cannot render, **When** the pipeline runs, **Then** it either substitutes the character with a visible placeholder and reports the substitution, or fails with a clear message pointing to the offending JSON path — never silently drops the character.
3. **Given** a hand-authored template that includes LaTeX macros with syntax patterns that resemble template-engine directives (e.g. `\somecmd{#1}`, `{ {{ x }} }`), **When** the pipeline renders the template, **Then** the template engine does not misinterpret the LaTeX, or emits a specific diagnostic pointing to the exact conflicting characters.

---

### User Story 4 — Zero-config first run (Priority: P3)

A user with a freshly installed `tex-cli` and no existing config runs the pipeline command against a JSON and gets a PDF, without being asked to first run a separate initialisation command or hand-edit a config file.

**Why this priority**: Removes an unnecessary step gate. Config setup was the second point of friction in the first-run test (after template authoring) and is fundamentally boilerplate.

**Independent Test**: Can be tested by removing all `tex-cli` config from a clean environment, running the pipeline command against a valid JSON, and confirming a PDF is produced with the config auto-created at a sensible default location.

**Acceptance Scenarios**:

1. **Given** no `tex-cli` config exists on the system, **When** the user runs the pipeline against a valid JSON, **Then** the pipeline creates a default config, produces the PDF, and reports where the config was written for future reference.
2. **Given** the compilation engine is not available on the host but a container-based engine is available, **When** the pipeline runs, **Then** it detects and uses the container-based engine transparently, and reports which engine was used.

---

### Edge Cases

- The JSON is well-formed but semantically empty (no `document`, no `sections`, no top-level content). Pipeline must fail with a message naming what it expected to find, not compile an empty PDF.
- The JSON contains a very long single string field (e.g. a 100-page block of text) that would overflow a single page. The PDF must still be produced with automatic pagination — the pipeline does not truncate or reject content by length.
- An installed third-party template introduces a LaTeX construct that fails to compile (e.g. missing package). The error must attribute the failure to the template identifier and version, not surface a raw LaTeX log without context, and must point the user to the entrypoint file within the installed template package for inspection.
- Two invocations produce two different PDFs for the same JSON due to non-determinism (e.g. random ID). Repeated invocations must produce byte-identical PDFs, or explicitly document any source of variance.
- The user passes a JSON path that does not exist, or `-` for stdin with no input. Pipeline exits with a distinct error code and message that does not mention templates or compilation.
- The compile step exceeds a reasonable time budget (e.g. more than 2 minutes for a single document). Pipeline surfaces a timeout with the current phase (rendering vs compiling), not a silent hang.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a single command that accepts a JSON source (file path or stdin) and produces a compiled PDF at a resolved output path, with no intermediate manual step required.
- **FR-002**: The system MUST maintain a library of at least one document-type template that ships with the tool and can be matched to incoming JSON by a well-defined identifying field.
- **FR-003**: When incoming JSON matches a library template, the system MUST render and compile without prompting the user to select, name, or create a template.
- **FR-004**: When incoming JSON does not match any built-in or installed template, the system MUST fail with a message that (a) names the JSON field it used for resolution (`document.template` or `document.type`), (b) lists any built-in or installed templates whose identifiers are near-matches, and (c) instructs the user how to install a matching third-party template via the install command (per FR-015). Auto-generation of a template is explicitly out of scope in v1 (Clarification Q1 = library-only).
- **FR-005**: The system MUST escape or transform every value interpolated from the JSON into the LaTeX source so that no LaTeX special character in user data can cause a compile failure or silent visual corruption.
- **FR-006**: The system MUST detect and diagnose collisions between template-engine syntax and LaTeX macro syntax at template-load time (before compilation), and emit an error that names the exact character sequence and location responsible.
- **FR-007**: When no config exists at first-run, the system MUST create a default config at a documented location and continue with the pipeline, without failing or requiring a separate initialisation command.
- **FR-008**: The system MUST detect whether the configured compilation engine is available and, when it is not, MUST select a working fallback (e.g. a containerised engine) if one is available; if none is available it MUST fail with a message that names the missing engine(s).
- **FR-009**: The system MUST produce byte-identical PDFs on repeated invocations against the same JSON and template, unless the template explicitly declares a variable source (which MUST be documented).
- **FR-010**: The system MUST report, on successful completion, the output path, the template identifier used (bare name for built-in, `namespace/name` for installed) plus its version when applicable, the engine used, and the total elapsed time — enough information to reproduce the run.
- **FR-011**: The system MUST expose the intermediate rendered LaTeX (before compilation) on demand, so a user can inspect or edit it without having to re-derive it from the pipeline.
- **FR-013**: The system MUST bound the total pipeline runtime (rendering + compilation) with a configurable timeout and surface a distinct error identifying the phase that timed out.
- **FR-014**: The system MUST distinguish, in its exit codes and error messages, between input errors (bad JSON, missing file), rendering errors (template failed to interpolate), compilation errors (engine failed), and environment errors (engine not available, config invalid).
- **FR-015**: The system MUST provide a template-install command that accepts either a Git URL or a local filesystem path and, on success, places the third-party template in a documented local templates directory where the pipeline can discover it by document type on subsequent runs. No central registry is required in v1.
- **FR-016**: The system MUST require explicit user approval for each installed third-party template at a specific version before that template is used in any compile. Approval MUST be persisted so subsequent runs of the same template + version do not re-prompt. When the local install of an approved template advances to a new version, the system MUST re-prompt for approval and MUST NOT compile with the new version until it is approved. Built-in library templates are pre-trusted and require no approval.
- **FR-017**: When compiling with a third-party (non-built-in) template, the system MUST run the compilation with `\write18` shell-escape disabled, with no network access, and with filesystem writes restricted to the resolved output directory. Built-in library templates retain the compilation engine's default sandbox. If a third-party template attempts a restricted operation, the compile MUST fail with a message that names the restricted capability and the template that requested it.
- **FR-018**: The system MUST identify built-in templates by a bare name (e.g. `invoice`) and MUST require every third-party template to declare a namespaced identifier of the form `namespace/name` (e.g. `acme/invoice`); the install command MUST reject any third-party template that lacks a namespace or whose namespace is empty. When resolving which template to use for an incoming JSON, the system MUST first honor an explicit `document.template` field on the JSON if present, otherwise match `document.type` against built-in templates first, and only if no built-in matches consider installed templates. If neither an explicit selection nor a built-in match resolves the template and two or more installed templates match, the system MUST fail with an error listing every candidate identifier and instructing the user to set `document.template` explicitly.
- **FR-019**: A third-party template package MUST include a manifest declaring exactly three required fields: `identifier` (a `namespace/name` string per FR-018), `version` (a semver string), and `entrypoint` (a relative path within the package pointing to the main `.tex` template). The install command MUST reject any package that lacks the manifest, lacks any required field, or contains a required field that fails its format validation, and MUST report the specific missing or invalid field. No additional manifest fields are required in v1; unknown fields MUST be preserved but ignored.

### Key Entities

- **Document Type Descriptor**: Identifies a class of document (e.g. `resume`, `acme/invoice`) plus the JSON shape it expects. Attributes include an identifier (bare name for built-in, `namespace/name` for third-party), a human name, the expected top-level fields, and a pointer to the template implementing it (built-in or installed).
- **Library Template**: A packaged, versioned template for one document type. Shipped with the tool; not authored by the user for a first-time flow.
- **Installed Template**: A template acquired via the install command (Git URL or filesystem path) and placed in the local templates directory. Once installed and approved, it is discoverable by document type on subsequent pipeline runs (per FR-015).
- **Template Package Manifest**: The declaration shipped alongside a third-party template body, holding the three required fields (`identifier`, `version`, `entrypoint`) that the install command validates and that the trust prompt and discovery mechanism consume (per FR-019).
- **Trust Record**: A persisted approval tying a specific installed template to a specific version, marking it usable for compilation without re-prompting. New versions of the same template produce no record until re-approved (per FR-016).
- **Pipeline Invocation**: A single request combining a JSON source, an optional output path, an optional engine override; produces a Pipeline Report plus a PDF file.
- **Pipeline Report**: A structured summary of one invocation: input source, output path, template source (library/generated), engine used, elapsed time, warnings (e.g. character substitutions).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with a JSON matching a library document type can produce a PDF with a single command in under 60 seconds on the first run (including any first-run environment setup), without opening, creating, or editing a template file.
- **SC-002**: 100% of characters present in the JSON string values appear in the final PDF, unless the pipeline emitted an explicit substitution warning for that character — zero silent drops.
- **SC-003**: 100% of compile failures identify (a) the phase (render / compile), (b) the offending JSON path or template location, and (c) a corrective action, without requiring the user to read the raw compilation log.
- **SC-004**: Templates authored by an intermediate user (able to write basic LaTeX) never fail with template-engine syntax collisions at compile time — either the pipeline supports the LaTeX pattern natively, or emits a load-time diagnostic pointing to the exact collision.
- **SC-005**: A user running the pipeline with no prior config completes their first PDF without invoking any command other than the pipeline command itself.
- **SC-006**: Given the built-in library covers the document types listed in Assumption A-04, at least 90% of JSON files a user is likely to bring in the first month of use are handled either by a built-in template or by a third-party template the user installed via the SDK; unmet cases fail with the actionable error defined in FR-004 (not a silent partial PDF).
- **SC-007**: Re-running the pipeline against an unchanged JSON produces a byte-identical PDF, or the source of variance is documented in the Pipeline Report.

## Assumptions

- **A-01**: The tool continues to run in an environment where a containerised LaTeX engine (Tectonic in a Debian container) is available; the pipeline layers on top of the existing render + compile subsystems delivered in specs 003, 004, and 005.
- **A-02**: The tool's audience remains developers or technically-comfortable users on Linux/macOS; a full GUI is out of scope.
- **A-03**: The JSON source of truth is authored by the user (or a system they control); the pipeline does not need to defend against adversarial JSON inputs, only against unintentional structural or character-set issues. This assumption does NOT extend to third-party templates installed via the SDK — those are assumed potentially adversarial and are sandboxed at compile time per FR-017.
- **A-04**: The initial built-in library covers exactly the `resume` document type (extracted from the spec 006 first-run test) and the two shapes already present in `examples/templates/` (`artigo-basico`, `carta`). Additional document types are delivered by third-party template packages that users install via the SDK (per Clarification Q3 = open-ended library + contribution SDK); the built-in library does not grow organically in v1.
- **A-05**: The pipeline command may be a new top-level subcommand or an extension of the existing `build` subcommand — that choice is a design decision, not a scope decision.
- **A-06**: Character substitution (for glyphs unsupported by the compilation environment) is preferred over hard failure when the substitution is unambiguous (e.g. em-dash → LaTeX `---` shorthand), and each substitution is reported.
- **A-07**: Third-party templates installed via the SDK are expected to vary widely in quality; the tool does not attempt to grade or rank them. The user's judgment at the trust prompt (FR-016), combined with the sandbox at compile time (FR-017), is the entire quality/safety envelope in v1.

## Clarifications Needed

Three decisions materially shape the scope, effort, and user experience of this feature. They are not internal implementation details — each represents a distinct product path.

### Q1: Auto-template generation strategy for unknown JSON shapes

**Context**: FR-004 depends on this. When a JSON does not match any library template, the system needs a policy for what to do.

**What we need to know**: Which strategy for handling unknown JSON shapes?

| Option | Answer                                                                                       | Implications                                                                                                                    |
|--------|----------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------|
| A      | Heuristic / schema-based generator (no external calls): infer structure, emit a generic template. | Fully offline, deterministic, but produces plain / uniform output; deeply nested or mixed shapes may hit limits.                |
| B      | LLM-assisted generation (calls an external model at pipeline time).                          | Higher-quality templates, adapts to novel shapes; adds network dependency, non-determinism, cost, and a large new surface.      |
| C      | Library-only: unknown JSON is a hard error with a suggestion to author or extend a template. | Smallest scope, most predictable; leaves the first-run friction in place for anything outside the shipped library.              |
| Custom | Provide your own answer                                                                      | E.g. "A for v1, B behind a flag later".                                                                                          |

**Your choice**: **C — Library-only**. Unknown JSON is a hard error with a suggestion to author or install a template. No auto-generation in v1. This collapses US2 (P2, auto-generated template for unknown shape) and FR-004/FR-012 (auto-template generation and rendering) out of v1 scope; the coverage promise shifts to the contribution SDK selected in Q3.

### Q2: Interaction model when a template is auto-generated

**Context**: US2 and FR-012 depend on this. When the system generates a template on the fly, does the user see it before compilation?

**What we need to know**: Which interaction model when a template is generated?

| Option | Answer                                                                                                          | Implications                                                                                                          |
|--------|-----------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| A      | Fully automatic: generate → compile → deliver PDF without prompting; user can save the template afterward.      | Fastest path, matches the "one command → PDF" vision; risk of shipping a template the user never inspected.           |
| B      | Automatic with iteration loop: generate → compile → deliver PDF + template side-by-side for edits.              | Best learning path; adds one extra file to the output surface.                                                        |
| C      | Approval-required: generate → show template → wait for user to approve → compile.                               | Safest for high-stakes outputs (legal, financial); breaks the single-command vision.                                  |
| Custom | Provide your own answer                                                                                         | E.g. "A by default, `--review` flag to switch to C".                                                                  |

**Your choice**: **C — Approval-required, reinterpreted for the SDK path**. Because Q1 = C removes auto-generation from v1, the "approval-required" model no longer applies to a generator loop. Instead, it applies to **templates installed via the contribution SDK selected in Q3**: before a third-party or freshly-installed template is used in a compile, the user must explicitly confirm/approve it (first-use trust prompt, or an equivalent one-time acknowledgement). Templates shipped in the built-in library are pre-trusted and require no approval.

### Q3: Scope of the shipped document-type library for v1

**Context**: Assumption A-04 and SC-006 depend on this. The library covers today only `resume`, `artigo-basico`, `carta`.

**What we need to know**: Which library scope for v1?

| Option | Answer                                                                                                       | Implications                                                                                                          |
|--------|--------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| A      | Ship exactly what exists: `resume`, `artigo-basico`, `carta`. All others rely on auto-generation (per Q1).   | Smallest surface, ships fastest; SC-006 (90% coverage) leans hard on auto-generation quality.                         |
| B      | Ship 5–6 common types: add `invoice`, `report`, `slides` (or equivalent).                                    | Moderate surface; each template is real work; better user coverage without leaning on auto-generation.                |
| C      | Explicit non-goal: the library is open-ended and grows organically; ship a template SDK/schema for contributions. | Reframes the feature — auto-generation becomes secondary; distribution of user-contributed templates is now in scope. |
| Custom | Provide your own answer                                                                                      |                                                                                                                       |

**Your choice**: **C — Open-ended library + contribution SDK**. Built-in library stays at `resume`, `artigo-basico`, `carta` for v1. The feature centrepiece becomes a **template SDK/schema** that lets third parties author, package, and distribute templates that `tex-cli` can discover and install. Auto-generation is explicitly out of v1 scope (per Q1). SC-006 (90% coverage) is re-anchored on "library + installable community templates" rather than on generator quality.
