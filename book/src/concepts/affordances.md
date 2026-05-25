# What vaani offers

vaani illuminates the internal structural makeup of text, enabling effective higher-order reasoning on text. This page is the capability inventory: what vaani produces today, and what it plans to produce.

---

## Ships today (✅ v0.1)

### Structured parse

Every token in every sentence gets a lemma, a part-of-speech tag, a dependency relation (`dep`), and the position of its head in the sentence tree. Tokens nest into `Sentence` values, sentences into `Paragraph`, paragraphs into `Section`, sections into `Document`.

The result is a typed hierarchy you can traverse and query. If you want to know whether the subject of the main clause in paragraph two is an agent or a patient, the structure gives you that handle directly.

See [dependency parsing](./dependency-parsing.md) for what the relation labels mean. See [domain types](../reference/domain-types.md) for the full field inventory.

### Document metrics

Six instruments derived from the structural parse:

- Flesch-Kincaid readability grade
- Lexical density
- Vocabulary type-token ratio (TTR)
- Nominalization ratio
- Passive ratio
- Brotli compression ratio

These are measurements, not judgments. vaani measures; your application decides what the measurements mean. See [reference/methodology.md](../reference/methodology.md) for the formulas and explicit non-claims.

### Summarization

Two extractive summarization algorithms:

- **TF-IDF**: scores sentences by term frequency weighted by inverse document frequency; favors coverage.
- **TextRank**: builds a graph over sentence similarity; favors coherence.

Capped at `MAX_SENTENCES = 2000`. See [summarization algorithms](./tfidf-textrank.md) for the algorithm details.

### Keyphrase extraction

Two keyphrase extraction algorithms:

- **RAKE**: rule-based, fast; identifies phrases by co-occurrence in a sentence window.
- **YAKE**: adds positional weighting and statistical context; favors phrases that are distinctive to this document.

Capped at `MAX_TOKENS = 200_000`. See [keyphrase extraction algorithms](./rake-yake.md) for the algorithm details.

### HTML report

`Document::to_html_report()` (Rust) / `Vaani.report(text, format="html")` (Python) / `vaani report essay.md --format html` (CLI). Renders the parse and metrics as an HTML page suitable for visual inspection, Jupyter notebooks, and supplementary materials. See [HTML report](../reference/html-report.md).

---

## Planned (🛠️ v0.2+)

### Rule evaluation over parsed structure

A sub-module that lets consumers query parse trees with predicate-like rules: matchers over POS sequences, dependency relations, lemma sets, subtrees. Enables: relation extraction, schema extraction, modality detection, speech act classification, and voice signature analysis.

Rule evaluation is what closes the gap between the record tier (what vaani ships today: tokens, dependencies, metrics) and the abstract tier (what text is *doing*: who is making commitments, who is hedging, what the relational structure of an argument is).

**Trigger:** surface design settled and a consumer commits to using it. See [future direction](../architecture/future-direction.md).

---

## What vaani does not do

vaani does not interpret. It structures. The distinction matters.

- vaani tells you that 50% of sentences in a document are passive constructions. It does not tell you whether that is appropriate for the genre, intentional, or a problem.
- vaani tells you that the readability grade is 11.2. It does not tell you whether that is too high for the intended audience.
- vaani tells you which tokens have high TF-IDF scores. It does not tell you whether those keyphrases are the "important" ones for your application's purpose.

The interpretation is the job of whatever system consumes vaani's output: an LLM, a rule engine, a human analyst.

See [architecture/conviction.md](../architecture/conviction.md) for the architectural reasoning behind this boundary.
