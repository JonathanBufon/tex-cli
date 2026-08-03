# Contract — JSON `document` fields consumed by discovery

**Feature**: 007-auto-pdf-pipeline
**Source**: FR-002, FR-018.

The pipeline reads two optional top-level fields from user JSON to resolve which template to compile. All other fields are passed through to the template renderer unchanged.

## Fields

| Field                | Type   | Required? | Purpose                                                                                       |
|----------------------|--------|-----------|-----------------------------------------------------------------------------------------------|
| `document.type`      | string | Yes (unless `document.template` is provided) | Identifies a document class; matched first against built-in templates, then installed. |
| `document.template`  | string | No        | Explicit override — bypasses `document.type` matching. Must be a full identifier: bare `<name>` selects a built-in, `<namespace>/<name>` selects an installed template. |

Both fields live under the top-level `document` object by convention already established in the spec 006 resume JSON test artifact.

## Resolution algorithm (mirror of `discovery::resolve`)

```text
if json.document.template is present:
    parse as Identifier
    if identifier.namespace is Some:
        require installed match on identifier
    else:
        require built-in match on identifier
    return match or error ExplicitMissing

if json.document.type is a string t:
    if any built-in has identifier.name == t and identifier.namespace == None:
        return that built-in
    candidates = every installed I where
                    (I.identifier.name == t)
                 or (I.identifier.to_string() == t)
    if candidates.len() == 0: error NoMatch { candidates: near_matches() }
    if candidates.len() == 1: return candidates[0]
    if candidates.len() >= 2: error Ambiguous { candidates }

error MissingTypeField
```

## Examples

### Case A — Built-in match by type

```json
{ "document": { "type": "resume" }, "sections": [...] }
```

Resolves to the built-in `resume` template.

### Case B — Installed match by type (no built-in of the same name)

```json
{ "document": { "type": "invoice" } }
```

If exactly one installed template has `identifier = "*/invoice"` (any namespace), it wins. If two do (e.g. `acme/invoice` and `myorg/invoice`) → `Ambiguous` error listing both.

### Case C — Explicit override for a specific installed variant

```json
{ "document": { "type": "invoice", "template": "acme/invoice" } }
```

Bypasses the ambiguity by pinning to `acme/invoice`. `document.type` is ignored during resolution but may still be consumed by the template body.

### Case D — Explicit override for a built-in

```json
{ "document": { "template": "resume" } }
```

Forces the built-in `resume` template even if the user has installed a `foo/resume`.

### Case E — Neither field present

```json
{ "sections": [...] }
```

Error: `MissingTypeField`. No default is applied — silent defaults are the exact anti-pattern that motivates this contract.

## Backward compatibility

Existing JSON documents that already use `document.type` (spec 006 resume artifact) continue to work unchanged, because bare-name matching against built-in templates is the first and highest-priority resolution rule.
