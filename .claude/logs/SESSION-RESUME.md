# Session resume

Run the OODA loop below at the start of every session in this directory. Then act.

---

## State (2026-05-25)

- **Branch:** `m2-docsite-ia-restructure` (working surface; do not push to remote `main`; alpha is the pre-0.1.0 stage branch)
- **Last commit:** `8498a39 docs(claude): add Docsite generation pointer to CLAUDE.md + track bootstrap`
- **Status of `book/src/`:** SUMMARY.md placeholder only. Awaiting fresh pipeline-grounded content production.
- **Iteration in flight:** Docsite generation. Pipeline workflow + per-bucket specialist subsets specified in `.claude/logs/bootstrap-fresh-docsite-generation.md`.

---

## OODA

### Observe

Before any action, gather state:

```bash
git status --short
git log --oneline -8
git branch --show-current   # expect: m2-docsite-ia-restructure
find book/src -type f -name '*.md' | sort   # expect: SUMMARY.md only (plus whatever batches have shipped)
just docs-floor             # expect: all 4 gates pass (gate 1 skip-with-warning locally)
lsof -nP -iTCP:3000 -sTCP:LISTEN 2>/dev/null   # is mdbook serve running?
```

Read these in this order:

1. This file (`.claude/logs/SESSION-RESUME.md`) — you are here.
2. `CLAUDE.md` — project posture, conventions, gotchas, the seven boundary rules.
3. `.claude/logs/bootstrap-fresh-docsite-generation.md` — the docsite production protocol.
4. `.rhet/ground-truth.md` and `.rhet/voice.md` — load-bearing inputs every pipeline step reads.

### Orient

Identify which of these states you are in:

| State | Signal | Where to go |
|---|---|---|
| **Fresh start, no batches shipped** | `book/src/` has SUMMARY.md only | Bootstrap "First task": feynman inventio for `introduction.md` |
| **Batches in flight** | `book/src/` has some batches landed; SUMMARY.md references them | Resume from the next bucket in the suggested order (intro → tutorials → concepts → reference → architecture → guides → contributing) |
| **Batch mid-pipeline** | `.rhet/inventio/v2/<bucket>/` or `.rhet/memoria/v2/<bucket>/` has artifacts but no corresponding `book/src/<bucket>/` files | Read the latest agent's return summary; route to the next specialist in the chain |
| **Awaiting user decision** | The last commit message or a comment in `.claude/logs/` flags a pending question | Surface the question to the user; do not act |
| **Floor gates failing** | `just docs-floor` returns non-zero | Diagnose the failure before producing more content |
| **mdbook serve down** | port 3000 has no listener | Restart: `cd book && mdbook serve --hostname 0.0.0.0 --port 3000` (background) |

### Decide

Pick the next action from Orient. Default decisions:

- **Continue the work** if a clear next batch / next specialist is identifiable
- **Restart mdbook serve** if it is down and the user expected live preview
- **Surface to user** if pause-point criteria are met (see bootstrap "Pause points")
- **Do not act** if the state is ambiguous; ask the user

### Act

Execute the chosen action per the bootstrap protocol. After acting, update this file's "State" section so the next session resumes from the new state.

---

## What "complete" means for this iteration

All 31 pages in `.rhet/arrangement/ia-proposal.md` produced via the per-bucket pipeline, integrated into `book/src/`, floor gates green, cross-architecture verification PASS, PR opened against `alpha`, merged via the standard ritual (rationale comment + `gh pr merge --rebase --delete-branch` after explicit user approval).

Until then, the iteration is in flight.

---

## Pause points (when to ask the user)

- Before opening the PR
- Before merging the PR
- If ebert returns a second time on the same batch (the protocol's max-2-returns rule)
- If a structural change is needed that wasn't authorized in the IA proposal
- If a new dependency is added
- If you find an inconsistency between the IA target and the cartography evidence
- If the user has just sent a message — read it first before acting

Do not pause for routine progress reporting. Surface progress at batch boundaries only.

---

## Live serve

```bash
export PATH="$HOME/.cargo/bin:$PATH" && cd book && mdbook serve --hostname 0.0.0.0 --port 3000
```

Background it. Local: http://localhost:3000/. LAN: http://10.0.0.244:3000/.

If port 3000 is held by a stale process, `lsof -nP -iTCP:3000 -sTCP:LISTEN | awk 'NR>1 {print $2}' | xargs -r kill` then restart.

---

## Update protocol

After each batch ships (or after any meaningful state change), update this file's "State" section:

- New "Last commit" hash + subject
- Updated batch progress
- Any new pending decisions

The file is the durable resume surface. Keep it accurate.

---

**End of session resume. Begin OODA above.**
