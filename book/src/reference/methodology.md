# Methodology Reference

Authoritative formula definitions, inputs, citations, and non-claims for every metric and algorithm vaani computes. Concept pages link here for full precision. This page does not explain why these metrics are useful; for that, see [concepts/readability.md](../concepts/readability.md), [concepts/tfidf-textrank.md](../concepts/tfidf-textrank.md), and [concepts/rake-yake.md](../concepts/rake-yake.md).

---

## Document metrics

### Flesch-Kincaid Grade Level ✅

**Formula**

```
FK_grade = 0.39 * (words / sentences) + 11.8 * (syllables / words) - 15.59
```

**Inputs**

| Input | How vaani computes it |
|---|---|
| `words` | Whitespace-split token count (`text.split_whitespace().count()`) |
| `sentences` | Count of `.`, `!`, and `?` characters in the paragraph text, with a floor of 1 |
| `syllables` | Sum over words: vowel-run count per word (adjacent vowels counted once), with trailing silent `e` removed when count > 1, with a floor of 1 syllable per word |

**Important.** vaani uses whitespace splitting for word count, matching the formula's original specification. NLP token count (which excludes punctuation and may split contractions) gives a different number. These are not equivalent.

**Applied to.** Non-blockquote paragraphs with more than 10 whitespace-split words. `Paragraph::readability_grade` holds the result.

**Citation.** Kincaid, J.P., Fishburne, R.P., Rogers, R.L., & Chisholm, B.S. (1975). *Derivation of new readability formulas (Automated Readability Index, Fog Count and Flesch Reading Ease Formula) for Navy enlisted personnel.* Research Branch Report 8-75, Naval Technical Training Command.

**What it measures.** Predicted U.S. school grade level needed to read the paragraph.

**What it does not measure.** Conceptual difficulty, argument quality, domain knowledge required, or any form of meaning. Two paragraphs with identical vocabulary and sentence structure but opposite claims score identically.

---

### Lexical Density ✅

**Formula**

```
lexical_density = content_words / total_words
```

**Inputs**

| Input | How vaani computes it |
|---|---|
| `total_words` | Whitespace-split token count for the paragraph |
| `content_words` | Whitespace-split tokens whose alphabetic characters, lowercased, are not in the stop-word list |

**Applied to.** Non-blockquote paragraphs with at least one word. `Paragraph::lexical_density` holds the result. Range: 0.0 to 1.0.

**Citation.** Ure, J. (1971). Lexical density and register differentiation. In G. Perren & J.L.M. Trim (Eds.), *Applications of Linguistics: Selected Papers of the Second International Congress of Applied Linguistics*. Cambridge University Press.

**What it measures.** Proportion of content-bearing words relative to total words, as a proxy for information density.

**What it does not measure.** Comprehensibility, domain complexity, or whether the content words are being used correctly.

---

### Vocabulary Type-Token Ratio (TTR) ✅

**Formula**

```
vocabulary_ttr = unique_lemmas / total_lemmas
```

**Inputs**

| Input | How vaani computes it |
|---|---|
| `total_lemmas` | Count of non-punctuation tokens across all sentences in the document (via `Sentence.tokens` where `is_punct == false`) |
| `unique_lemmas` | Count of distinct lemma strings in the same set |

**Applied to.** All non-punctuation tokens document-wide, across all paragraphs including blockquotes (document-level metrics aggregate from the sentence slice, not from the paragraph filter). `Document::vocabulary_ttr` holds the result.

**What it measures.** Lexical variety: how often the text reuses the same base forms.

**What it does not measure.** The value is sensitive to document length; longer documents have more repetition by chance and will show lower TTR than shorter documents on the same topic. Do not compare TTR values across documents of very different lengths without correcting for length.

**Citation.** Johnson, W. (1944). Studies in language behavior. *Psychological Monographs*, 56(2).

---

### Nominalization Ratio ✅

**Formula**

```
nominalization_ratio = nominalizing_nouns / total_non_punct_lemmas
```

**Inputs**

| Input | How vaani computes it |
|---|---|
| `total_non_punct_lemmas` | Same denominator as vocabulary TTR |
| `nominalizing_nouns` | Tokens where `pos == "NOUN"` AND `text.to_lowercase()` ends with one of: `"tion"`, `"ment"`, `"ness"`, `"ity"`, `"ence"`, `"ance"` |

The check is on `token.text` (surface form), not `token.lemma`. This catches plurals (`conditions`, `measurements`) that share the same lemma as a nominalizing singular.

**Applied to.** All sentences document-wide. `Document::nominalization_ratio` holds the result.

**What it measures.** Density of nominalizations (verbs or adjectives converted to nouns via suffix), a marker associated with bureaucratic and abstract writing styles.

**What it does not measure.** The suffix list is a heuristic. It produces false positives (e.g. "station", "lement" in "parliament") and false negatives (nominalizations with other endings, such as "growth", "failure"). POS tagging must correctly identify the token as `NOUN`; POS errors propagate into this metric.

---

### Passive Ratio ✅

**Formula**

```
passive_ratio = passive_sentences / total_sentences
```

**Inputs**

| Input | How vaani computes it |
|---|---|
| `passive_sentences` | Sentences where at least one token has `dep` equal to `"nsubj:pass"`, `"nsubjpass"`, or `"aux:pass"` |
| `total_sentences` | All sentences across the document |

**Applied to.** All sentences document-wide. Computed by `Document::passive_ratio()` at call time; not stored as a field. Range: 0.0 to 1.0; returns 0.0 when there are no sentences.

**What it measures.** Fraction of sentences containing a passive voice construction as detected by dependency labels.

**What it does not measure.** The detection depends on the NLP model correctly assigning passive dependency labels. Models trained on formal written English perform well; code-switched or heavily colloquial text may produce label errors. The three labels cover the Universal Dependencies conventions (`nsubj:pass`, `aux:pass`) and the older Stanford convention (`nsubjpass`). Not all passive constructions in all languages use these labels.

---

### Brotli Compression Ratio ✅

**Formula**

```
compression_ratio = compressed_bytes / original_bytes
```

**Inputs**

| Input | How vaani computes it |
|---|---|
| `original_bytes` | UTF-8 byte length of the paragraph text |
| `compressed_bytes` | Brotli-compressed byte length using quality=6, lgwin=18 (256 KiB window) |

**Applied to.** Non-blockquote paragraphs with more than 50 whitespace-split words AND byte length at or below 256 KiB. Paragraphs above the byte cap are skipped (`compression_ratio = None`) to bound worst-case CPU time. `Paragraph::compression_ratio` holds the result.

**Lower ratio means more compressible, meaning more repetitive prose.** Ratio near 1.0 means the compressed output is similar in size to the original; very low ratios (e.g. 0.3) indicate high repetition.

**What it measures.** Surface redundancy as a proxy signal. A rough detector for repetitive or template-generated text.

**What it does not measure.** Meaning, quality, or accuracy. Highly compressed ratio is consistent with both high-quality technical prose (which uses precise repeated terminology) and with low-quality repetitive filler. The metric is a signal, not a verdict.

---

## Summarization

### TF-IDF Sentence Scoring ✅

**Algorithm.** Each sentence is treated as a document. Score for sentence `i`:

```
score(i) = mean over unique terms t in sentence i of:
    tf(t, i) * idf(t)

where:
    tf(t, i) = count(t in sentence i) / total_terms(sentence i)
    idf(t)   = ln( total_sentences / df(t) )
    df(t)    = number of sentences containing term t
```

**Terms.** Non-punctuation, non-stop-word lemmas, lowercased. Stop words are excluded before TF computation. The term set for IDF includes all such terms across all input sentences.

**Mean denominator.** The score divides by `unique_term_count(sentence i)` (the number of distinct terms in the sentence's TF map), not by total term count.

**Output.** Top-N sentences returned in document order (ascending position). Cap: 2,000 sentences; returns `Error::InputTooLarge { what: "tfidf" }` above the cap.

**Citation.** Luhn, H.P. (1958). The Automatic Creation of Literature Abstracts. *IBM Journal of Research and Development*, 2(2), 159-165. Salton, G., & Buckley, C. (1988). Term-weighting approaches in automatic text retrieval. *Information Processing & Management*, 24(5), 513-523.

**What it does not measure.** Semantic similarity, topic coherence, or whether the selected sentences form a coherent summary when read together. Sentences that happen to use rare terms score higher regardless of their informational value.

---

### TextRank Similarity and Ranking ✅

**Similarity between sentences `a` and `b`.**

```
similarity(a, b) = shared_count(a, b) / ( ln_1p(|a|) + ln_1p(|b|) )
```

where:

- `shared_count(a, b)` = sum over terms shared between `a` and `b` of `min(count_a[term], count_b[term])`. This is the sum of minimum occurrence counts for shared terms, not a count of distinct shared terms.
- `|a|` and `|b|` are the sizes of the term sets (unique-term counts in the term `HashMap` for each sentence, excluding punctuation and stop words). Not raw word counts or token counts.
- `ln_1p(x)` = `ln(1 + x)` (natural log, shifted to avoid `ln(0)` when a sentence has no content terms).

**PageRank iteration.**

```
score(i) at step k+1 = (1 - d) / N + d * sum over j != i of:
    ( similarity(j, i) / out_sum(j) ) * score(j) at step k

where:
    d          = 0.85 (damping factor)
    N          = total sentence count
    out_sum(j) = sum of similarity(j, *) over all sentences
```

Initial scores: `1/N` for all sentences. Convergence threshold: `max_delta < 1e-6`. Maximum iterations: 50.

**Output.** Top-N sentences by final PageRank score, returned in document order. Cap: 2,000 sentences; returns `Error::InputTooLarge { what: "textrank" }` above the cap.

**Citation.** Mihalcea, R., & Tarau, P. (2004). TextRank: Bringing Order into Text. *Proceedings of the 2004 Conference on Empirical Methods in Natural Language Processing*, 404-411. Page, L., Brin, S., Motwani, R., & Winograd, T. (1999). *The PageRank Citation Ranking: Bringing Order to the Web.* Stanford InfoLab Technical Report.

**What it does not measure.** Semantic meaning beyond lemma overlap. Sentences using different vocabulary to express the same idea will have zero similarity. Long documents where important sentences use rare vocabulary may not surface at the top.

---

## Keyphrase extraction

### RAKE Word Scoring ✅

**Candidate generation.** Candidates are contiguous runs of `NOUN`, `ADJ`, and `PROPN` tokens delimited by stop words, punctuation, or any other POS. Token lemmas are lowercased. Verbs and function words are boundaries, not candidates.

**Co-occurrence matrix.** For each candidate phrase of length `k`, each word in the phrase contributes:
- `freq[word] += 1`
- `degree[word] += k` (phrase length, including the word itself)

**Word score.**

```
word_score(w) = degree(w) / freq(w)
```

**Phrase score.**

```
phrase_score(phrase) = sum of word_score(w) for w in phrase
```

When the same phrase appears multiple times, the highest score is kept.

**Output.** Top-N phrases by phrase score, sorted descending. Cap: 200,000 tokens total across all input sentences; returns `Error::InputTooLarge { what: "rake" }` above the cap.

**Citation.** Rose, S., Engel, D., Cramer, N., & Cowley, W. (2010). Automatic Keyword Extraction from Individual Documents. In M.W. Berry & J. Kogan (Eds.), *Text Mining: Applications and Theory*. John Wiley & Sons.

**What it does not measure.** Semantic relevance beyond surface co-occurrence. RAKE does not use word embeddings or topic models. Phrases with high degree/frequency ratios are not necessarily topically central; they are structurally distinct (uncommon in co-occurrence with other words).

---

### YAKE Feature Scoring and Inversion ✅

**Term feature computation.** For each content term (non-punct, non-stop-word, length > 1 character):

```
pos_score         = ln_1p( mean_position / total_positions )
freq_score        = term_frequency / total_positions
context_diversity = unique_neighbors / total_neighbor_observations

term_score = ( pos_score + context_diversity ) / ( freq_score + 1.0 )
```

where:
- `mean_position` = mean token position index across all occurrences of the term
- `total_positions` = total content token count (across all sentences)
- `unique_neighbors` = count of distinct adjacent non-stop-word lemmas across all occurrences
- `total_neighbor_observations` = total adjacency observations (with minimum of 1 to avoid division by zero)

Lower `term_score` means more relevant in the raw YAKE scoring: frequent terms that appear early and in diverse contexts score low.

**N-gram candidate scoring.**

Candidates are 1-, 2-, and 3-gram windows over content tokens within each sentence (stop words and single-character tokens excluded from the window).

```
ngram_score = product of term_score for each word in the n-gram
```

When the same phrase appears multiple times, the minimum score is kept (lower = more relevant).

**Score inversion for output.**

```
output_score = 1.0 / ngram_score
```

After inversion, higher output score = more relevant. Candidates with zero or non-finite score are excluded.

**Output.** Top-N phrases by output score descending. Cap: 200,000 tokens total; returns `Error::InputTooLarge { what: "yake" }` above the cap.

**Citation.** Campos, R., Mangaravite, V., Pasquali, A., Jorge, A., Nunes, C., & Jatowt, A. (2020). YAKE! Keyword Extraction from Single Documents Using Multiple Local Features. *Information Sciences*, 509, 257-289.

**What it does not measure.** Semantic topic modeling or cross-document relevance. YAKE is unsupervised and operates on positional and frequency statistics within a single document. Term scores are document-relative; a term that is important in a corpus may score poorly in a single short document if it appears too infrequently.

---

*For the domain types that hold metric values, see [reference/domain-types.md](domain-types.md). For conceptual explanation of why these metrics matter, see the concepts pages.*
