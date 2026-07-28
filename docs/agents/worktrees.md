# Git worktrees: local workflow

How to run parallel work on this repo using `git worktree` on a local machine, and where to launch Claude Code.

The short version: **create worktrees as sibling directories by hand, then launch Claude inside one.** Do not use Claude Code's built-in worktree tools for this repo — see "Why not the built-in tools" below.

## Placement rule: siblings, not nested

`AGENTS.md` points the learning workspace at **`../rustruct-teach`** — a sibling of the repo, referenced by a relative path. That path has to keep resolving from wherever the session runs, so worktrees must sit at the same depth as the repo itself.

```
~/projects/
├── rustruct/              # main clone
├── rustruct-teach/        # learning workspace (see AGENTS.md)
└── rustruct-topcoat/      # worktree — same depth, so ../rustruct-teach still resolves
```

A worktree nested inside the repo (for example under `.claude/worktrees/`) breaks this: `../rustruct-teach` would resolve to a sibling of the worktree, not of the repo.

## Creating a worktree

For an existing branch:

```
cd ~/projects/rustruct
git worktree add ../rustruct-topcoat poc/topcoat
```

For a new branch off the current default:

```
git fetch origin
git worktree add -b <new-branch> ../rustruct-<topic> origin/main
```

Name the directory after the work, not the branch, when they differ — the directory is what you'll `cd` into.

## Launching Claude

```
cd ../rustruct-topcoat
claude
```

The session picks up `AGENTS.md`, `CONTEXT.md`, and `.claude/skills/` from the worktree's own checkout, so skills and conventions follow the branch. A branch that predates a skill won't have it — rebase or merge if that matters.

## Do not share `CARGO_TARGET_DIR`

Sharing one target directory across worktrees looks like an easy win and is counterproductive here: `rust-toolchain.toml` is pinned per branch (`poc/topcoat` pins 1.97.1 for topcoat's MSRV, other branches do not), so a shared target directory forces a full rebuild every time you switch between worktrees on different toolchains.

Leave each worktree with its own `target/`. The first build is expensive — topcoat pulls in ~31 crates — but it happens once per worktree.

`target/` is already gitignored, so nothing leaks between worktrees.

## Cleaning up

```
git worktree remove ../rustruct-topcoat     # refuses if the tree is dirty
git worktree list                           # verify
git worktree prune                          # drop stale registrations
```

Removing the directory with `rm -rf` alone leaves a stale registration behind; `git worktree prune` clears it.

## Why not the built-in tools

Claude Code has `EnterWorktree` / `ExitWorktree`, and subagents can take `isolation: "worktree"`. Both create worktrees **under `.claude/worktrees/`**, which violates the placement rule above, and `EnterWorktree` defaults to branching fresh from `origin/<default>` rather than checking out an existing branch.

Leave those to background subagents, where the isolation is short-lived and the teaching workspace is not in play. For hand-driven work, use `git worktree add` as described here.

`.claude/worktrees/` is gitignored on branches that carry the rule — keep it that way; those trees are transient and must never be committed.
