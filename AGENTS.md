<!-- SPECKIT START -->
Current Spec Kit plan: `specs/001-core-harness/plan.md`.
Use it for project structure, shell commands, and verification expectations.
<!-- SPECKIT END -->

# Project Guidance

- Treat the Rust core as the source of truth for rules, state transitions,
  visibility, and player decision contracts.
- Keep Electron as an adapter over the core library.
- Do not implement hidden-information leaks in player-facing views.
- Add or update deterministic harness coverage for behavior changes.
- Prefer Spec Kit artifacts under `specs/` for non-trivial features.
- Follow `docs/git-commit-conventions.md` for commit messages and commit
  boundaries.
