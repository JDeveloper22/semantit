# Contributing to Semantit

This repository follows a trunk-based workflow with strict Git quality gates.

## Branching model

- `main` is protected and cannot receive direct pushes.
- Open short-lived branches from `main` using one of these prefixes:
  - `feat/*`
  - `fix/*`
  - `chore/*`
  - `docs/*`
  - `hotfix/*`
  - `codex/*` (automation/assistant tasks)

## Pull requests

- Every change goes through PR.
- Minimum `1` approval is required.
- Required CI checks must be green:
  - `ci/rust-fmt-clippy`
  - `ci/rust-test`
  - `ci/e2e-battery`
  - `ci/frontend-build`
  - `ci/commit-policy`
- Keep linear history (`rebase and merge`).

## Commit and PR title convention

Semantit uses Conventional Commits for both commit subjects and PR titles.

Examples:

- `feat(core): support new entity matcher`
- `fix(cli): handle missing base file`
- `docs(readme): clarify merge output`
- `chore(ci): add rust cache`

## Local Git defaults (recommended)

```bash
git config --global pull.ff only
git config --global fetch.prune true
git config --global rerere.enabled true
git config --global rebase.autoStash true
```

## Force push policy

- Never force push protected branches.
- If history rewrite is required on a non-protected branch, use:

```bash
git push --force-with-lease
```

## Release tags

- Use annotated tags with format `v0.x.y` (or `v0.x.y-rc.n` for release candidates).
- Create releases only from commits that already passed required CI checks.
