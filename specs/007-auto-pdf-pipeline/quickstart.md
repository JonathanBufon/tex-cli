# Quickstart — Auto PDF Pipeline (spec 007)

**Feature**: 007-auto-pdf-pipeline
**Prereqs**: `tex-cli` built at branch `007-auto-pdf-pipeline`, Docker container per project convention, `git` binary available.

This walkthrough exercises every new surface from spec 007 end to end: install a third-party template from a Git URL, get the trust prompt, compile a JSON to PDF using it, observe sandbox enforcement, and verify the report.

## 0. Warm the Tectonic bundle cache

The sandbox for third-party templates requires `--only-cached`. On a fresh install the cache is empty, which would fail every third-party compile. `tex-cli init` (existing subcommand from spec 001) warms it by running a no-op compile against a bundled fixture.

```bash
tex-cli init
```

Expected: config written to `~/.config/tex/config.toml`, Tectonic bundle downloaded to its default cache location, and stdout confirms both paths.

## 1. Baseline — compile a built-in template

Sanity check that the built-in path still works and does not require any trust prompt.

```bash
cat > /tmp/resume.json <<'JSON'
{
  "document": { "type": "resume" },
  "person": { "name": "Alice Example" },
  "sections": []
}
JSON

tex-cli build --json /tmp/resume.json --output /tmp/resume.pdf
```

Expected stdout (shape only):

```text
output: /tmp/resume.pdf
template: resume (built-in)
engine: tectonic
elapsed: 3.42s
```

No trust prompt, no sandbox errors.

## 2. Install a third-party template from a local path

Simulate a third-party author with a minimal package in a scratch directory.

```bash
mkdir -p /tmp/acme-invoice
cat > /tmp/acme-invoice/tex-template.toml <<'TOML'
identifier = "acme/invoice"
version    = "1.0.0"
entrypoint = "template.tex"
TOML

cat > /tmp/acme-invoice/template.tex <<'TEX'
\documentclass{article}
\begin{document}
Invoice for {{ client }} — total {{ total }}.
\end{document}
TEX

tex-cli template install /tmp/acme-invoice
```

Expected stdout:

```text
installed acme/invoice@1.0.0 at /home/<user>/.local/share/tex/templates/acme/invoice
```

Verify with `tex-cli template list`:

```text
IDENTIFIER      VERSION   PATH
acme/invoice    1.0.0     /home/<user>/.local/share/tex/templates/acme/invoice
```

## 3. Compile a JSON that resolves to the installed template — hit the trust prompt

```bash
cat > /tmp/inv.json <<'JSON'
{
  "document": { "type": "invoice", "template": "acme/invoice" },
  "client": "Bob Client",
  "total": "\\$1{,}234.56"
}
JSON

tex-cli build --json /tmp/inv.json --output /tmp/inv.pdf
```

Expected: an interactive prompt appears —

```text
? Trust `acme/invoice@1.0.0`? This template will run inside the sandbox
  described in spec 007 (no shell escape, no network, writes confined to
  the output directory). [y/N]
```

Answer `y`. A trust record is written to `~/.local/share/tex/trust.toml` and the compile proceeds:

```text
output: /tmp/inv.pdf
template: acme/invoice
version: 1.0.0
engine: tectonic
elapsed: 4.11s
```

Re-run the same command; this time there is **no prompt** — the trust record is honored:

```bash
tex-cli build --json /tmp/inv.json --output /tmp/inv.pdf --force
```

(The `--force` here permits PDF overwrite per FR-005 / Constitution V — trust is orthogonal.)

## 4. Upgrade the template — verify re-prompt

Publish a new version.

```bash
sed -i 's/version    = "1.0.0"/version    = "1.1.0"/' /tmp/acme-invoice/tex-template.toml
tex-cli template install /tmp/acme-invoice --force
```

Re-run the build:

```bash
tex-cli build --json /tmp/inv.json --output /tmp/inv.pdf --force
```

Expected: the trust prompt appears again — this time for `acme/invoice@1.1.0`. The previous approval for `1.0.0` is not carried forward (FR-016).

## 5. Sandbox check — a malicious template attempts `\write18`

Update the template to try shell escape:

```bash
cat > /tmp/acme-invoice/template.tex <<'TEX'
\documentclass{article}
\immediate\write18{echo pwned > /tmp/pwned}
\begin{document}
attempt
\end{document}
TEX

sed -i 's/version    = "1.1.0"/version    = "1.2.0"/' /tmp/acme-invoice/tex-template.toml
tex-cli template install /tmp/acme-invoice --force
tex-cli build --json /tmp/inv.json --output /tmp/inv.pdf --force
```

After approving the new version at the trust prompt, expected: compile fails with an FR-017 message —

```text
error: template acme/invoice@1.2.0 attempted a restricted operation:
       shell escape (\write18) — disabled for third-party templates.
       compile aborted; no PDF produced.
exit: 5
```

Confirm nothing was written outside the output directory:

```bash
ls /tmp/pwned 2>/dev/null || echo "no pwned file — sandbox held"
```

Expected: `no pwned file — sandbox held`.

## 6. Ambiguity check — install a second `*/invoice` template

```bash
mkdir -p /tmp/myorg-invoice
cat > /tmp/myorg-invoice/tex-template.toml <<'TOML'
identifier = "myorg/invoice"
version    = "0.2.0"
entrypoint = "t.tex"
TOML
cp /tmp/acme-invoice/template.tex /tmp/myorg-invoice/t.tex
# Fix template to remove \write18 attempt for this test.
sed -i '/write18/d' /tmp/myorg-invoice/t.tex
tex-cli template install /tmp/myorg-invoice
```

Now run a JSON that uses only `document.type`, no explicit `template`:

```bash
cat > /tmp/ambig.json <<'JSON'
{ "document": { "type": "invoice" }, "client": "x", "total": "1" }
JSON

tex-cli build --json /tmp/ambig.json --output /tmp/ambig.pdf
```

Expected FR-018 ambiguity error:

```text
error: `document.type = "invoice"` matched multiple installed templates:
         acme/invoice
         myorg/invoice
       set `document.template` in the JSON to disambiguate.
exit: 2
```

## 7. Cleanup

```bash
tex-cli template remove acme/invoice --yes
tex-cli template remove myorg/invoice --yes
rm -rf /tmp/acme-invoice /tmp/myorg-invoice /tmp/*.json /tmp/*.pdf
```

The trust records for both identifiers are dropped automatically as part of `template remove` (per FR-016 lifecycle).

## What this proves

By the end of the walkthrough, every FR added in spec 007 has been exercised end-to-end:

- **FR-015** (install) — steps 2 and 4.
- **FR-016** (trust granularity + re-prompt on version bump) — steps 3 and 4.
- **FR-017** (sandbox) — step 5.
- **FR-018** (identifier + collision) — step 6.
- **FR-019** (manifest) — implicit in every install (packages with missing fields would have been rejected).
- **FR-010** (report) — visible on every successful compile.
- **US1** (built-in) — step 1.
- **US2** (install and use third-party) — steps 2–4.
- **US3** (safe rendering of user data) — the `\\$` escape in step 3 verifies FR-005.
