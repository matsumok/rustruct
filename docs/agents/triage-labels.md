# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.

## Loop labels

Two more labels this repo uses on top of the triage roles. The implement loop (the local skill `.claude/skills/implement-loop/`, started with `/implement-loop ui` or `calc`) reads them.

| Label         | On        | Meaning |
| ------------- | --------- | ------- |
| `ui`          | Issue, PR | The work needs a visual check (screenshots at phone and PC widths). Put it on every Issue that touches what a page looks like, even when the Issue also changes the calculation core. The `ui` loop grabs these; the `calc` loop grabs every `ready-for-agent` Issue without it. The loop copies the label onto the PR it opens, so the PR stays with the same loop. Whoever files the Issue (for example `/to-tickets`) applies it, and anyone who finds a `ready-for-agent` Issue missing it adds it before a loop picks the Issue up. |
| `owner-merge` | PR        | The PR strays from its Issue: it changes an existing value, expectation or tolerance, or weakens a comparison test, in a way neither the Issue body nor the owner's comments describe. The owner merges it, not the agent (`CODING_STANDARDS.md` is the source of truth). |
