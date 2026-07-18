```bash
████████╗███████╗██╗  ██╗     ██████╗██╗     ██╗
╚══██╔══╝██╔════╝╚██╗██╔╝    ██╔════╝██║     ██║
   ██║   █████╗   ╚███╔╝     ██║     ██║     ██║
   ██║   ██╔══╝   ██╔██╗     ██║     ██║     ██║
   ██║   ███████╗██╔╝ ██╗    ╚██████╗███████╗██║
   ╚═╝   ╚══════╝╚═╝  ╚═╝     ╚═════╝╚══════╝╚═╝
```
# tex-cli

A terminal CLI written in Rust that turns structured JSON data into
LaTeX documents and compiles them to PDF via `tectonic` (default) or
another pluggable engine. This binary is an automation layer over the
LaTeX ecosystem — not a reimplementation of it.

This release closes the **JSON → PDF end-to-end** pipeline: `init` and
`config` for managing `~/.config/tex/config.toml`, `templates` for
administering the `.tex` catalog, `render` for applying JSON data to a
template, `compile` for producing a PDF from a `.tex`, and the new
`build` for orchestrating render + compile in a single invocation.

## Installation

Requires Rust stable (edition 2021).

```bash
cargo install --path .
```

After installing, verify:

```bash
tex-cli --version
```

## Basic usage

### First-time setup

Interactive:

```bash
tex-cli init
```

Or non-interactive (useful in scripts / CI):

```bash
tex-cli init \
  --templates-dir ~/Documents/tex/templates \
  --output-dir    ~/Documents/tex/output \
  --engine        tectonic \
  --create-dirs
```

Available flags:

- `-t, --templates-dir <path>` — skips the templates prompt
- `-o, --output-dir <path>` — skips the output prompt
- `-e, --engine <name>` — skips the engine prompt (`tectonic`,
  `latexmk`, `pdflatex`, `xelatex`, `lualatex`)
- `--create-dirs` — creates missing directories without confirming
- `--force` — overwrites an existing config without confirming

### Inspecting the config

```bash
tex-cli config show                    # default: human
tex-cli config show --format json      # pipe straight into jq
tex-cli config show --format toml      # TOML re-serialization
```

### Changing a key

```bash
tex-cli config set compiler.keep_tex false
tex-cli config set paths.templates_dir ~/new/folder
tex-cli config set compiler.engine tectonic
```

Accepted keys (canonical dotted-path, no aliases):

- `paths.templates_dir`
- `paths.output_dir`
- `compiler.engine`
- `compiler.keep_tex`
- `compiler.keep_logs`
- `behavior.ask_output_path_every_time`

### Managing templates

The `templates` group administers the `.tex` files stored under
`paths.templates_dir`.

```bash
tex-cli templates list                     # default: human
tex-cli templates list --format json       # pipe straight into jq

tex-cli templates show article             # raw bytes to stdout
tex-cli templates show article.tex         # extension optional

tex-cli templates add /tmp/new.tex                   # basename → new
tex-cli templates add /tmp/x.tex --name report       # custom name
tex-cli templates add /tmp/x.tex --force             # overwrite

tex-cli templates remove article --force   # remove without prompt

tex-cli templates                          # interactive menu (TTY)
```

Important contracts:

- `list --format json` produces valid JSON for piping into `jq` (even
  with an empty directory → `[]`).
- Each template carries `modified_at_epoch` (seconds since
  UNIX_EPOCH); use `jq '... | strftime("%Y-%m-%dT%H:%M:%SZ")'` to
  format as RFC3339.
- `add` validates UTF-8 before writing and performs an atomic write
  with `0644` permissions (shareable via git, unlike the config).
- `remove` requires `--force` or confirmation in an interactive
  terminal.

Curated template examples live under [`examples/templates/`](examples/templates/)
with attribution to the original author.

### Rendering JSON → `.tex`

The `render` subcommand applies JSON data to a LaTeX template and
produces the intermediate `.tex` — the central piece of the Data →
Template → PDF pipeline.

```bash
tex-cli render <template> <data.json>            # writes to output_dir
tex-cli render <template> <data.json> --output <path>
tex-cli render <template> <data.json> --dry-run  # stdout, no write
tex-cli render <template> <data.json> --force    # overwrite

echo '{"name":"Ana"}' | tex-cli render greeting -  # JSON via stdin

tex-cli render                                    # interactive menu
```

Important contracts:

- Templates use [`tera`](https://keats.github.io/tera/) syntax —
  `{{ var }}`, `{% if %}`, built-in filters (`upper`, `lower`,
  `default`, `join`, `length`, etc.).
- JSON MUST be a top-level object. Top-level arrays are rejected with
  exit 31 and a clear message.
- Autoescape=false by design — `.tex` is not HTML, escapes stay
  literal (`&`, `%`, `$` preserved).
- Atomic writes with `0644` permissions.
- `--dry-run` produces stdout byte-for-byte identical to the file
  that would be written (SC-005).
- `render` with no args in a TTY opens the menu (Select template +
  Text JSON + Confirm dry-run). Outside a TTY: exit 1 with a message
  pointing to the direct subcommand.

### Compiling `.tex` → PDF

The `compile` subcommand turns a `.tex` into a PDF using the
configured engine. Closes the JSON → PDF pipeline together with
`render`.

```bash
tex-cli compile <tex-file>                            # writes to output_dir
tex-cli compile <tex-file> --output <path>           # custom path
tex-cli compile <tex-file> --engine latexmk          # override engine for this invocation only
tex-cli compile <tex-file> --keep-tex --keep-logs    # copies .tex and .log to output_dir
tex-cli compile <tex-file> --no-keep-tex             # inverts the config default
tex-cli compile <tex-file> --force                   # overwrites existing PDF

tex-cli compile                                       # interactive menu
```

Important contracts:

- The default engine comes from `compiler.engine` in the config
  (`tectonic` is the default). `--engine <name>` overrides **only for
  this invocation** — the config file stays immutable.
- Accepted engines: `tectonic`, `latexmk`, `pdflatex`, `xelatex`,
  `lualatex`. Anything else → exit 42.
- Engine not installed on PATH → exit 41 with a clear message.
- Compile error → exit 40, stderr contains the **last ~30 lines** of
  the engine's `.log` for direct diagnosis.
- PDF written atomically with `0644` permissions.
- Compilation runs in an isolated `TempDir`; no residual artifacts on
  the filesystem after the command (guaranteed by tempfile RAII).
- `stdout` reports the PDF path and compile time:
  `PDF generated at <path>. Compile took X.Ys.`

### JSON → PDF pipeline (`build`)

The `build` subcommand orchestrates render + compile in a single
invocation — the "closer" of the JSON → PDF end-to-end pipeline.

```bash
tex-cli build <template> <data.json>                 # writes to output_dir/<template>.pdf
tex-cli build <template> <data.json> --output <path> # custom PDF path
tex-cli build <template> <data.json> --engine latexmk # override engine for this invocation only
tex-cli build <template> <data.json> --keep-tex      # preserves the intermediate .tex in output_dir
tex-cli build <template> <data.json> --keep-logs     # copies the engine's .log
tex-cli build <template> <data.json> --no-keep-tex   # inverts the config default
tex-cli build <template> <data.json> --force         # overwrites existing PDF

echo '{"title":"X"}' | tex-cli build <template> -    # JSON via stdin
tex-cli build                                        # interactive menu
```

Important contracts:

- Fully reuses `render` (spec 003) and `compile` (spec 004) — the
  `build.rs` module is a pure orchestrator, with zero new LaTeX
  logic.
- With `--keep-tex`, the intermediate `.tex` is written to
  `output_dir/<template>.tex` **before** compile — so a compile
  failure preserves the file for debugging (FR-15).
- `--engine <name>` does not alter the config; the
  `compiler.engine` key remains immutable after the invocation
  (SC-005).
- No residual artifacts: the render/compile TempDir is cleaned by
  RAII even on failure (SC-006).
- `stdout` reports the PDF path and total pipeline time:
  `PDF generated at <path>. Pipeline (render + compile) took X.Ys.`
- Interactive mode in a TTY: Select template + Text JSON + Confirm
  keep_tex/keep_logs; outside a TTY it delegates to exit 1 pointing
  to the direct CLI.

### Verbosity

Repeatable global `-v` flag on any subcommand:

```bash
tex-cli -v config show      # INFO
tex-cli -vv init            # DEBUG
tex-cli -vvv config set ... # TRACE
```

Logs always go to stderr; stdout stays clean for pipes.

## Exit codes

| Code   | Meaning                                                             |
|--------|---------------------------------------------------------------------|
| 0      | Success                                                             |
| 1      | Generic unclassified error                                          |
| 2      | CLI argument error (via clap)                                       |
| 10     | Config missing                                                      |
| 11     | Corrupted config (invalid TOML)                                     |
| 12     | Unknown key in `config set`                                         |
| 13     | Invalid value (e.g., malformed bool)                                |
| 14     | Permission denied while writing                                     |
| 15     | User aborted an interactive operation                               |
| 20     | Template does not exist (`show`/`remove`/`render`)                  |
| 21     | File passed to `add` is not valid UTF-8                             |
| 22     | `paths.templates_dir` does not exist                                |
| 30     | Tera error during render (undefined var, syntax)                    |
| 31     | Invalid JSON (parse or shape) in `render`                           |
| 40     | LaTeX engine failure during `compile` (log tail on stderr)          |
| 41     | LaTeX engine not installed on PATH                                  |
| 42     | Unsupported engine (outside tectonic/latexmk/pdflatex/xelatex/lualatex) |

## Development

Tests run inside a Debian container with Rust + tectonic:

```bash
docker build -t tex-cli -f docker/Dockerfile .
docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all
```

Standard hygiene before committing:

```bash
docker run --rm -v $(pwd):/src -w /src tex-cli \
  sh -lc "cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings"
```

Details in [`docker/README.md`](docker/README.md).

## Design documentation

- [Project constitution](.specify/memory/constitution.md)
- [v1 spec (config init/show/set)](specs/001-config-init-management/spec.md)
- [Plan](specs/001-config-init-management/plan.md)
- [Quickstart](specs/001-config-init-management/quickstart.md)

## Contributing

Contributions are welcome. Before opening a PR, please read:

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — setup, workflow, commit/branch conventions
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) — Contributor Covenant v2.1
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting (private)
- [`CHANGELOG.md`](CHANGELOG.md) — version history
- [`ROADMAP.md`](ROADMAP.md) — road to `v1.0.0` (blockers, nice-to-haves, deferred scope)
- [`SEMVER.md`](SEMVER.md) — versioning policy and stability contract

Bugs and feature requests via [GitHub issues](https://github.com/JonathanBufon/tex-cli/issues/new/choose).

## License

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([`LICENSE-MIT`](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.
