# Dependency parsing

🛠️ This page is a stub. Full content lands in a follow-up iteration.

Dependency parsing reveals the grammatical structure of a sentence by drawing directed arcs between words. Each arc connects a dependent word to its head: the word it grammatically modifies or is governed by. The result is a tree rooted at the main verb of the sentence.

---

## Head-dependent arcs

Every token (except the root) has exactly one head. The arc from token to head carries a label naming the grammatical relation. A few relations that appear frequently in vaani output:

| Relation | Meaning | Example |
|---|---|---|
| `nsubj` | nominal subject | "The committee approved..." ("committee" is `nsubj` of "approved") |
| `obj` | direct object | "approved the proposal" ("proposal" is `obj` of "approved") |
| `aux:pass` | passive auxiliary | "was submitted" ("was" is `aux:pass` of "submitted") |
| `nsubj:pass` | passive nominal subject | "amendments were submitted" ("amendments" is `nsubj:pass` of "submitted") |
| `obl` | oblique nominal | "without debate" ("debate" is `obl` of "approved") |
| `root` | root of the sentence | The main verb; its head position is 0 |

These relation labels are from the Universal Dependencies inventory. vaani uses UDPipe's output directly; the full label set is documented at [universaldependencies.org](https://universaldependencies.org/u/dep/).

---

## Dependency trees vs constituency trees

Constituency parsing (like Penn Treebank notation) groups words into nested phrase brackets: NP, VP, PP. Dependency parsing draws arcs directly between words.

The difference matters for vaani's use case: dependency arcs are directly queryable as typed fields on `Token`. You do not need to navigate a nested phrase structure to ask "what is the subject of this verb?" You look up the `nsubj` arc on the verb's dependents.

---

## How vaani exposes the parse

`Token` fields directly available from the CoNLL-U annotation:

- `form`: the surface word as it appears in text
- `lemma`: the base form (e.g., "approved" becomes "approve")
- `upos`: the Universal POS tag
- `dep`: the dependency relation from this token to its head
- `head`: the 1-based index of the head token in the sentence (0 = root)

`Sentence` exposes a `tree_depth()` method that returns the maximum depth of the dependency tree. Cycle detection uses a visited set; cycles return `usize::MAX` as a sentinel (not a magic depth ceiling).

See [reference/domain-types.md](../reference/domain-types.md) for the complete field definitions.

---

## Planned for this page

A follow-up iteration will add:

- Worked example: the committee sentence parsed step-by-step, with arc labels annotated
- How to traverse the dependency tree using vaani's `Token` fields
- Common patterns: passive detection, agent extraction, subordinate clause detection
- What "sentence depth" means and when it is useful as a document metric
