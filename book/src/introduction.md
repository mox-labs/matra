# matra

matra reports the structure of text and measurements over it. It parses a document into a typed tree of sections, paragraphs, sentences, and tokens, measures that tree, and ranks what is in it. Every value it returns is structure or a number over structure. What any of it means for your purpose is your code's decision, which is what makes the output reusable across purposes.

## Four tiers of output

- **Structure.** Sections, paragraphs, sentences, and full CoNLL-U tokens, plus the structural primitives read off each dependency tree: negations, modals, bare assertion, reportings, root adverbials, Hearst pairs.
- **Measures.** Readability, lexical density, and compression ratio per paragraph; vocabulary TTR, nominalization ratio, and passive ratio per document.
- **Extraction.** Two extractive summarizers (TF-IDF, TextRank) and two keyphrase extractors (RAKE, YAKE).
- **Semantic clusters.** Sentence embeddings grouped by cosine similarity, behind the `model2vec` feature, carrying the identity of the model that produced them.

## End to end

```python
from matra import Matra

v = Matra.english()
text = (
    "The committee approved the proposal without debate. "
    "Three amendments were submitted by the working group."
)
result = v.analyze(text)

print(result["passive_ratio"])
print(result["sections"][0]["paragraphs"][0]["readability_grade"])
```

The same analysis from Rust, and from the command line:

```rust
use matra::domain::Format;
use matra::{Engine, Ingest};

let engine = Engine::with_defaults()?;

let text = "The committee approved the proposal without debate. \
            Three amendments were submitted by the working group.";
let doc = engine
    .analyze(Ingest::text(text, Format::PlainText))
    .next()
    .expect("a stream of one")
    .map_err(|e| e.error)?
    .analysis;

println!("{:?}", doc.passive_ratio);
```

```console
$ matra analyze essay.md
essay.md
  sentences          35
  words              332
  mean sentence len  9.5
  sentence len sd    7.4
  passive ratio      0.057
```

All three return the same shape: a struct in Rust, a dict in Python, a table or `--json` at the command line. The second sentence in the example is passive, and `amendments` carries the relation `nsubj:pass`. matra records that and stops there.

## Where to go next

**Understand the model.** [Concepts](./explanation/concepts.md) covers the document tree, the four tiers, and what a structural primitive is. [Situation model](./explanation/situation-model.md) covers what the output is for and what it deliberately withholds.

**Write against it.** [Programming model](./explanation/programming-model.md) is the surface: `Ingest`, `Engine`, ports, bounds, and what Python exposes. [Pragmatics](./explanation/pragmatics.md) answers the choices you have to make.

**Get it running.** [Installation](./tutorials/installation.md), then [Rust](./guides/rust.md), [Python](./guides/python.md), or [CLI](./guides/cli.md).

**Look something up.** [What matra gives you](./capabilities.md) lists every field and function the pipeline returns.
