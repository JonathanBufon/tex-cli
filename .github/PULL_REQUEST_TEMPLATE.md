# Pull request

## Summary

<!-- What does this PR change and why? Link to any issue or spec. -->

- Closes #
- Implements `specs/NNN-feature-name/` (if applicable)

## Scope

<!-- Which user stories, tasks, or spec phases are covered? -->

## Test plan

<!-- Commands the reviewer can run to verify. Include expected results. -->

```bash
docker build -t tex-cli -f docker/Dockerfile .
docker run --rm -v $(pwd):/src -w /src tex-cli cargo test --all
```

Expected: all tests pass. Manual smoke steps if applicable:

- [ ] `tex-cli <command> …` produces `<expected output>`
- [ ] Error path: `<command>` exits with code `<N>`

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all` passes inside the Docker container
- [ ] New behavior has tests (happy path + at least one error path)
- [ ] Docs updated (README, CHANGELOG, or spec) where user-facing
- [ ] No new dependencies added, or the addition is justified below

## Constitution alignment

<!-- Which principles does this touch? Mark ✅ PASS, 🟡 DEFERRED, or ❌ N/A. -->

| Principle                                          | Status |
|----------------------------------------------------|--------|
| I. Personal scope — no LaTeX replacement           |        |
| II. Separation of data / template / PDF layers     |        |
| III. Dual interface (CLI + interactive)            |        |
| IV. Pluggable compiler engine                      |        |
| V. Safe file handling                              |        |

## Notes for the reviewer

<!-- Anything non-obvious: tricky tradeoffs, alternatives you considered,
follow-up work you deferred. -->
