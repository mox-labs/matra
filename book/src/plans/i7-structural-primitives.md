# I7 — Five structural primitives

**Boundary:** rule-substrate. Lands before any `src/rules/` design work.

**Source of scope:** an internal research synthesis dated 2026-05-23 naming five concrete primitives, cross-walked against a bidirectional report dated 2026-05-20 that maps the surveyed literature onto matra's actual surface. Neither is public; the grounding each cites is.

---

## Why this iteration exists

The research pass produced a list of five additions and declared the rule-evaluation deferral fired. Neither the list nor the verdict reached this repository. Three months of the chain from research to roadmap was lost, and the loss was found only when a session re-derived four of the five by probing the parser by hand.

The five are not features. They are the primitives a rule engine would be written in terms of, which is why they land before `Rule` and `Predicate` get designed. Designing the rule vocabulary first would be anticipating the shape rather than pulling it from use, which is the thing the trigger condition was written to prevent.

There is a second reason, and it is stronger because it comes from inside the repository rather than from research.

`Sentence::is_passive` is a method. Methods do not cross FFI. So `python/matra/cli.py:40` re-implements passive detection over raw tokens, in Python, against the same parse the Rust method already read. matra's own crust duplicates matra's own primitive. Every consumer in every language does the same today, and will do the same for negation, modality and evidentiality unless this iteration settles the question.

**That question is the spine of this iteration: are structural primitives fields or methods?** It is not settled here. M1 settles it with a real primitive in hand, and M2 through M5 inherit the answer.

---

## Milestones

Ordered by dependency, then by size. No milestone starts before the previous one meets its rubric.

| Milestone | Primitive | Why here |
|---|---|---|
| M1 | Negation on `Sentence` | Smallest. Mirrors `is_passive` exactly, so it is the cheapest place to settle field-vs-method |
| M2 | Typed `feats` accessor | Unblocks M3, which reads `Mood` and `VerbForm` from the feats string |
| M3 | Modal classification | Closed word class, so the lexicon is small and enumerable |
| M4 | Evidentiality markers | Open class. Needs M3's lexicon mechanism to already exist |
| M5 | Hearst patterns | Largest. Multi-token, multi-arc, and the only one that spans clauses |

---

## M1 — Negation

**What lands.** Negation scope on `Sentence`, derived from the dependency graph. The signal verified present in the parse: `not` appears as `advmod` with `lemma == "not"` attached to the verb it negates. `never`, `no`, `neither`, `nor` follow the same shape.

**The decision this milestone makes.** Field or method.

- *Method* matches `is_passive`, costs nothing at parse time, and stays invisible to Python and to any future WASM crust. Consumers re-implement, as `cli.py` already does.
- *Field* crosses FFI by construction and is computed once, but is stored on every sentence whether or not the consumer wants it, and `Sentence` currently has no metric slots at all.

Whichever wins, it binds M2 through M5 and should be recorded as an ADR, because it is the first time matra decides how a derived structural fact reaches a non-Rust consumer.

**Steps.**
1. ADR: structural primitives, field or method. Decide with the `cli.py` duplication as the primary evidence.
2. Implement per the ADR in `src/domain.rs`, beside `is_passive`.
3. Unit tests over hand-built `Sentence` fixtures, no model needed, matching the existing `is_passive` test style.
4. If field: extend `python/matra/types.py`, `_core.pyi`, and add a conformance fixture under `spec/tests/`.
5. Delete the hand-rolled passive detection in `python/matra/cli.py` if and only if the ADR made primitives crossable. If it did not, leave it and note why in a comment, because the duplication is then intentional.

**Rubric.**

| Dimension | Pass |
|---|---|
| Correctness | Negation detected for `not`, `never`, `no`, `neither`. No false positive on `nothing` used as a subject or on `cannot` split by the tokenizer |
| Boundary | `domain.rs` gains no dependency. Rule 1 holds |
| Cross-language | Either the primitive reaches Python and a conformance fixture proves it, or the ADR states why it does not and what consumers do instead |
| Substrate discipline | Reports scope, not judgement. No "hedged", no "weak", no polarity verdict |

---

## M2 — Typed `feats`

**What lands.** Typed access to CoNLL-U column 6, which `Token.feats` carries today as a pipe-separated string like `Mood=Ind|Number=Sing|Tense=Pres`. Consumers parse that string themselves, every time.

**The known counter-argument, recorded so it is answered rather than forgotten.** `bidirectional/vaani.md` judged this boundary correctly placed: CoNLL-U has thousands of feature combinations, so typing them fully is costly on matra's side and cheap on the consumer's. The synthesis disagreed and listed it. Both are in scope for the ADR.

The resolution likely sits between them: a lookup accessor (`feat("Mood")` returning `Option<&str>`) rather than an exhaustive enum. That answers M3's need without matra taking a position on the full feature inventory.

**Steps.**
1. ADR resolving the two positions. Name the accessor shape.
2. Implement on `Token` in `src/domain.rs`. No allocation on the hot path if avoidable.
3. Property test: round-trip against a corpus of real `feats` strings drawn from an actual parse, including the empty string and single-feature cases.

**Rubric.**

| Dimension | Pass |
|---|---|
| Correctness | Handles empty feats, single feature, malformed input without panic |
| Boundary | No new dependency. No `HashMap` allocated per token unless benchmarked as acceptable |
| Cross-language | Python already receives `feats` as a string, so this may be Rust-only by design. State that explicitly |
| Substrate discipline | Exposes what UDPipe emitted. Does not normalise, infer, or fill in absent features |

---

## M3 — Modal classification

**What lands.** Classification of modal auxiliaries into the epistemic / deontic / dynamic distinction, plus the structural discriminator for bare assertion.

**Verified signal.** `might`, `must`, `may` each appear as `aux` with their own lemma. A root carrying `Mood=Ind` with no modal `aux` is the bare assertoric case. Both were confirmed against a live parse.

**The hard part is not the parse, it is the lexicon.** English has roughly a dozen modal auxiliaries and most are ambiguous across categories. `must` is deontic in "must be completed by Friday" and epistemic in "that must be the reason". matra cannot disambiguate those without context it does not model.

**So the primitive reports the modal and its structural position, and does not resolve the category.** A consumer with document context resolves it. Anything else would be matra taking an interpretive position, which is the boundary the whole design rests on.

**Steps.**
1. Enumerate the closed class from the UD English treebank tag set, not from intuition.
2. Expose the modal auxiliaries governing each clause, with their lemma and their head.
3. Expose the bare-assertion discriminator (root `Mood=Ind`, no modal `aux`).
4. Tests over fixtures covering all three of potential, assertoric and directive surface forms.

**Rubric.**

| Dimension | Pass |
|---|---|
| Correctness | Every modal in the closed class is found. Multi-auxiliary chains (`might have been`) report all of them |
| Boundary | Lexicon is data, not logic. It does not import anything |
| Cross-language | Inherits M1's decision |
| Substrate discipline | **Reports the modal. Does not assign epistemic/deontic/dynamic.** Ambiguity is surfaced, not resolved |

---

## M4 — Evidentiality markers

**What lands.** Detection of evidential marking: reported speech, perception verbs, hearsay adverbs.

**Verified signal.** `Reportedly` attaches as sentence-scope `advmod` with lemma `reportedly`. Reporting verbs take a `ccomp`, and the construction `X ―nsubj→ claims` with `supports ―ccomp→ claims` was confirmed on a live parse.

**Open class, so this milestone has a different risk from M3.** There is no closed list of evidential adverbs or reporting verbs. Whatever list ships will be incomplete, and an incomplete list that looks authoritative is worse than no list. The milestone should ship the *construction detector* (sentence-scope `advmod`, reporting-verb-plus-`ccomp`) and treat the lexicon as consumer-supplied.

**Steps.**
1. Detect the reporting construction structurally: a verb with a `ccomp` complement, plus its `nsubj`.
2. Detect sentence-scope `advmod` attaching to the root.
3. Take the lexicon as a parameter rather than embedding one.
4. Tests covering self-attribution (`We show that`), other-attribution (`Smith reported`), and impersonal (`These results suggest`).

**Rubric.**

| Dimension | Pass |
|---|---|
| Correctness | The construction is found regardless of which verb fills it |
| Boundary | No embedded word list in `domain.rs`. If a default lexicon ships it lives in `stopwords.rs`-style data, and rule 1 holds |
| Cross-language | Inherits M1's decision |
| Substrate discipline | Detects the construction. Does not decide whether the source is credible or the claim hedged |

---

## M5 — Hearst patterns

**What lands.** Detection of the classical lexico-syntactic hypernymy patterns: "X such as Y", "X including Y", "Y and other X", "X, especially Y".

**Why last.** It is the only primitive that spans a clause rather than reading a single arc, and the only one whose output is a *pair* of spans rather than a flag on a sentence. It will exercise whatever M1 decided in the way most likely to break it.

**Steps.**
1. Implement the six original Hearst 1992 patterns as dependency-arc patterns, not as regex over surface text. Regex is what the literature used in 1992 because parsers were not available; matra has a parser.
2. Return span pairs (hypernym, hyponym) referencing token ids so provenance holds.
3. Tests over each of the six patterns plus at least two known-hard negatives.

**Rubric.**

| Dimension | Pass |
|---|---|
| Correctness | All six patterns detected. Precision favoured over recall: a missed pattern is acceptable, a false hypernymy pair is not |
| Boundary | Lives in a new module, not in `domain.rs`. Imports only `domain` |
| Cross-language | Span pairs cross as data. Fixture in `spec/tests/` |
| Substrate discipline | Returns candidate pairs with the pattern that matched. Does not build a taxonomy or assert the relation is true |

---

## Validation

Applies at every milestone landing, in addition to that milestone's rubric.

1. `cargo test` count strictly greater than the previous milestone. No milestone deletes a test.
2. `cargo test --no-default-features` passes. Boundary rule 6.
3. `cargo test --features cli` passes.
4. `scripts/check-boundaries.sh` passes. Rules 3, 4, 8.
5. `just docs-floor` passes.
6. If the milestone changed anything crossing FFI, `spec/tests/` gains a fixture and every crust runs it.

Known trap, from this session: `cargo test --all-features` fails to link because the `python` feature builds against libpython with symbols left undefined. Do not treat that as a regression, and do not add it to any gate.

---

## Acceptance gate

The iteration is done when all five primitives ship, and:

- an ADR records the field-versus-method decision with its reasoning;
- if primitives cross FFI, `python/matra/cli.py` no longer re-implements passive detection, and a conformance fixture proves at least one primitive agrees across crusts;
- no primitive assigns an interpretive category. Every one reports structure and leaves the reading to the caller;
- `ROADMAP.md`'s rule-evaluation entry is updated with what the five primitives revealed about the shape `Rule` and `Predicate` should take.

That last point is the actual output of this iteration. The five primitives are worth having on their own, but the reason to build them before designing the rule vocabulary is that they are what the vocabulary must describe.

---

## Risks

**The interpretive line.** M3 and M4 are the closest matra has come to interpretation. The rubric line "reports the modal, does not assign the category" is the guard, and it will feel like under-delivery. It is not. The moment matra decides `must` is deontic, it has taken a position on meaning, and the substrate argument that justifies the whole architecture is gone.

**Lexicon rot.** M4 and M5 need word lists. Any list that ships will be incomplete and will look authoritative. Prefer consumer-supplied lexicons and a structural detector; ship a default only where the class is genuinely closed, as in M3.

**Sentence segmentation upstream.** UDPipe splits `Smith et al. reported a similar finding` into two sentences at the period in `et al.`, verified this session. Every sentence-scoped primitive here inherits that. It is not this iteration's to fix, but M4 in particular will show it, since attribution lands in a different sentence from its reporting verb. Record it; do not paper over it.

**Scope creep toward coreference.** The research names coreference as the bridge between sentence-internal and document-level extraction, and it will look adjacent to M4. It is not in scope. The same synthesis places it above matra with SRL and NLI, on the argument that matra stops at Marr's algorithmic level.
