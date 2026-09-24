# Concepts

## The document tree

Analysis returns one nested value. Each level owns the level below it and carries its own data.

| Level | Type | Carries |
|---|---|---|
| document | `Document` | the section list, plus `vocabulary_ttr`, `nominalization_ratio`, `passive_ratio` |
| section | `Section` | `heading`, `level`, and its paragraphs in document order |
| paragraph | `Paragraph` | verbatim `text`, `in_blockquote`, its sentences, plus `readability_grade`, `lexical_density`, `compression_ratio` |
| sentence | `Sentence` | verbatim `text`, id-sorted tokens, and the structural primitive fields |
| token | `Token` | the ten CoNLL-U columns (`text`, `lemma`, `pos`, `xpos`, `feats`, `head`, `dep`, `deps`, `misc`, plus `id`) and the derived `is_punct` |

`Paragraph.text` and `Sentence.text` are verbatim slices of the input. Each non-blockquote paragraph is parsed on its own, so a sentence always belongs to exactly one paragraph, rather than being matched back by text.

Inside a sentence, exactly one token has `head` equal to `0`: the root. Every other token names its governor in `head` and the relation in `dep`. Those two columns are what turn a flat token list into a tree.

<svg class="mx-dt" role="img" aria-label="Dependency tree for the sentence: The system was built by the team" viewBox="0 0 720 305" width="720" height="305" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Dependency tree for: The system was built by the team</title>
<style>
.mx-dt text{fill:currentColor;font-family:inherit}
.mx-dt .w{font-size:13px;text-anchor:middle}
.mx-dt .m{font-size:9.5px;text-anchor:middle;opacity:.55;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-dt .rel{font-size:10px;text-anchor:middle;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;paint-order:stroke;stroke:var(--bg,transparent);stroke-width:3.5px;stroke-linejoin:round}
.mx-dt .ax{font-size:10px;text-anchor:end;opacity:.55}
.mx-dt .nt{font-size:9.5px;opacity:.55}
.mx-dt .e{stroke:currentColor;fill:none;opacity:.5;stroke-width:1.1px}
.mx-dt .p{opacity:1;stroke-width:2.4px}
.mx-dt .s{stroke:currentColor;opacity:.2;stroke-width:1px}
.mx-dt .d{fill:currentColor}
</style>
<text class="ax" x="76" y="64">root</text>
<text class="ax" x="76" y="134">depth 1</text>
<text class="ax" x="76" y="204">depth 2</text>
<line class="e p" x1="358" y1="60" x2="186" y2="130"/>
<line class="e p" x1="358" y1="60" x2="272" y2="130"/>
<line class="e" x1="358" y1="60" x2="616" y2="130"/>
<line class="e" x1="186" y1="130" x2="100" y2="200"/>
<line class="e" x1="616" y1="130" x2="444" y2="200"/>
<line class="e" x1="616" y1="130" x2="530" y2="200"/>
<line class="s" x1="100" y1="200" x2="100" y2="238"/>
<line class="s" x1="186" y1="130" x2="186" y2="238"/>
<line class="s" x1="272" y1="130" x2="272" y2="238"/>
<line class="s" x1="358" y1="60" x2="358" y2="238"/>
<line class="s" x1="444" y1="200" x2="444" y2="238"/>
<line class="s" x1="530" y1="200" x2="530" y2="238"/>
<line class="s" x1="616" y1="130" x2="616" y2="238"/>
<circle class="d" cx="100" cy="200" r="3.2"/>
<circle class="d" cx="186" cy="130" r="3.2"/>
<circle class="d" cx="272" cy="130" r="3.2"/>
<circle class="d" cx="358" cy="60" r="4.4"/>
<circle class="d" cx="444" cy="200" r="3.2"/>
<circle class="d" cx="530" cy="200" r="3.2"/>
<circle class="d" cx="616" cy="130" r="3.2"/>
<text class="nt" x="370" y="50">root, head = 0</text>
<text class="rel" x="238" y="109">nsubj:pass</text>
<text class="rel" x="328" y="85">aux:pass</text>
<text class="rel" x="487" y="95">obl</text>
<text class="rel" x="143" y="165">det</text>
<text class="rel" x="530" y="165">case</text>
<text class="rel" x="573" y="165">det</text>
<text class="w" x="100" y="252">The</text>
<text class="w" x="186" y="252">system</text>
<text class="w" x="272" y="252">was</text>
<text class="w" x="358" y="252">built</text>
<text class="w" x="444" y="252">by</text>
<text class="w" x="530" y="252">the</text>
<text class="w" x="616" y="252">team</text>
<text class="m" x="100" y="266">1 DET</text>
<text class="m" x="186" y="266">2 NOUN</text>
<text class="m" x="272" y="266">3 AUX</text>
<text class="m" x="358" y="266">4 VERB</text>
<text class="m" x="444" y="266">5 ADP</text>
<text class="m" x="530" y="266">6 DET</text>
<text class="m" x="616" y="266">7 NOUN</text>
<text class="nt" x="30" y="292">heavier lines are the arcs is_passive() matches</text>
</svg>

Words keep their reading order left to right; height is depth in the tree. `built` is the root, everything else hangs off it directly or through one more hop, so `tree_depth()` returns 2.

## The four output tiers

The pipeline produces them in this order, and each is a different kind of result.

1. **Structure**, from `Engine::annotate`. Sections, paragraphs, sentences, tokens, and the structural primitive fields. Derived from the parse, checkable against the source bytes.
2. **Measures**, from `Engine::compose`. Six `Option<f64>` slots filled by the metric suite: three per paragraph, two per document, plus the materialized `passive_ratio`. `None` means not computed, which is distinct from a computed zero.
3. **Extraction**, from standalone functions over a sentence slice: `tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`. They return `ScoredSentence` and `Keyphrase` values, never fields on the tree.
4. **Semantic**, behind the `model2vec` feature. `embed_and_cluster` returns a `SemanticClusters` value carrying the model hash and the threshold you supplied. This tier depends on the model, so by design it never becomes a field on `Document` or `Sentence`.

## Structural primitives

A structural primitive is a construction read off the dependency graph and reported as a field. Six of them are computed once and travel with the sentence into every language binding: five at `Sentence` construction, and `hearst_pairs` at `Engine::annotate`, because its detector lives outside the domain (a `Sentence` built by hand has an empty `hearst_pairs`). They name an arc shape. They never name a judgment.

| Field | The arc shape | Grounded example |
|---|---|---|
| `negations` | `not`, `never`, `no`, `neither`, `nor` on an `advmod`, `det`, or `cc` arc | `It was never reviewed.` gives one `Negation`: cue `never`, head `reviewed` |
| `modals` | one of ten modal lemmas on an `aux` arc or with the `AUX` part of speech | `You must complete the form by Friday.` gives one `Modal`: `must` on `complete` |
| `bare_assertion` | root clause finite indicative (`Mood=Ind` on the root or on its `cop`, `aux`, or `aux:pass` child) with no modal governing it | `The committee approved it.` is `true`; `It might have been done.` is `false` |
| `reportings` | a verb governing a clausal complement (`ccomp`), with its subject when the parse has one | `Smith reported that the effect vanished.` gives one `Reporting`: verb `report`, complement head `vanished`, subject `Smith` |
| `root_adverbials` | every `advmod` arc into the root | `Reportedly, the deal closed.` gives one `RootAdverbial`: `reportedly` |
| `hearst_pairs` | the six Hearst (1992) lexico-syntactic patterns, matched as dependency arcs | `Animals such as dogs and cats need daily care.` gives two `HearstPair` values, both tagged `such_as`, hypernym `animal`, hyponyms `dog` and `cat` |

Two consequences of reporting arcs rather than readings:

**`bare_assertion` does not consult negation.** The detector reads the root, its finiteness carriers, and the modal list. Nothing else. `The sky is not blue` is a bare assertion that also carries a negation, and combining the two is your code's job.

**Overlap is intentional.** `never` in the example above is both a `Negation` and a `RootAdverbial`, because it satisfies both arc shapes. Neither field is assigning it a meaning, so neither has to win.

Passive voice is the seventh construction and the one exception to the field shape: `Sentence::is_passive` matches `nsubj:pass`, `nsubjpass`, or `aux:pass` on demand, and its document-level share is materialized as the `passive_ratio` field.

Reporting verbs and root adverbials are open word classes, so matra ships no lexicon for either. `Sentence::reportings_in` and `Sentence::root_adverbials_in` filter by a list you supply.

## Where to go next

[Situation model](./situation-model.md) covers what this output is for. [Programming model](./programming-model.md) covers the surface that produces it.
