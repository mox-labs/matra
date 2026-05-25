# Extractive summarization: TF-IDF and TextRank

vaani provides two extractive summarization algorithms. Both return a ranked subset of original sentences. Neither generates new text.

---

## Extractive vs generative

Extractive summarization selects sentences from the source document. The output is always a verbatim fragment of the input. What changes is which sentences get selected.

Generative summarization (as in large language models) produces new text. The summary may be shorter, use different vocabulary, and combine information from multiple parts of the document. It may also hallucinate facts that are not in the source.

vaani's algorithms are extractive. The text in a `ScoredSentence` result is always a verbatim sentence from the input. This makes extractive summarization appropriate when reproducibility and traceability matter: you can verify that the selected sentence appears at `position` in the original document.

---

## TF-IDF: coverage by term frequency

TF-IDF (term frequency-inverse document frequency) scores each sentence by how distinctively it uses the document's key terms.

The algorithm treats each sentence as a "document":

1. For every content lemma (stop words excluded) in every sentence, compute how many sentences contain it. This is the document frequency (DF).
2. For each sentence, compute the frequency of each of its terms within that sentence. This is the term frequency (TF).
3. For each term in a sentence, TF-IDF score = TF * ln(total\_sentences / DF). Terms that appear in many sentences have low IDF (they are not distinctive). Terms that appear in few sentences have high IDF.
4. The sentence score is the mean TF-IDF across its terms.

A high-scoring sentence uses terms that appear frequently within that sentence but infrequently across other sentences. This makes TF-IDF good at finding sentences that cover distinctive, specific content.

**When to use it:** documents with many topics where you want the summary to cover the range of distinct terms. Research reports, meeting transcripts, multi-topic articles.

**When it underperforms:** documents where the same idea is expressed with many different words. TF-IDF does not recognize synonyms; it matches on lemmas only.

---

## TextRank: coherence by mutual reinforcement

TextRank (Mihalcea and Tarau, 2004) builds a graph where nodes are sentences and edges represent similarity. A sentence scores higher when many other sentences are similar to it.

The algorithm:

1. For each pair of sentences, compute a similarity score: the count of shared content lemmas divided by the sum of the log-lengths of both sentences (to avoid favoring long sentences).
2. Build an n×n similarity matrix from these scores.
3. Run iterative PageRank: each sentence's score is the weighted sum of the scores of sentences similar to it. Damping factor = 0.85; convergence threshold = 1e-6; maximum iterations = 50.
4. Select the top-N sentences by final score.

A high-scoring sentence shares vocabulary with many other sentences. This makes TextRank good at finding sentences that are central to the document's argument: the sentences the rest of the document's language echoes.

PageRank was originally described for web pages by Brin and Page (1998). TextRank applies the same mutual-reinforcement idea to sentences. An implementation in Python (the `summa` package) and in many other languages exists; vaani's implementation is from scratch in Rust using the same algorithm.

**When to use it:** documents with a coherent central topic where you want the summary to capture the core, not the periphery. Arguments, essays, focused technical documents.

**When it underperforms:** documents with multiple independent topics. TextRank will favor sentences from the largest cluster of similar sentences and may miss smaller topics entirely.

---

## The 2000-sentence cap

Both algorithms are capped at 2000 input sentences. Above this limit, they return an error (`Error::InputTooLarge`).

The cap exists because the memory and compute costs differ between the two algorithms. TF-IDF is linear in total tokens; at 2000 sentences with roughly 30 content tokens each, the working set is around 2 MB and completes in well under a second. TextRank's similarity matrix is O(n^2): at 2000 sentences, the matrix holds 4,000,000 pairs of f64 values, which is about 32 MB. Beyond 2000 sentences, that grows to memory sizes that would stall unattended processing.

The two algorithms have separate cap constants in the source, even though both happen to be 2000. The TextRank cap is derived from its 32 MB matrix bound; the TF-IDF cap is a separate decision. If TextRank's memory cost model changes, its cap changes independently.

The cap is per algorithm call, not per document. If you have a 3000-sentence document and need to summarize it, split it by section first and summarize each section.

---

## Output shape

Both algorithms return `Vec<ScoredSentence>` in document order (not score order). Each `ScoredSentence` has:

- `text`: the verbatim sentence
- `score`: the relevance score (higher is more relevant)
- `position`: the sentence's index in the input slice

Scores are not normalized to a common scale between algorithms. A TF-IDF score of 0.4 and a TextRank score of 0.4 are not comparable; the scales are different.

[Methodology](../reference/methodology.md) documents the formulas in full. [Affordances](./affordances.md) covers the other capabilities vaani provides.
