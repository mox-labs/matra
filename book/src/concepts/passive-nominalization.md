# Passive voice and nominalization

🛠️ This page is a stub. Full content lands in a follow-up iteration.

vaani's `passive_ratio` and `nominalization_ratio` metrics reveal two structural properties of text that signal shifts in how agency and action are encoded.

---

## Passive voice detection

vaani detects passive constructions by looking for the `aux:pass` dependency relation in the parse output from UDPipe. When UDPipe assigns `aux:pass` to an auxiliary verb (e.g., "was" in "The proposal was approved"), the sentence is counted as passive.

**What passive detection captures:** The proportion of sentences containing a UDPipe-recognized passive auxiliary. In the committee example:

```
sentence 2: "Three amendments were submitted by the working group."
  submitted [VERB, root]
  were [AUX, aux:pass]
```

This sentence is passive. Sentence 1 ("The committee approved the proposal") is active. Passive ratio for the two-sentence document: 0.50.

**Non-claims (full list in [reference/methodology.md](../reference/methodology.md#passive-ratio)):**

- Not all passive constructions carry `aux:pass`. Participial passives in complex sentences may not be tagged this way by UDPipe; detection is UDPipe's parse quality, not an exhaustive linguistic definition.
- A high passive ratio does not indicate a writing problem. Legal and scientific writing conventionally uses passive voice.
- vaani detects; your application judges.

---

## Nominalization detection

vaani detects nominalizations using a suffix heuristic applied to tokens with `NOUN` POS tags. Tokens matching suffix patterns associated with derivation from verbs or adjectives are counted as nominalized:

```
-tion    (-ation, -ization, -ification)
-ment
-ness
-ity     (-ality, -ability, -ibility)
-ance / -ence
-ing     (when tagged as NOUN, not VERB)
```

**Non-claims (full list in [reference/methodology.md](../reference/methodology.md#nominalization-ratio)):**

- The suffix heuristic produces false positives. "station," "government," "distance" match the patterns but are not nominalizations derived from verbs in any productive sense.
- The heuristic produces false negatives for nominalizations with unlisted suffixes.
- Nominalization ratio is not a measure of clarity, formality, or writing quality. Heavy nominalization is appropriate in many genres.

---

## Why these metrics together

Passive voice and nominalization both suppress the grammatical subject of an action. "The committee approved the proposal" (active, agentive) vs "The proposal was approved" (passive) vs "Approval was granted" (nominalization). Both the passive ratio and nominalization ratio are relevant to the agentive face of the four faces of voice. See [architecture/four-faces.md](../architecture/four-faces.md).

---

## Planned for this page

A follow-up iteration will add:

- The full suffix list used in the nominalization heuristic
- Worked examples showing true positives and false positives for each metric
- Guidance on interpreting passive and nominalization ratios in specific genres (legal, scientific, journalistic)
