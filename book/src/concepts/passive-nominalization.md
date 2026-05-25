# Passive voice and nominalization

vaani computes two structural signals at the document level: passive ratio and nominalization ratio. Both are heuristics over the dependency parse. This page explains exactly what they detect and where the heuristics break down.

---

## Passive voice detection

vaani detects passive voice by checking dependency labels, not surface patterns.

A sentence is counted as passive when any of its tokens carries one of these `dep` labels:

- `nsubj:pass`: the nominal subject of a passive clause
- `nsubjpass`: an older UD label for the same relation (kept for compatibility with some UDPipe outputs)
- `aux:pass`: the passive auxiliary (`was`, `were`, `been`)

From the committee example:

```
Three amendments were submitted by the working group.
  amendments  [dep="nsubj:pass"]
  were        [dep="aux:pass"]
  submitted   [dep="root"]
  group       [dep="obl"]
```

Both `amendments` (the grammatical subject receiving the action) and `were` (the passive auxiliary) independently trigger the detection. Either label alone is sufficient.

**Why labels, not patterns.** Surface patterns (searching for "was/were + past participle") miss many passive constructions: "The proposal, having been submitted..." or "Submitted by the committee, the proposal..." Both are passive; neither matches a simple "was + verb" pattern. Dependency labels are assigned by the parser based on grammatical structure, so they catch reformulations that surface matching misses.

**The passive ratio.** The `Document::passive_ratio()` method is the count of passive sentences divided by the total sentence count. A ratio of 0.25 means one sentence in four contains a passive construction.

---

## What passive detection does not claim

Passive voice is not a quality problem. It is a structural choice with different implications in different contexts.

Scientific writing uses passive voice for legitimate reasons: "The experiment was conducted" removes the agent from the description of procedure, where the agent is irrelevant. Legal documents use passive voice to focus on the action rather than attribution. Journalism uses passive voice when the actor is unknown.

vaani's passive ratio is a signal for your application to interpret, not a quality score. A ratio of 0.6 in a legal document is expected. A ratio of 0.6 in a persuasive essay might indicate a different authorial choice. The metric does not know the genre.

---

## Nominalization detection

Nominalization is the use of a noun derived from a verb or adjective: "investigation" (from "investigate"), "improvement" (from "improve"), "darkness" (from "dark").

vaani identifies nominalization by suffix matching on NOUN tokens. The suffixes checked are:

- `-tion` (investigation, construction, solution)
- `-ment` (improvement, assessment, movement)
- `-ness` (darkness, effectiveness, readiness)
- `-ity` (probability, clarity, complexity)
- `-ence` (existence, reference, evidence)
- `-ance` (performance, compliance, reliance)

A token matches when `pos == "NOUN"` and the lowercased surface form ends with one of these suffixes.

The nominalization ratio is the count of matching tokens divided by the total count of non-punctuation lemmas across all sentences.

---

## Where the heuristic breaks down

The suffix check is a heuristic. Several categories of false positives exist:

**Non-nominalized nouns with these endings.** Nation, station, portion, section end in `-tion` but are not derived from verbs by nominalization. Science, patience, sentence, presence end in `-ence`/`-ance` but are not straightforwardly nominalizations of verbs. These will be counted in the ratio.

**Proper nouns.** Washington, Florence, Provence end in `-ence`/`-ance` and have `pos == "NOUN"` (or `PROPN`, but UDPipe sometimes assigns `NOUN`). These will be counted.

**Domain vocabulary.** In chemistry or pharmacology, many common nouns end in nominalization-like suffixes (substance, instance, distance) without being nominalized in the relevant sense.

The ratio is best interpreted comparatively: a document that scores 0.15 on nominalization ratio relative to a baseline of 0.08 for similar texts in the same domain is using more nominal forms. Without a domain baseline, the raw number is harder to interpret.

---

## Why these metrics exist

Passive voice and nominalization are two of the four faces of structural voice that vaani's architecture exposes. Passive ratio is an agentive signal: it measures how often the text obscures who is acting. Nominalization ratio is a structural signal: high nominalization can make prose denser and more abstract by hiding actions inside nouns.

Neither metric is a style judgment. They are instruments: they make a structural property of the text visible in a form your application can act on. What to do with that visibility is yours to decide.

[Affordances](./affordances.md) covers the full metric set. [Dependency parsing](./dependency-parsing.md) explains the `dep` labels that passive detection relies on. [Methodology](../reference/methodology.md) is the canonical reference for all metric definitions.
