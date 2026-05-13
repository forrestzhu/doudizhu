# Git Commit Conventions

This repository uses a pragmatic Conventional Commits style.

## Format

```text
<type>(<scope>): <summary>

<body>

<footer>
```

Only the first line is required. Keep it under 72 characters when practical.

## Types

- `feat`: user-visible feature or new capability
- `fix`: bug fix or behavior correction
- `docs`: documentation-only change
- `test`: add or update tests
- `refactor`: code restructuring without behavior change
- `perf`: performance improvement
- `style`: formatting or lint-only change
- `build`: build system, dependency, or tooling change
- `ci`: CI workflow change
- `chore`: maintenance that does not fit another type
- `revert`: revert a previous commit

## Scopes

Use a short area name when it adds clarity:

- `core`: game engine, state transitions, cards, rules
- `rules`: hand classification and comparison logic
- `visibility`: player view and hidden-information boundaries
- `decision`: player policy contracts or bots
- `arena`: CLI simulations and regression scenarios
- `electron`: desktop UI adapter
- `spec`: Spec Kit artifacts under `specs/`
- `docs`: architecture/design documents

If no scope is useful, omit it:

```text
docs: add git commit conventions
```

## Summary

Write the summary in imperative mood, lower-case after the type:

```text
feat(arena): add seeded self-play runner
fix(rules): reject unmatched pair responses
docs(spec): describe player visibility contract
```

Avoid vague summaries:

```text
update stuff
fix bug
changes
```

## Body

Use the body when the reason or tradeoff is not obvious. Explain what changed and
why, not every file touched.

```text
feat(visibility): add player-specific game views

Expose own hand, public hand counts, turn history, and current previous play.
Keep opponent hands out of the default view so human, bot, and LLM players share
the same hidden-information contract.
```

## Breaking Changes

Mark incompatible contract changes with `!` and a footer:

```text
feat(decision)!: require policies to return structured decisions

BREAKING CHANGE: decision policies no longer return raw card lists.
```

## Issue References

Use footers for issue references when relevant:

```text
fix(rules): compare bombs by primary rank

Closes #12
```

## Commit Boundaries

- Keep one commit focused on one logical change.
- Do not mix generated build output with source changes.
- Include tests or arena updates in the same commit as behavior changes when
  they prove that behavior.
- Do not include local IDE state or personal machine configuration.

## Verification Before Commit

Choose the narrowest command set that proves the change:

- Rust behavior change: `cargo fmt --check && cargo test && cargo clippy -- -D warnings`
- Arena behavior change: add `cargo run --bin arena -- --games 10 --seed 42`
- Docs-only change: review rendered Markdown or run a lightweight text check

Record important verification in the PR or final handoff, not necessarily in the
commit message.
