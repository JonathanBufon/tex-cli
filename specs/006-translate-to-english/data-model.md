# Phase 1 — Data Model: Translate All User-Facing Strings to English

**Feature**: `006-translate-to-english`
**Date**: 2026-07-18

## Entities

**None.**

This feature is a **pure text refactor**. It:

- Adds no new data structures.
- Modifies no existing data structures.
- Introduces no new domain entities.
- Does not touch the config schema (`~/.config/tex/config.toml`).
- Does not touch the on-disk format of templates, `.tex`
  intermediates, PDFs, or JSON payloads.

The only enum change is the removal of the string literal `"humano"`
from the accepted values of `--format`, replaced by `"human"`. This
is a **value change**, not a schema change: the surrounding
`clap::ValueEnum` structure (`Format::Human`) already existed.

## Rationale for including this file

The Spec Kit plan template produces `data-model.md` as a Phase 1
output. For features that touch no entities, standard practice is
to include the file with an explicit "N/A" note rather than omit
it — future readers scanning the spec directory see the empty
entity list and know the feature was scoped out of the data-model
concern, rather than wondering whether it was overlooked.

## Cross-references

- [Plan](./plan.md)
- [Research](./research.md) § D-01 (`humano` fate)
- [Contracts](./contracts/cli.md)
