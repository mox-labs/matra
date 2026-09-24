# 0009. Feats lookup accessor, Rust-only

- **Status:** Accepted
- **Date:** 2026-08-21
- **Decider(s):** project maintainer; question framed by I7 M2 (typed `feats`)

## Context

`Token.feats` carries CoNLL-U column 6 verbatim as a pipe-separated
string like `Mood=Ind|Number=Sing|Tense=Pres`. Consumers who want one
feature parse the whole string themselves, every time. I7 M3 needs
`Mood` and `VerbForm` lookups, so the access shape has to be decided
before M3 lands.

Two prior positions were recorded in the plan so they would be answered
rather than forgotten. The bidirectional report judged the boundary
correctly placed as-is: CoNLL-U has thousands of feature combinations,
so typing them fully is costly on matra's side and cheap on the
consumer's. The research synthesis disagreed and listed typed access as
one of the five primitives.

## Options considered

### Option A: exhaustive typed enums

A `Feature` enum (or per-key enums) covering the UD feature inventory.

**Pros:**
- Compile-time checked keys and values.

**Cons:**
- The inventory is huge, language-dependent, and grows with treebank
  releases. matra would take a position on the full feature set and pay
  maintenance for it. This is the cost the bidirectional report judged
  not worth paying, and nothing in M3's need requires it.

### Option B: `HashMap<String, String>` per token

Parse `feats` into a map at construction.

**Pros:**
- O(1) repeated lookups.

**Cons:**
- An allocation per token on the hot path for a lookup that happens
  rarely, on strings that hold at most a handful of pairs. The plan's
  rubric forbids a per-token `HashMap` unless benchmarked as
  acceptable, and there is no benchmark showing the linear scan is a
  problem.

### Option C: borrowed lookup accessor

`pub fn feat(&self, key: &str) -> Option<&str>` on `Token`: a linear
scan over `feats.split('|')` using `split_once('=')`, first exact-key
match, value borrowed raw from `feats`.

**Pros:**
- No allocation, no new dependency, no position on the feature
  inventory. Answers M3's need (`feat("Mood")`, `feat("VerbForm")`)
  and nothing more.
- Malformed segments (no `=`) are simply never matched; the empty
  string and the CoNLL-U placeholder `_` contain no `key=value` pair,
  so every lookup on them returns `None` by construction.

**Cons:**
- O(pairs) per lookup. Feats strings hold single-digit pair counts, so
  this is not measurable.
- Multi-valued features (`Case=Nom,Acc`) come back unsplit. That is
  exposure, not a defect: matra reports what UDPipe emitted and does
  not normalise it.

## Decision

We choose Option C. The accessor is Rust-only by design because `feats`
already crosses FFI as a string, so a lookup over it adds no
information to the wire. This is the other half of ADR-0008's
criterion, recorded there and repeated here because future primitives
are judged by it: derivations cross as fields; views over data already
crossing stay methods. `feat` derives nothing, so it stays a method,
and nothing crossing FFI changes, so no `spec/tests/` fixture is added.

The udpipe adapter clones `w.feats` verbatim and stores the empty
string (not `_`) for feature-less tokens, verified against a live
parse. Both spellings yield `None` for every key by construction, and
both are pinned by test so a change in the adapter's behaviour
surfaces loudly.

## Consequences

- Positive: M3 reads `Mood` and `VerbForm` through one audited scan
  instead of ad-hoc string splitting. No wire change, no binding
  change, no dependency change.
- Negative: consumers in other languages still parse the string
  themselves. That is the boundary the bidirectional report judged
  correctly placed; a crust that wants a helper writes a three-line
  one over data it already has.
- Neutral: if a future primitive needs the full inventory typed, that
  is a new decision against real need, superseding this one.

## Validation

Right if M3 lands on this accessor without needing more. Falsified if
profiling ever shows feats lookups hot enough that the linear scan
matters (then benchmark Option B), or if a consumer class needs typed
feature values across FFI (then the derivation would cross as a field
per ADR-0008, not as this view).

## References

- Plan: `book/src/plans/i7-structural-primitives.md` M2, including
  both recorded prior positions.
- [ADR-0008](0008-structural-primitives-are-fields.md): the
  derivations-vs-views criterion this accessor instantiates.
- `src/nlp/udpipe.rs`: the adapter stores `w.feats` verbatim.
