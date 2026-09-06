---
name: structure
summary: Tokens, dependency arcs, the sentence tree, and what each structural field reports.
---

# Structure: tokens, arcs, and the primitives

## The tree the analysis returns

One nested value. Each level owns the level below it and carries its own data.

| Level | Carries |
|---|---|
| document | The section list, plus `vocabulary_ttr`, `nominalization_ratio`, `passive_ratio` |
| section | `heading`, `level`, and its paragraphs in document order |
| paragraph | Verbatim `text`, `in_blockquote`, its sentences, plus three metric slots |
| sentence | `text`, id-sorted `tokens`, and the six structural fields |
| token | The ten CoNLL-U columns plus the derived `is_punct` |

`Paragraph.text` is a verbatim slice of the input. Each non-blockquote paragraph is parsed on its own, so a sentence always belongs to exactly one paragraph and is never matched back to it by comparing text. That is why the chain from a token up to its section always holds.

<!-- needs: model -->

```console
$ matra analyze notes.md --sections
```

## Arcs

Inside a sentence, exactly one token has `head` equal to `0`: the root. Every other token names its governor in `head` and the relation in `dep`. Those two columns are what turn a flat token list into a tree.

For "The system was built by the team." the shipped English model produces:

```text
 id  text     lemma    pos     head  dep
  1  The      the      DET        2  det
  2  system   system   NOUN       4  nsubj:pass
  3  was      be       AUX        4  aux:pass
  4  built    build    VERB       0  root
  5  by       by       ADP        7  case
  6  the      the      DET        7  det
  7  team     team     NOUN       4  obl
  8  .        .        PUNCT      4  punct
```

```text
                    built (4, root, head = 0)
                   /   |   \        \
        system (2)  was (3)  team (7)  . (8)
       nsubj:pass  aux:pass    obl     punct
            |                 /    \
        The (1)          by (5)    the (6)
          det             case      det
```

Words keep their reading order in the token table; height in the drawing is depth in the tree. Everything hangs off `built` directly or through one more hop, so the nesting depth is 2.

Passive detection matches three labels: `nsubj:pass`, `nsubjpass` and `aux:pass`. The first two are the Universal Dependencies and older Stanford spellings of a passive subject; the third is a passive auxiliary. A sentence carrying any of the three counts as passive, once, however many passives it contains.

## Reading arcs without matra's helpers

The tree walkers (`root_token`, `head_of`, `children_of`, `subtree`, `tree_depth`) are Rust methods and do not cross to Python or to JSON. From either of those, walk the token list yourself: the root is the token whose `head` is `0`, the children of a token are the tokens whose `head` equals its `id`, and a subtree is that relation followed transitively. Guard the walk against a cycle; a malformed parse can produce one, and the Rust walker reports it by returning the maximum `usize` rather than by truncating at a magic depth.

## The six structural fields

A structural field is a construction read off the dependency graph and reported as data. Five are computed when the sentence is constructed and the sixth, `hearst_pairs`, at the parse stage. They name an arc shape. They never name a judgment.

| Field | The arc shape | Grounded example |
|---|---|---|
| `negations` | `not`, `never`, `no`, `neither`, `nor` on an `advmod`, `det`, or `cc` arc | "It was never reviewed." gives one `Negation`: cue `never`, head `reviewed` |
| `modals` | one of ten modal lemmas on an `aux` arc or with the `AUX` part of speech | "You must complete the form by Friday." gives one `Modal`: `must` on `complete` |
| `bare_assertion` | root clause finite indicative (`Mood=Ind` on the root or on its `cop`, `aux`, or `aux:pass` child) with no modal governing it | "The committee approved it." is true; "It might have been done." is false |
| `reportings` | a verb governing a clausal complement (`ccomp`), with its subject when the parse has one | "Smith reported that the effect vanished." gives one `Reporting`: verb `report`, complement head `vanished`, subject `Smith` |
| `root_adverbials` | every `advmod` arc into the root | "Reportedly, the deal closed." gives one `RootAdverbial`: `reportedly` |
| `hearst_pairs` | the six Hearst (1992) patterns, matched as dependency arcs | "Animals such as dogs and cats need daily care." gives two `HearstPair` values, both tagged `such_as`, hypernym `animal`, hyponyms `dog` and `cat` |

Three consequences worth holding on to.

**Bare assertion does not consult negation.** The detector reads the root, its finiteness carriers, and the modal list, and nothing else. "The sky is not blue." is a bare assertion that also carries a negation. Combining the two is the caller's job.

**Overlap is intentional.** `never` satisfies both the negation shape and the root-adverbial shape, so it appears in both fields. Neither field is assigning it a meaning, so neither has to win.

**Recall is bounded by decision.** Each detector matches the shapes verified against live parses and no others. Pronominal `neither` as a subject does not fire as a negation. A `ccomp` under an adjective ("I am sure that it works.") is not a reporting, because the construction is defined as verbal. A modal in a subordinate clause is reported in `modals` but does not defeat `bare_assertion`, which reads the root clause only.

## The open classes

Reporting verbs and root adverbials are open word classes, so no lexicon ships: any list would be incomplete while looking authoritative. Every verb that fills the reporting construction is reported and every root-attached adverbial is reported, and selecting the ones that matter is the caller's list. In Rust that is `reportings_in(lexicon)` and `root_adverbials_in(lexicon)`; from JSON or Python it is a filter on `verb_lemma` and `adv_lemma`.

## What the parse layer does and does not do

The parser is UDPipe with the English Web Treebank model, Universal Dependencies 2.5. It performs no named entity recognition, no coreference resolution, and no semantic role labeling. Tags and relations are statistical predictions, not ground truth, and the error rate on text unlike the training corpus is higher than on text like it. Everything downstream inherits those errors.

Two shipped-adapter details that matter when reporting results: `deps` (CoNLL-U column 9) is always `_`, because the binding does not surface enhanced dependencies, and `Sentence.text` is rebuilt from token surface forms rather than sliced out of your input.

## Which text gets parsed

The format decides the segmentation. `notes.md` is decomposed as markdown and the same bytes as `notes.txt` are decomposed as plain text, which gives different paragraphs and therefore different sentences. On markdown, decomposition drops frontmatter, fenced code blocks and table rows, and stops entirely at a heading reading `## References`. Blockquote paragraphs are kept in the tree with `in_blockquote` true and are never parsed.

Report which format the numbers were produced under. It is part of the result.
