# Semantit Git Playbook

Operational runbook for day-to-day and incident Git workflows.

## 1. Mandatory repository snapshot

Before risky operations, run:

```bash
.agents/skills/git-expert/scripts/git_state_snapshot.sh
```

Confirm:

- clean/expected branch state
- no in-progress sequencers (`rebase`, `merge`, `cherry-pick`, `revert`, `bisect`)
- upstream and ahead/behind status

## 2. Hotfix from dirty working tree

```bash
git stash push -u -m "wip-before-hotfix"
git switch -c hotfix/<ticket> origin/main
# implement fix
git add -A
git commit -m "fix(<scope>): <summary>"
```

After merge:

```bash
git switch -
git stash pop
```

## 3. Divergence sync with explicit policy

```bash
git fetch --prune --prune-tags
upstream=$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}')
git rev-list --left-right --count HEAD..."$upstream"
```

Then pick one policy intentionally:

- linear history: `git rebase "$upstream"`
- preserve local branch shape: `git merge "$upstream"`

## 4. Recovery after bad rebase or force-push

```bash
git reflog -n 100
```

If rebase still running:

```bash
git rebase --abort
```

If bad rebase already finished:

```bash
git branch recovery/pre-rewrite-$(date +%Y%m%d-%H%M%S)
git reset --hard <sha-before-rewrite>
```

For accidental force-push on shared branches:

1. Freeze additional pushes.
2. Recover known-good SHA from reflog.
3. Restore with lease:

```bash
git push --force-with-lease origin <branch>
```

## 5. Release checklist and tag rollback

1. Ensure required CI checks on target commit are green.
2. Create annotated tag:

```bash
git tag -a v0.x.y -m "release v0.x.y"
git push origin v0.x.y
```

3. Publish release notes from GitHub release page.

If a tag is wrong:

```bash
git tag -d v0.x.y
git push --delete origin v0.x.y
# recreate corrected annotated tag and push again
```

## 6. Safety rules

- Prefer `git revert` over rewriting shared/public history.
- Use `git restore` / `git switch` over overloaded `git checkout`.
- Create a backup branch before destructive operations:

```bash
git branch backup/$(date +%Y%m%d-%H%M%S)-$(git rev-parse --short HEAD)
```
