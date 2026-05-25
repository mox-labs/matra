# Summarization algorithms

🛠️ This page is a stub. Full content lands in a follow-up iteration.

vaani provides two extractive summarization algorithms: TF-IDF sentence scoring and TextRank. Both return a ranked subset of sentences from the original document. Neither generates new text.

---

## The two algorithms

**TF-IDF** scores each sentence by the sum of its terms' TF-IDF weights: how frequently each term appears in the document weighted by how rare it is across sentences. A sentence that contains many terms that appear frequently in the document but are concentrated in few sentences scores high. TF-IDF summarization favors **coverage**: the summary tends to include sentences that represent the document's major topics.

**TextRank** builds a graph where each node is a sentence and each edge weight is the cosine similarity between two sentences' TF-IDF vectors. It then runs a PageRank-style iteration to score each sentence by how similar it is to many other sentences. TextRank favors **coherence**: the summary tends to include sentences that are central to the document's main thread.

For the exact formulas, see [reference/methodology.md](../reference/methodology.md#summarization-tf-idf).

---

## When to use each

The choice depends on what "summary" means for your application:

- If you want a summary that covers all major topics (even if the coverage is repetitive), TF-IDF is appropriate.
- If you want a summary that represents the dominant, internally consistent thread of the document, TextRank is appropriate.
- Neither is "better." They optimize for different things.

Both are capped at `MAX_SENTENCES = 2000` input sentences. Documents exceeding this limit return an error.

---

## What extractive summarization does not do

Both algorithms select sentences from the source document. They do not:

- Paraphrase or shorten individual sentences
- Detect topic shifts or narrative structure
- Identify the "most important" sentences in any goal-sensitive sense
- Guarantee that the selected sentences form a coherent readable text

The resulting summary is a subset of the original sentences, in their original form. If those sentences are not individually readable in isolation (e.g., they contain unresolved pronouns or forward references), the summary inherits that limitation.

---

## Planned for this page

A follow-up iteration will add:

- A side-by-side comparison: the same document summarized by TF-IDF vs TextRank, with annotations explaining why the two results differ
- Guidance on choosing `n` (the number of sentences to return)
- The original TextRank paper citation (Mihalcea and Tarau, 2004)
