# Methodology

What each number matra reports is computed from, how to cite the work it comes from, and what it does not license you to conclude. Every formula on this page is transcribed from the implementation, not from the publication it descends from; where matra departs from the published method, the departure is stated.

Related pages: [Domain types](domain-types.md) for the fields these values live in, [Errors](errors.md) for the caps each algorithm enforces.

## The parse layer

### Model identity

The English model is pinned in `src/nlp/udpipe.rs`.

| Property | Value |
|---|---|
| File | `english-ewt-ud-2.5-191206.udpipe` |
| Size | 16,309,608 bytes |
| SHA-256 | `784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957` |
| Treebank | English Web Treebank, Universal Dependencies 2.5, release 191206 |
| Distributor | LINDAT/CLARIAH-CZ repository, handle 11234/1-3131 |

`Udpipe::english(model_dir)` downloads the file when it is absent, checks the size, computes the SHA-256, and loads the same bytes it hashed. There is no second read from disk between verification and load. A file that fails verification is deleted and downloaded once more; a second failure returns `Error::ModelInvalid` and nothing is loaded.

Verification belongs to that constructor alone. `Udpipe::from_path` and `Udpipe::from_bytes` load whatever you hand them without checking size or hash, so a result produced through either one is only as identifiable as the file you supplied.

Changing the model means editing the pinned constants, which makes the change visible in version control and in any caller that pins a matra version.

### What the parse produces

One `Token` per token with all ten CoNLL-U columns, grouped into sentences. Two properties of the shipped adapter matter when you report results:

- `deps` (CoNLL-U column 9) is always `_`. The `udpipe-rs` binding does not surface enhanced dependencies.
- `Sentence::text` is built from token surface forms, with a space inserted unless the preceding token carries `SpaceAfter=No` in `misc`. It is a reconstruction, not a substring of your input.

### Which text is parsed

The route through the library changes what gets segmented into sentences.

Every pipeline route parses each non-blockquote paragraph separately. Blockquote paragraphs are not parsed, so they contribute nothing to any metric. Calling a provider's `parse` directly parses the whole string in one call with no decomposition, and its segmentation can differ from the pipeline's at paragraph boundaries.

`Ingest::path` picks the format from the file extension, so the same bytes segment differently under `.md` and under `.txt`; `Ingest::text` takes the format as an argument.

Paragraph-at-a-time parsing means a sentence can never be attributed to the wrong paragraph. It also means sentence segmentation cannot run across a paragraph boundary. Report which format your numbers were produced under.

### Citation

Straka, M., & Straková, J. (2017). Tokenizing, POS Tagging, Lemmatizing and Parsing UD 2.0 with UDPipe. *Proceedings of the CoNLL 2017 Shared Task: Multilingual Parsing from Raw Text to Universal Dependencies*, 88-99.

Silveira, N., Dozat, T., de Marneffe, M.-C., Bowman, S., Connor, M., Bauer, J., & Manning, C.D. (2014). A Gold Standard Dependency Corpus for English. *Proceedings of the Ninth International Conference on Language Resources and Evaluation (LREC 2014)*.

Zeman, D., et al. (2019). *Universal Dependencies 2.5*. LINDAT/CLARIAH-CZ digital library, Institute of Formal and Applied Linguistics, Charles University.

### Limitations

UDPipe is a statistical syntactic parser. It performs no named entity recognition, no coreference resolution, and no semantic role labeling. Tags and relations are predictions, not ground truth, and error rates on text unlike the training corpus are higher than on text like it. The shipped configuration is English only.

## Paragraph metrics

Two different counts appear below, and confusing them changes the answer. `Paragraph::word_count` counts non-punctuation tokens from the parse, and every applicability gate is stated in those terms. The formulas themselves split the paragraph text on whitespace. A gate stated in `word_count` terms is not a gate on whitespace words.

### Flesch-Kincaid grade level

```
grade = 0.39 * (words / sentences) + 11.8 * (syllables / words) - 15.59
```

| Input | As computed |
|---|---|
| `words` | Count of whitespace-separated pieces of the paragraph text |
| `sentences` | Count of `.`, `!`, and `?` characters in the paragraph text, floored at 1 |
| `syllables` | Sum over words of the syllable estimate below. A word with no alphabetic characters contributes 0 |

The syllable estimate for one word: keep only alphabetic characters, lowercase them; if the result is three bytes or shorter, return 1; otherwise count runs of the letters `a`, `e`, `i`, `o`, `u`, `y`, where adjacent vowels count once; subtract 1 if the word ends in `e` and the count is above 1; return at least 1.

**Applied to** paragraphs that are not in a blockquote and whose `word_count` is above 10. Stored in `Paragraph::readability_grade`. The value is unbounded and can be negative on short, simple text. `metrics::readability::flesch_kincaid_grade(text: &str) -> f64` computes it on any string, and returns 0.0 for a string with no whitespace-separated pieces.

**Citation.** Kincaid, J.P., Fishburne, R.P., Rogers, R.L., & Chisholm, B.S. (1975). *Derivation of New Readability Formulas for Navy Enlisted Personnel*. Research Branch Report 8-75, Naval Technical Training Command.

**Limitations.** The formula was fitted to Navy training material in 1975 and predicts nothing about conceptual difficulty, argument quality, or required background knowledge. Two paragraphs with the same syllable and length profile score the same whatever they assert. The original publication specifies no syllable-counting algorithm, so grade values from different tools are not comparable; the heuristic above miscounts many words, and non-ASCII letters register as consonants. Counting sentence-ending characters treats abbreviations, decimals, and ellipses as sentence boundaries.

### Lexical density

```
lexical_density = content_words / total_words
```

| Input | As computed |
|---|---|
| `total_words` | Count of whitespace-separated pieces of the paragraph text |
| `content_words` | Those pieces whose alphabetic characters, lowercased, form a non-empty string that is not in the stop word list |

A piece made only of punctuation counts in the denominator and never in the numerator.

**Applied to** paragraphs that are not in a blockquote, whose `word_count` is not zero, and whose text yields at least one whitespace-separated piece. Stored in `Paragraph::lexical_density`. Range 0.0 to 1.0.

**Citation.** Ure, J. (1971). Lexical density and register differentiation. In G. Perren & J.L.M. Trim (Eds.), *Applications of Linguistics*. Cambridge University Press.

**Limitations.** The value depends entirely on the stop word list, which is matra's own list of 105 English words, not a standard one. Ure's definition counts lexical items by part of speech; matra approximates that with a stop word list over whitespace pieces. Density says nothing about whether the content words are used well, or at all correctly.

### Compression ratio

```
compression_ratio = compressed_bytes / original_bytes
```

| Input | As computed |
|---|---|
| `original_bytes` | UTF-8 byte length of the paragraph text |
| `compressed_bytes` | Byte length of the brotli-compressed text at quality 6 with a window parameter of 18, which is a 256 KiB window |

**Applied to** paragraphs that are not in a blockquote, whose `word_count` is above 50, and whose text is at most 262,144 bytes. Paragraphs above that byte ceiling are skipped and keep `None`, which bounds worst-case compression time. The slot also stays `None` if the encoder fails to consume the text. Stored in `Paragraph::compression_ratio`. A lower ratio means the text compressed further, which means more surface repetition.

**Citation.** Alakuijala, J., & Szabadka, Z. (2016). *Brotli Compressed Data Format*. RFC 7932, Internet Engineering Task Force.

**Limitations.** This is a redundancy proxy over bytes, not a measure of meaning, quality, or novelty. Precise technical prose that reuses terminology compresses much like repetitive filler. The number is a property of one encoder at two specific parameter settings: a different compressor, or a different version of this one, gives different values.

## Document metrics

The two stored document metrics and the derived document methods are computed over the sentences attached to the document's paragraphs, which excludes blockquote paragraphs. There is exactly one sentence set, and it is derivable from the tree in one way, so the numbers cannot disagree with the structure they describe.

### Vocabulary type-token ratio

```
vocabulary_ttr = distinct_lemmas / total_lemmas
```

| Input | As computed |
|---|---|
| `total_lemmas` | Count of non-punctuation tokens across the sentence slice |
| `distinct_lemmas` | Count of distinct `lemma` strings in that same set, compared exactly, without case folding |

**Applied to** the whole slice. Stored in `Document::vocabulary_ttr`, which stays `None` when the slice contains no non-punctuation token.

**Citation.** Johnson, W. (1944). Studies in language behavior: A program of research. *Psychological Monographs*, 56(2), 1-15.

**Limitations.** Type-token ratio falls as documents get longer, because repetition accumulates. Values from documents of different lengths are not comparable without a length correction, and matra applies none. Because lemmas are compared exactly, a provider that emits case-varying lemmas inflates the distinct count.

### Nominalization ratio

```
nominalization_ratio = nominalizing_nouns / total_lemmas
```

| Input | As computed |
|---|---|
| `total_lemmas` | The same denominator as vocabulary type-token ratio: non-punctuation tokens across the slice |
| `nominalizing_nouns` | Tokens whose `pos` is `NOUN` and whose `text`, lowercased, ends in `tion`, `ment`, `ness`, `ity`, `ence`, or `ance` |

The suffix test reads the surface form, not the lemma, so a plural nominalization such as `conditions` or `measurements` is missed: the plural `-s` falls after the suffix, so `"conditions".ends_with("tion")` is false even though the singular `condition` matches.

**Applied to** the whole slice. Stored in `Document::nominalization_ratio`, which stays `None` when the slice contains no non-punctuation token.

**Limitations.** Six suffixes are a heuristic, not a morphological analysis. Words that end in those letters without being nominalizations count as false positives, and nominalizations formed otherwise, such as `growth` or `failure`, are missed. The test depends on the POS tagger having assigned `NOUN`, so tagging errors propagate. The denominator counts every running word rather than every noun, so the value is a share of the text, not a share of the nouns in it.

### Passive ratio

```
passive_ratio = passive_sentences / total_sentences
```

A sentence counts as passive when any of its tokens carries a `dep` of `nsubj:pass`, `nsubjpass`, or `aux:pass`. The first two are the Universal Dependencies and older Stanford spellings of a passive subject; the third is a passive auxiliary.

**Computed** by `Document::passive_ratio()` and `Corpus::passive_ratio()`, and per sentence by `Sentence::is_passive()`. The measure stage stores the document-level value in the `passive_ratio` field, which is how it crosses to Python; the `Corpus` aggregate and `is_passive` stay methods, Rust only. Returns 0.0 when there are no sentences, and the field is `None` when the measure stage has not run.

**Citation.** de Marneffe, M.-C., Manning, C.D., Nivre, J., & Zeman, D. (2021). Universal Dependencies. *Computational Linguistics*, 47(2), 255-308.

**Limitations.** This counts sentences that contain a passive construction, not passive clauses, so a sentence with three passives counts once. Detection is only as good as the parser's relation labels. Nothing here judges whether a passive is appropriate.

### Sentence length statistics

`Document::mean_sentence_length()` is `total_words / total_sentences`, where `total_words` counts non-punctuation tokens. `Document::sentence_length_std()` is the sample standard deviation of per-sentence non-punctuation token counts, with denominator n minus 1. Both return 0.0 when undefined: no sentences for the mean, fewer than two for the deviation. Both are computed on demand and are not stored.

### Corpus aggregates

`Corpus` reports three figures over its entries, all computed on demand.

| Method | Definition | Value with nothing to average |
|---|---|---|
| `total_words()` | Non-punctuation tokens summed across every entry | 0 |
| `passive_ratio()` | Passive sentences over total sentences, pooled across every entry | 0.0 |
| `mean_readability()` | Unweighted mean of every `readability_grade` that is present | 0.0 |

`mean_readability` averages paragraphs, not documents, so a long document contributes more terms than a short one. Paragraphs whose slot is `None` are absent from both the numerator and the count.

## Summarization

Both summarizers take `&[Sentence]` and a count, score every sentence, keep the highest N, and return them in ascending `position`. Neither generates text: the output sentences are the input sentences.

### Shared inputs

A term is a token's `lemma` with punctuation tokens removed and stop words removed. The stop word test lowercases the lemma before looking it up; the term itself is the lemma as the provider produced it, without case folding. The stop word list holds 105 English words and is shared with the paragraph lexical density metric.

### TF-IDF

Each sentence is treated as a document when computing document frequency.

```
score(s) = ( sum over distinct terms t in s of tf(t, s) * idf(t) ) / distinct_terms(s)

tf(t, s)  = occurrences of t in s / total terms in s
idf(t)    = ln( total_sentences / df(t) )
df(t)     = number of sentences containing t
```

A sentence with no terms scores 0.0. The mean divides by the number of distinct terms, not by the number of term occurrences. A term present in every sentence has an `idf` of 0 and contributes nothing.

**Cap.** 2,000 sentences. Above it, `Error::InputTooLarge` with `what` set to `"tfidf"`.

**Citation.** Luhn, H.P. (1958). The Automatic Creation of Literature Abstracts. *IBM Journal of Research and Development*, 2(2), 159-165. Spärck Jones, K. (1972). A statistical interpretation of term specificity and its application in retrieval. *Journal of Documentation*, 28(1), 11-21. Salton, G., & Buckley, C. (1988). Term-weighting approaches in automatic text retrieval. *Information Processing and Management*, 24(5), 513-523.

**Limitations.** Treating sentences as documents is an adaptation of a corpus-level weighting scheme, and the resulting weights are not the weights you would get from a real document collection. Scores measure term rarity within one text, not informativeness. Selected sentences are not guaranteed to read coherently together, and nothing checks for redundancy between them.

### TextRank

```
similarity(a, b) = shared(a, b) / ( ln(1 + |a|) + ln(1 + |b|) )

shared(a, b) = sum over terms t in both of min(count_a(t), count_b(t))
|a|          = number of distinct terms in a
```

Similarity is 0 when either sentence has no terms or they share none. The matrix is symmetric and its diagonal stays 0, so a sentence never reinforces itself.

```
score(i) at step k+1 = (1 - d) / N
                     + d * sum over j not equal to i of
                       ( similarity(j, i) / out_sum(j) ) * score(j) at step k

d          = 0.85
N          = number of sentences
out_sum(j) = sum of row j of the similarity matrix
```

All scores start at `1 / N`. Iteration stops after 50 passes, or earlier when the largest single-sentence change in a pass falls below 1e-6. Sentences whose `out_sum` is 0 send no weight.

**Cap.** 2,000 sentences, which bounds the dense similarity matrix at roughly 32 MB of `f64`. Above it, `Error::InputTooLarge` with `what` set to `"textrank"`.

**Citation.** Mihalcea, R., & Tarau, P. (2004). TextRank: Bringing Order into Text. *Proceedings of the 2004 Conference on Empirical Methods in Natural Language Processing*, 404-411. Page, L., Brin, S., Motwani, R., & Winograd, T. (1999). *The PageRank Citation Ranking: Bringing Order to the Web*. Stanford InfoLab Technical Report 1999-66.

**Limitations.** Similarity here is lemma overlap. Two sentences expressing one idea in different words have similarity 0. The normalization by the logarithm of distinct-term counts is matra's, not the one in the 2004 paper, so scores are not comparable with other TextRank implementations. Reaching the iteration ceiling without converging is possible and is not reported.

## Keyphrase extraction

Both extractors take `&[Sentence]` and a maximum count, and return `Keyphrase` values in descending score order. Both cap input at 200,000 tokens counted across the slice with punctuation included.

### RAKE

**Candidates.** Within each sentence, a candidate is a maximal run of tokens whose `pos` is `NOUN`, `ADJ`, or `PROPN` and whose lowercased lemma is not a stop word. Punctuation, stop words, and any other part of speech end the run. Candidate words are lowercased lemmas.

```
for each candidate phrase p of length k, for each word w in p:
    freq(w)   += 1
    degree(w) += k

word_score(w)     = degree(w) / freq(w)
phrase_score(p)   = sum over w in p of word_score(w)
```

When the same phrase string occurs more than once, the highest score is kept. Phrases are then sorted by score descending and truncated to the requested count.

**Cap.** 200,000 tokens. Above it, `Error::InputTooLarge` with `what` set to `"rake"`.

**Citation.** Rose, S., Engel, D., Cramer, N., & Cowley, W. (2010). Automatic Keyword Extraction from Individual Documents. In M.W. Berry & J. Kogan (Eds.), *Text Mining: Applications and Theory*. John Wiley and Sons.

**Limitations.** Published RAKE delimits candidates with stop words and punctuation over surface words. matra adds a part-of-speech filter and works over lemmas, so scores and candidate sets differ from a reference implementation. A high degree over frequency ratio means a word appears in long phrases relative to how often it appears at all; that is a structural property, not evidence of topical centrality.

### YAKE

**Terms.** Non-punctuation tokens whose lowercased lemma is not a stop word and is longer than one character. Position is a running index over every non-punctuation token, so stop words and single-character tokens occupy positions without becoming terms.

```
pos_score(t)         = ln( 1 + mean_position(t) / total_positions )
freq_score(t)        = occurrences(t) / total_positions
context_diversity(t) = distinct_neighbors(t) / neighbor_observations(t)

term_score(t) = ( pos_score(t) + context_diversity(t) ) / ( freq_score(t) + 1.0 )
```

`total_positions` is the count of all non-punctuation tokens across the slice, floored at 1. A neighbor is the adjacent non-punctuation token on either side within the same sentence, recorded only when it is not a stop word; a term with no recorded neighbor gets a context diversity of 0.0. Lower `term_score` means more relevant at this stage.

```
ngram_score(p) = product over words w in p of term_score(w)
output_score(p) = 1.0 / ngram_score(p)
```

Candidates are sliding windows of 1, 2, and 3 words over each sentence's term list, with stop words and single-character tokens already removed, so an n-gram can join words that were not adjacent in the source. A sentence with fewer than three terms yields correspondingly shorter windows. When a phrase string occurs more than once, the lowest `ngram_score` is kept.

Candidates whose score is not strictly positive, or is not finite, are dropped before inversion. A term reaches a score of exactly 0 when it occurs only at position 0 and has no eligible neighbor, and any n-gram containing it is dropped with it. The remainder are sorted by `output_score` descending, so higher is more relevant in the returned `Keyphrase`.

**Cap.** 200,000 tokens. Above it, `Error::InputTooLarge` with `what` set to `"yake"`.

**Citation.** Campos, R., Mangaravite, V., Pasquali, A., Jorge, A., Nunes, C., & Jatowt, A. (2020). YAKE! Keyword extraction from single documents using multiple local features. *Information Sciences*, 509, 257-289.

**Limitations.** Published YAKE combines five features, including casing and sentence spread, and applies deduplication over similar candidates. matra implements three features, no deduplication, and a different combination formula. Scores are not comparable with the reference implementation, and the two will not agree on rankings. The inversion means output scores are unbounded above and have no interpretable unit; use them for ordering, not as magnitudes.

## Determinism and reproducibility

### What is fixed

matra draws no random numbers, sets no seeds, samples nothing, and runs the analysis path on one thread. The model is pinned by SHA-256, and the bytes that pass verification are the bytes that get loaded. Given the same model file and the same input, the parse is the same.

Given the same parse, these are reproducible bit for bit on one machine: every paragraph metric, every document metric, every derived method on `Document` and `Corpus`, TextRank scores and selection, RAKE word and phrase scores, and YAKE term and n-gram scores. TextRank in particular is order-stable because its shared-term counts are integer sums and its iteration walks sentence indices in order.

### What is not bit-stable

Two places carry hash map iteration order into the result.

1. **TF-IDF sentence scores.** The per-sentence score sums over a hash map whose iteration order is not fixed between runs. Floating-point addition is not associative, so the low bits of a score can differ between runs.

   Selection changes only when two sentences differ by less than that rounding difference.
2. **RAKE and YAKE output order among ties.** Candidates are collected out of a hash map into a vector and sorted by score with a stable sort. Phrases with exactly equal scores can therefore come out in a different relative order between runs.

   When such a tie straddles the truncation point, which phrase is returned can differ.

Two further sources of variation are worth recording in a methods section.

- Natural logarithms come from the platform's math library. Values can differ by an ulp across operating systems and toolchains, so compare cross-platform results with a tolerance rather than for equality.
- The compression ratio depends on the brotli encoder. A change in the encoder version can change the value for unchanged text.

### What to record

To let someone reproduce a result, report: the matra version, the model file name and its SHA-256, which format the document was analyzed under, and the parameter values you passed, such as the requested sentence count or maximum phrase count.

```
matra <crate version> (https://github.com/mox-labs/matra)
parse: UDPipe, english-ewt-ud-2.5-191206.udpipe
       SHA-256 784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957
format: markdown
parameters: tfidf_summarize(sentences, 5)
```

Cite the algorithm publications listed above for the methods themselves, and the UDPipe and Universal Dependencies references for the parse layer.

Cite matra separately from the methods. The `CITATION.cff` file at the repository root carries the software entry, and it identifies the implementation and its version. The publications identify the method. Because matra departs from several published methods in the ways recorded on this page, a reader who has only the method citation does not know which values you produced, and a reader who has only the software citation does not know what the values are meant to measure. Both are needed.

## General limitations

These hold for every number on this page.

- matra reports structure and stops. It measures; your application decides what a measurement means.
- No output is a judgment of quality, correctness, originality, or authorship. Nothing here detects machine-generated text.
- No output carries meaning. Passive ratio counts a syntactic pattern, compression ratio counts bytes, keyphrase scores count co-occurrence. None of them read.
- Metrics that share a name with published measures are matra's implementations of them, with the departures listed per metric. Do not compare matra's values with another tool's values for a like-named metric.
- Every value downstream of the parse inherits the parser's errors. On text unlike the English Web Treebank, that inheritance is larger.
