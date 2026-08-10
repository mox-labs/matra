# matra

matra parses text into a typed structure of tokens, sentences, paragraphs, and sections, then computes a set of metrics over that structure. The three examples below run the same analysis three ways.

## Command line

```console
$ cargo install matra --features cli
$ matra analyze essay.md
essay.md
  sentences          35
  words              332
  mean sentence len  9.5
  sentence len sd    7.4
  passive ratio      0.057
```

`matra summarize` and `matra keyphrases` take the same file. Add `--json` to any of them to get the full structure instead of the table.

## Rust

```rust
use matra::nlp::udpipe::Udpipe;

let nlp = Udpipe::english("/tmp/matra-models")?;
let text = "The committee approved the proposal without debate. \
            Three amendments were submitted by the working group.";
let doc = matra::analyze(text, &nlp)?;

println!("{:?}", doc.sections[0].paragraphs[0].readability_grade);
println!("{:?}", doc.vocabulary_ttr);
```

## Python

```python
from pathlib import Path
from matra import Matra

v = Matra.english(str(Path.home() / ".matra" / "models"))
text = (
    "The committee approved the proposal without debate. "
    "Three amendments were submitted by the working group."
)
result = v.analyze(text)

print(result["sections"][0]["paragraphs"][0]["readability_grade"])
print(result["vocabulary_ttr"])
```

All three return the same shape: sections, paragraphs, sentences, tokens. A struct on the Rust side, a dict on the Python side. Every token carries its part of speech and the dependency relation connecting it to the token that governs it.

The second sentence in the example is passive. "amendments" carries the relation `nsubj:pass`. matra tags it and moves on.

It does not flag the sentence as weak, hedge it as evasive, or score it as good or bad writing. Whether that passive clause matters, and what it means when it does, is your code's decision.

matra reports structure. Your code interprets it. That boundary is where higher-order reasoning about text gets its ground: not matra's opinion of the sentence, but the structure underneath it.

## Where to go next

**Understand what it produces.** [What matra gives you](./capabilities.md) is the full list of values the pipeline returns: the token fields, the tree walks, the five metrics, and the four extractors.

**See the type graph.** [Domain model](./reference/domain-types.md) covers what each type owns, which values are stored and which are computed on demand, and what crosses into Python.

**Get it running.** [Installation](./tutorials/installation.md) covers the model download that happens the first time `Udpipe::english` or `Matra.english` runs. Then [Rust](./guides/rust.md), [Python](./guides/python.md), or [CLI](./guides/cli.md).
