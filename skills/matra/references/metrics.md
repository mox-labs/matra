---
name: metrics
summary: Every measure's formula, when it is computed, and what it does not license you to conclude.
---

# Metrics: formulas and limitations

Every formula here is transcribed from the implementation. Where the implementation departs from the published method it descends from, the departure is stated. Values are not comparable with another tool's values for a like-named metric.

Two counts appear below and confusing them changes the answer. A paragraph's word count is its non-punctuation tokens from the parse, and every applicability gate is stated in those terms. The formulas themselves split the paragraph text on whitespace.

<!-- needs: model -->

```console
$ matra analyze draft.txt --sections
```

## Paragraph metrics

### `readability_grade`, Flesch-Kincaid grade level

```text
grade = 0.39 * (words / sentences) + 11.8 * (syllables / words) - 15.59
```

`words` is whitespace-separated pieces of the paragraph text. `sentences` is the count of `.`, `!` and `?` characters, floored at 1. `syllables` sums a per-word estimate: keep alphabetic characters, lowercase, return 1 if three bytes or shorter, else count runs of a, e, i, o, u, y with adjacent vowels counting once, subtract 1 if the word ends in `e` and the count is above 1, return at least 1.

**Computed for** non-blockquote paragraphs with more than 10 words. Unbounded, and negative on short simple text.

**Cite** Kincaid, Fishburne, Rogers and Chisholm (1975), Naval Technical Training Command Research Branch Report 8-75.

**Does not mean** conceptual difficulty, argument quality, or required background. Two paragraphs with the same syllable and length profile score the same whatever they assert. The 1975 publication specifies no syllable algorithm, so values from different tools are not comparable, and this heuristic miscounts many words. Non-ASCII letters register as consonants, and abbreviations, decimals and ellipses read as sentence boundaries.

### `lexical_density`

```text
lexical_density = content_words / total_words
```

`total_words` is whitespace-separated pieces. `content_words` are those pieces whose lowercased alphabetic characters form a non-empty string that is not a stop word. A piece made only of punctuation counts in the denominator and never in the numerator.

**Computed for** non-blockquote paragraphs with at least one word and at least one whitespace piece. Range 0.0 to 1.0.

**Cite** Ure (1971), in Perren and Trim, *Applications of Linguistics*.

**Does not mean** that the content words are used well, or correctly. The value depends entirely on the stop word list, which is matra's own list of 105 English words rather than a standard one. Ure counts lexical items by part of speech; this approximates that with a stop list over whitespace pieces.

### `compression_ratio`

```text
compression_ratio = compressed_bytes / original_bytes
```

Brotli at quality 6 with a window parameter of 18, a 256 KiB window, over the UTF-8 bytes of the paragraph text. A lower ratio means the text compressed further, which means more surface repetition.

**Computed for** non-blockquote paragraphs with more than 50 words and at most 262,144 bytes. Larger paragraphs keep null, which bounds worst-case compression time, and the slot also stays null if the encoder fails.

**Cite** Alakuijala and Szabadka (2016), RFC 7932.

**Does not mean** redundancy of meaning, quality, or novelty. It is a byte-level proxy: precise technical prose that reuses terminology compresses much like repetitive filler. The number belongs to one encoder at two parameter settings, so a different compressor or a different version of this one gives a different value.

## Document metrics

All three are computed over the sentences attached to non-blockquote paragraphs. There is exactly one sentence set and one way to derive it, so the numbers cannot disagree with the structure they describe.

### `vocabulary_ttr`

```text
vocabulary_ttr = distinct_lemmas / total_lemmas
```

`total_lemmas` counts non-punctuation tokens. `distinct_lemmas` counts distinct `lemma` strings among them, compared exactly, without case folding. Null when there is no non-punctuation token.

**Cite** Johnson (1944), *Psychological Monographs* 56(2).

**Does not mean** vocabulary richness across documents. Type-token ratio falls as documents get longer, because repetition accumulates, and no length correction is applied. Documents of different lengths are not comparable on it. A provider emitting case-varying lemmas inflates the distinct count.

### `nominalization_ratio`

```text
nominalization_ratio = nominalizing_nouns / total_lemmas
```

`nominalizing_nouns` are tokens whose `pos` is `NOUN` and whose lowercased surface `text` ends in `tion`, `ment`, `ness`, `ity`, `ence` or `ance`. The denominator is the same one `vocabulary_ttr` uses. Null when there is no non-punctuation token.

**Does not mean** a morphological analysis. The suffix test reads the surface form, so plurals such as `conditions` are missed. Words ending in those letters without being nominalizations are counted. The test depends on the tagger having assigned `NOUN`, so tagging errors propagate. The denominator is every running word, not every noun, so the value is a share of the text rather than a share of the nouns.

### `passive_ratio`

```text
passive_ratio = passive_sentences / total_sentences
```

A sentence counts as passive when any token carries a `dep` of `nsubj:pass`, `nsubjpass`, or `aux:pass`. Returns 0.0 when there are no sentences; the field is null when the metric stage has not run.

**Cite** de Marneffe, Manning, Nivre and Zeman (2021), *Computational Linguistics* 47(2).

**Does not mean** bad writing, and does not count clauses. A sentence with three passives counts once. Detection is only as good as the parser's relation labels, and nothing here judges whether a passive is appropriate.

## Summarization

Both take a sentence slice and a count, score every sentence, keep the highest N, and return them in ascending position. Neither generates text: the output sentences are the input sentences. A term is a token lemma with punctuation and stop words removed.

### `tfidf`

```text
score(s) = ( sum over distinct terms t in s of tf(t, s) * idf(t) ) / distinct_terms(s)
tf(t, s) = occurrences of t in s / total terms in s
idf(t)   = ln( total_sentences / df(t) )
```

Each sentence is its own document for document frequency. A sentence with no terms scores 0.0. A term present in every sentence has an `idf` of 0 and contributes nothing. Capped at 2,000 sentences.

**Cite** Luhn (1958), Spärck Jones (1972), Salton and Buckley (1988).

**Does not mean** informativeness. Treating sentences as documents adapts a corpus-level weighting scheme, so the weights are not the ones a real collection would give. Selected sentences are not guaranteed to read coherently together, and nothing checks for redundancy between them. Scores are not bit-stable between runs, because the per-sentence sum walks a hash map and floating-point addition is not associative.

### `textrank`

```text
similarity(a, b) = shared(a, b) / ( ln(1 + |a|) + ln(1 + |b|) )
score(i) at k+1  = (1 - d) / N + d * sum over j != i of ( similarity(j, i) / out_sum(j) ) * score(j) at k
```

`shared` sums the minimum count of each term the two sentences share; `|a|` is the count of distinct terms in a. `d` is 0.85, all scores start at 1/N, iteration stops after 50 passes or when the largest change falls below 1e-6. The diagonal stays 0, so a sentence never reinforces itself. Capped at 2,000 sentences, which bounds the dense matrix at roughly 32 MB.

**Cite** Mihalcea and Tarau (2004), Page, Brin, Motwani and Winograd (1999).

**Does not mean** semantic centrality. Similarity here is lemma overlap, so two sentences expressing one idea in different words have similarity 0. The log-length normalization is matra's, not the 2004 paper's, so scores are not comparable with other implementations. Reaching the iteration ceiling without converging is possible and is not reported.

## Keyphrases

Both take a sentence slice and a maximum, return highest score first, and cap at 200,000 tokens counted with punctuation included.

The two scores live on unrelated scales and neither is a magnitude. A RAKE word score is at least 1, so a phrase of k words scores at least k and longer phrases outrank shorter ones by construction; a YAKE score is a reciprocal, unbounded above with no unit, and rises with phrase length too. Order phrases within one document under one method, and do not put a RAKE number beside a YAKE one.

### `rake`

Candidates are maximal runs of tokens whose `pos` is `NOUN`, `ADJ` or `PROPN` and whose lowercased lemma is not a stop word.

```text
for each candidate p of length k, for each word w in p: freq(w) += 1; degree(w) += k
word_score(w)   = degree(w) / freq(w)
phrase_score(p) = sum over w in p of word_score(w)
```

**Cite** Rose, Engel, Cramer and Cowley (2010), in Berry and Kogan, *Text Mining*.

**Does not mean** topical centrality. A high degree-over-frequency ratio means a word appears in long phrases relative to how often it appears at all, which is a structural property. Published RAKE works over surface words with no part-of-speech filter, so candidate sets and scores differ from a reference implementation.

### `yake`

```text
pos_score(t)         = ln( 1 + mean_position(t) / total_positions )
freq_score(t)        = occurrences(t) / total_positions
context_diversity(t) = distinct_neighbors(t) / neighbor_observations(t)
term_score(t)        = ( pos_score(t) + context_diversity(t) ) / ( freq_score(t) + 1.0 )
ngram_score(p)       = product over words w in p of term_score(w)
output_score(p)      = 1.0 / ngram_score(p)
```

Candidates are sliding windows of 1, 2 and 3 words over each sentence's term list after stop words and single-character tokens are removed, so an n-gram can join words that were not adjacent in the source. Candidates whose score is not strictly positive or not finite are dropped before inversion.

**Cite** Campos, Mangaravite, Pasquali, Jorge, Nunes and Jatowt (2020), *Information Sciences* 509.

**Does not mean** what published YAKE means. That version combines five features and deduplicates similar candidates; this one implements three features, no deduplication, and a different combination formula. Output scores are unbounded above and have no interpretable unit: use them for ordering, not as magnitudes.

## Reproducibility

No random numbers, no seeds, no sampling, one thread on the analysis path, and the model pinned by SHA-256. Given the same model and input, the parse is the same, and every metric, every TextRank score, and every RAKE and YAKE score is reproducible bit for bit on one machine.

Two things are not bit-stable. TF-IDF sentence scores carry hash map iteration order into a floating-point sum, so low bits can differ between runs, and RAKE and YAKE output order among exactly tied scores can differ, which can change which phrase survives truncation at the requested count. Sort keyphrase output before diffing it across runs.

Natural logarithms come from the platform math library, so cross-platform values can differ by an ulp; compare with a tolerance. The compression ratio depends on the brotli encoder version.

To let someone reproduce a result, report the matra version, the model file name and its SHA-256, the format the document was analyzed under, and the parameter values passed.
