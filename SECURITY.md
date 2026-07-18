# Security Policy

## Supported versions

`tex-cli` is currently pre-1.0. Only the latest release on `main`
receives security fixes.

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |
| < 0.1   | ❌        |

## Reporting a vulnerability

**Please do not open a public issue for security reports.**

Report vulnerabilities privately through GitHub's Private Vulnerability
Reporting:

1. Go to the [Security tab](https://github.com/JonathanBufon/tex-cli/security/advisories)
   of the repository.
2. Click **"Report a vulnerability"**.
3. Fill out the advisory form with as much detail as you can:
   - Affected version(s) or commit SHA
   - Steps to reproduce
   - Impact (what an attacker could achieve)
   - Any suggested mitigation, if you have one

If you cannot use the GitHub form, open a minimal public issue asking a
maintainer to contact you privately — do not include the vulnerability
details in that issue.

## What to expect

- **Acknowledgement**: within 5 business days.
- **Initial assessment**: within 10 business days, we will let you know
  whether the report is accepted as a vulnerability, needs more
  information, or does not apply.
- **Fix and disclosure**: for confirmed issues, we will work on a fix
  and coordinate a public advisory with you. Please do not disclose the
  issue publicly until the advisory is published.

## Scope

`tex-cli` is a local command-line tool. The kinds of issues we
consider security-sensitive include:

- **Path traversal** in template, output, or config paths.
- **Command injection** via engine or template arguments.
- **Unsafe file overwrites** that bypass the atomic-write or
  overwrite-consent contracts.
- **Config file parsing** vulnerabilities (TOML, JSON, template files).

Out of scope:

- Bugs in the underlying LaTeX engines (tectonic, latexmk, xelatex).
  Report those to their respective upstreams.
- Issues that require the attacker to already have write access to your
  `~/.config/tex/` or templates directory.
