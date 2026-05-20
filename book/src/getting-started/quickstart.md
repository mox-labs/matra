# Quickstart

## Rust

```rust,ignore
use vaani::{analyze_markdown, nlp::udpipe::Udpipe};

fn main() -> vaani::domain::Result<()> {
    let nlp = Udpipe::english("./models")?;
    let text = std::fs::read_to_string("essay.md")?;
    let analysis = analyze_markdown(&text, &nlp)?;

    println!("Sentences:           {}", analysis.total_sentences());
    println!("Mean sentence len:   {:.1}", analysis.mean_sentence_length());
    println!("Passive ratio:       {:.1}%", analysis.passive_ratio() * 100.0);

    for para in analysis.paragraphs() {
        if let Some(grade) = para.readability_grade {
            println!("  reading grade {:.1} for: {}", grade, &para.text[..40.min(para.text.len())]);
        }
    }
    Ok(())
}
```

The pattern:

1. Load an `NlpProvider` (here, `Udpipe::english(...)`).
2. Call one of the convenience APIs (`analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`).
3. Read structured results off the returned `Analysis`.

## Python

```python
from pathlib import Path
from vaani import Vaani

# Downloads the English model on first call (~16 MB).
v = Vaani.english(str(Path.home() / ".vaani" / "models"))

result = v.analyze_markdown(Path("essay.md").read_text())

# result is a dict mirroring the Rust Analysis type.
# Iterate sections → paragraphs → sentences → tokens.
for sec in result["sections"]:
    for para in sec["paragraphs"]:
        if para["readability_grade"] is not None:
            print(f"  grade {para['readability_grade']:.1f}: {para['text'][:40]}")
```

The Python surface is typed; `mypy --strict` will catch dict-access mistakes against the TypedDict shapes shipped in `_core.pyi`.

## The parse-once-use-many pattern

For documents you'll analyze multiple ways (summary + keyphrases + metrics), parse once and reuse the parsed sentences:

```rust,ignore
use vaani::{parse, analyze_from, extraction::{tfidf_summarize, rake_keyphrases}};
use vaani::decompose::{Decomposer, markdown::MarkdownDecomposer};

let nlp = Udpipe::english("./models")?;
let text = std::fs::read_to_string("essay.md")?;

let sections = MarkdownDecomposer.decompose(&text);
let sentences = parse(&text, &nlp)?;

let analysis = analyze_from(sections, &sentences)?;
let summary = tfidf_summarize(&sentences, 3)?;
let keyphrases = rake_keyphrases(&sentences, 10)?;
```

`parse` is the single expensive step; `analyze_from`, `tfidf_summarize`, and `rake_keyphrases` are cheap consumers of the parsed sentences.

## What's next

- [Rust usage](../usage/rust.md): full surface, including corpus analysis and error handling.
- [Python usage](../usage/python.md): type stubs, dict shapes, exception classes.
- [The pipeline](../concepts/pipeline.md): the five verbs (ingest, decompose, parse, measure, extract) and how they compose.
