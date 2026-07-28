## Agent skills

### Issue tracker

Issues are tracked as GitHub Issues in `matsumok/rustruct` via the `gh` CLI; external PRs are not treated as a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Uses the default label vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`) with no renaming. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout — `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Git worktrees

Parallel work uses `git worktree` with worktrees created **as sibling directories** of the repo, launched by hand rather than through Claude Code's built-in worktree tools. See `docs/agents/worktrees.md`.

### Learning workspace

The user is learning the underlying tech stack by implementing this project by hand (写経), guided by the `/teach` skill. The teaching workspace (`MISSION.md`, `RESOURCES.md`, `learning-records/`, `lessons/`, `reference/`, `assets/`) lives at `../rustruct-teach`, a sibling directory — not inside this repo. When the user asks to continue a lesson or teaching session, read that directory first.
