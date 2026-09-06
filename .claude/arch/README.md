# Architecture notes

Most of what used to live here now lives in the book, where it is built, gated, and read by people who are not us.

| What you want | Where it is |
|---|---|
| How a call runs, what is resident, what can fail, what is swappable | [`book/src/architecture/design.md`](../../book/src/architecture/design.md) |
| The type graph, stored versus computed, what crosses into Python | [`book/src/reference/domain-types.md`](../../book/src/reference/domain-types.md) |
| The eight boundary rules, why each exists, how each is enforced | [`book/src/reference/boundary-rules.md`](../../book/src/reference/boundary-rules.md) |
| Every metric formula and its applicability condition | [`book/src/reference/methodology.md`](../../book/src/reference/methodology.md) |
| What matra does not do yet and what would change that | [`book/src/roadmap.md`](../../book/src/roadmap.md) |
| How a triggered capability gets built | [`book/src/plans/`](../../book/src/plans/) |

## What is still here

[`evolution.md`](evolution.md) records decisions considered and **rejected**: the workspace split, built-in pattern extractors, a four-port model with a separate ingest trait, the async pipeline, the streaming reactor. Each says what was proposed and why it was turned down.

That is the one thing the book cannot carry. The book describes what matra is; a reader deciding whether to propose one of these needs to know it was already argued and lost, and on what grounds. Removing it would take the fence down without reading the sign.

## Why the rest went

Five files here duplicated the pages above: `architecture.md`, `ports.md`, `adapters.md`, `domain-model.md`, `boundary-rules.md`. Duplication is not free. Nothing under `.claude/` is gated, so the copies drifted silently and one of them still carried the project's previous name three months after the rename.

The book is gated by `just docs-floor`: every page reachable from `SUMMARY.md`, every backticked type name resolving in `src/`, every link resolving, a clean build, no em dashes outside quotations, and `book/src/llms.txt` current with `SUMMARY.md`. Architecture prose that lives there gets checked. Architecture prose that lives here does not.

So the rule is: if a fact about the architecture is worth writing down, it goes in the book. This directory holds only what the book has no place for.
