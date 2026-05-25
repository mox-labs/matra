# Dependency parsing

A dependency parse answers one question about every word in a sentence: what does this word grammatically depend on?

---

## Head-dependent relations

Every sentence has a root token: a word that nothing else governs (its `head` id is 0). Every other token has a head: a single word it depends on. The dependency is labeled with the grammatical relation it represents. That label is the `dep` field on a `Token`.

Take the sentence: *The committee approved the proposal without debate.*

```
approved [VERB, root, head=0]
  ├── committee [NOUN, nsubj, head=approved]
  │     └── The [DET, det, head=committee]
  ├── proposal [NOUN, obj, head=approved]
  │     └── the [DET, det, head=proposal]
  └── debate [NOUN, obl, head=approved]
        └── without [ADP, case, head=debate]
```

`committee` depends on `approved` with relation `nsubj` (nominal subject). `proposal` depends on `approved` with relation `obj` (direct object). The verb is the structural center of the sentence; the nouns and their modifiers hang from it.

This structure is why vaani can tell your application who is acting: the token with `dep = "nsubj"` is the grammatical agent. The token with `dep = "obj"` is the patient. No pattern matching; no regular expression over surface text. The relation is a field on a struct.

---

## Passive voice and the structure shift

Now the second sentence: *Three amendments were submitted by the working group.*

```
submitted [VERB, root, head=0]
  ├── amendments [NOUN, nsubj:pass, head=submitted]
  │     └── Three [NUM, nummod, head=amendments]
  ├── were [AUX, aux:pass, head=submitted]
  └── group [NOUN, obl, head=submitted]
        ├── by [ADP, case, head=group]
        ├── the [DET, det, head=group]
        └── working [ADJ, amod, head=group]
```

The label `nsubj:pass` (passive nominal subject) marks that `amendments` fills the subject position but receives the action rather than performing it. The label `aux:pass` on `were` marks the passive auxiliary. The group that did the submitting appears as an oblique (`obl`), not a subject.

These two sentences describe the same kind of event. The dependency parse makes the difference explicit in a form your code can check directly. If your application needs to know whether agency is stated or obscured, these two label sets are the answer.

---

## Relation labels

vaani uses the Universal Dependencies (UD) label set. A selection of labels relevant to most applications:

| Label | What it marks |
|---|---|
| `nsubj` | Nominal subject (active clause) |
| `nsubj:pass` | Nominal subject of a passive clause |
| `obj` | Direct object |
| `obl` | Oblique argument (prepositional phrases, temporal NPs) |
| `aux` | Auxiliary verb |
| `aux:pass` | Passive auxiliary (`was`, `were`, `been`) |
| `det` | Determiner |
| `amod` | Adjectival modifier |
| `compound` | Compound word component |
| `conj` | Conjunct (in a coordination) |
| `dep` | Unspecified or unclassifiable dependency |
| `root` | Root of the sentence (not a relation to another token) |

The full UD relation inventory is documented at universaldependencies.org. vaani does not filter or remap these labels; what UDPipe produces is what you receive in the `dep` field.

---

## Dependency vs phrase-structure parsing

An alternative way to represent sentence structure is phrase-structure (constituency) parsing, which groups tokens into nested constituents: noun phrases, verb phrases, clauses. Traditional grammar diagrams use this approach.

Dependency parsing uses a different primitive: the relation between two tokens rather than the group of tokens. Both representations carry the same structural information, but dependency structure is easier to query when the question is "which token governs which" rather than "what constituency does this span belong to."

For the kinds of questions vaani serves (who is the subject? is this sentence passive? what does this noun modify?), dependency structure gives direct answers. You check one field on one token rather than traversing a constituency tree.

---

## What vaani gives you

The `Sentence` type gives you the full dependency graph. Given a sentence:

- `root_token()` finds the token with `head = 0`.
- `children_of(id)` finds all tokens whose head is the given id.
- `head_of(id)` finds the token that governs the given id.
- `subtree(id)` collects the entire subtree rooted at a given token.
- `tree_depth()` returns the maximum depth of the dependency tree (returns `usize::MAX` on a malformed cyclic parse).

All of these work directly on the `tokens` vector. There is no separate tree data structure to build.

If you are building an application that needs to walk the parse tree (finding all noun phrases that modify a particular verb, or collecting all passive subjects in a document), these five methods are the complete traversal vocabulary. The domain types reference documents their signatures; the dependency tree you traverse is the same tree UDPipe produced.

[Domain types reference](../reference/domain-types.md) documents every field and method on `Sentence` and `Token`. [POS tags and lemmatization](./pos-lemmas.md) covers the `pos` field.
