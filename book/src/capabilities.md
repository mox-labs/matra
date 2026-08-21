# What matra gives you

matra turns a document into a typed tree and measures it. Everything below is a value you can read off that tree. Nothing on this page is an opinion about the text.

Four families of output, in the order the pipeline produces them.

## 1. Structure

Every word becomes a `Token` carrying the ten CoNLL-U columns:

| Field | What it holds |
|---|---|
| `text` | the word as written |
| `lemma` | dictionary form (`approved` becomes `approve`) |
| `pos` | universal part of speech (`NOUN`, `VERB`, `ADJ`) |
| `xpos` | treebank-specific tag, finer grained than `pos` |
| `feats` | morphology (tense, number, person) |
| `head` | id of the token this one depends on |
| `dep` | the dependency relation to that head |
| `deps`, `misc` | secondary dependencies and annotation |
| `is_punct` | punctuation flag, so you can filter without a regex |

`head` names another token's `id`, and `dep` names the relation between them. Those two columns turn a flat list of tokens into a tree:

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
<text class="nt" x="30" y="292">heavier lines are the two relations is_passive() matches</text>
</svg>

Words keep their reading order left to right; height is depth in the tree. `built` is the root, everything else hangs off it directly or through one more hop, and nothing sits deeper than that, so `tree_depth()` returns 2. `Sentence` gives you the walks over that tree:

| Call | Returns |
|---|---|
| `root_token()` | the token everything else hangs off |
| `head_of(id)` | the governor of a token |
| `children_of(id)` | its direct dependents |
| `subtree(id)` | the whole clause under it |
| `tree_depth()` | nesting depth, `usize::MAX` on a malformed cycle |
| `is_passive()` | whether the sentence carries `nsubj:pass` |
| `content_tokens()` | non-punctuation tokens |
| `word_count()` | count of those |

Above the sentence sit `Paragraph`, `Section`, and `Document`. Sections carry their heading and level, so the document tree mirrors the document's own outline.

## 2. Metrics

Five numbers computed over that structure. Three are per paragraph, two per document.

| Metric | Scope | What it measures |
|---|---|---|
| `readability_grade` | paragraph | Flesch-Kincaid grade level, from sentence and syllable length |
| `lexical_density` | paragraph | content words as a share of all words |
| `compression_ratio` | paragraph | brotli compressed size over raw size, a proxy for repetition |
| `vocabulary_ttr` | document | distinct words over total words |
| `nominalization_ratio` | document | share of nouns formed from verbs (`decide` becoming `decision`) |

Each is `Option<f64>`. `None` means the metric could not be computed for that unit, most often because it was too short or exceeded a size cap, and it is distinct from a computed zero. [Methodology](./reference/methodology.md) gives each formula and the exact condition under which it returns `None`.

One caution before you compare documents. `vocabulary_ttr` is a raw type-token ratio, and that measure falls as text grows: a longer document reuses words it has already spent. Two documents of different lengths are therefore not comparable on this field, and the gap you see is partly length rather than voice. Within a fixed length it is sound. Across a corpus, normalize first.

Alongside them, `Document` computes on demand: `total_words`, `total_sentences`, `paragraph_count`, `passive_ratio`, `mean_sentence_length`, and `sentence_length_std`. `Corpus` adds `total_words`, `passive_ratio`, and `mean_readability` across every document in a directory.

## 3. Summarization

Two rankers, both returning `ScoredSentence` with the sentence text, its score, and its original position.

`tfidf_summarize` scores each sentence by the rarity of the words in it against the rest of the document. It is fast and it favours sentences carrying distinctive vocabulary.

`textrank_summarize` builds a similarity graph across sentences and runs PageRank over it. It favours sentences that many others resemble, which tends to surface the document's central claim rather than its most unusual one. It is capped at `MAX_SENTENCES` because the graph is quadratic.

## 4. Keyphrases

Two extractors, both returning `Keyphrase` with the phrase and its score.

`rake_keyphrases` splits on stopwords and scores the runs between them by word degree over frequency. It finds multi-word phrases well and needs no corpus.

`yake_keyphrases` scores individual terms on position, casing, spread, and sentence frequency, then assembles phrases. It is more selective on single-word terms.

### What these four are not

All four rank what is already in the text by a statistical measure of importance. None of them produces a claim. A `ScoredSentence` is a sentence the document contained, selected because its vocabulary or its graph position scored well, and nothing about that selection asserts the sentence states something, or that what it states is atomic, or that it is true.

If you need claims rather than salient sentences, that work sits above matra and generally needs a model that reads for meaning. What matra contributes to it is the layer underneath: sentence boundaries, the predicate-argument structure of each one, and the verbatim text to ground a claim back to.

Used deliberately, the gap is informative. Where these extractors and a semantic extractor agree on what a document is about, that agreement is evidence. Where they diverge is worth looking at.

## Provenance is preserved

`Paragraph.text` and `Sentence.text` are verbatim slices of the input. Nothing is normalized, re-wrapped, or rewritten on the way through, and because each paragraph is parsed on its own, the chain from a token up through its sentence, paragraph, and section is unambiguous rather than reconstructed by matching text.

This matters if you store analysis and later have to show which bytes a value came from. Provenance holds by construction, not by convention.

## The surface

One pipeline, two values. `Ingest` says where documents come from; `Engine` says what happens to each one.

| Value | Constructors | What it carries |
|---|---|---|
| `Ingest` | `text(string, format)`, `path(file or directory)` | the source variation: a string is a stream of one, a directory is a stream of many |
| `Engine` | `new(provider, decomposer table)` | the pipeline: `analyze` a stream, `analyze_one` document, or the stages `annotate` and `compose` separately |

`engine.analyze(ingest)` returns a lazy stream of per-document results; collecting it into `CorpusResult` partitions successes from failures, and nothing in between aborts on one bad file. `annotate` gives you structure and sentences without the metric suite, which is the route when you want the extractors: read the sentences off the tree once and hand the same slice to every extractor. Nothing is parsed twice.

## What this is for

The output is structure, not verdict. matra tells you a sentence carries `nsubj:pass`; it does not tell you the sentence is weak. That boundary is deliberate, and it is what makes the output reusable.

Some things that fall out of it:

**Retrieval and chunking.** Section and paragraph boundaries come from the document's own outline, so chunks split where the author split rather than every N tokens.

**Corpus comparison.** A directory streamed through the pipeline gives per-document metrics on a shared scale, so drift between documents or across time is measurable rather than felt.

**Rule evaluation.** The dependency tree is queryable. A rule like "flag passive constructions whose agent is omitted" is a walk over `children_of` and `dep`, not a regex over prose.

**Feature extraction.** Every field is serde-serializable and crosses to Python as a plain dict, so the parse can feed a model without a Rust dependency downstream.

## Where to go next

[Domain model](./reference/domain-types.md) is the full type graph: what each type owns, which values are stored and which are computed, and what crosses the language boundary.

[Architecture](./architecture/design.md) is how a call actually runs: what executes in what order, where the expensive state lives, and what can fail.
