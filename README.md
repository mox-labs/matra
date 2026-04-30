# vaani

Prose metrics engine. Text in, structured analysis out.

Readability scores, POS distributions, dependency structures, lexical density, compression ratios. Rust core with Python bindings.

## Install

```bash
# Rust
cargo add vaani

# Python
pip install vaani
```

## Usage (Rust)

```rust,ignore
use vaani::{analyze_markdown, nlp::udpipe::Udpipe};

// Downloads the English model on first call (~16MB)
let nlp = Udpipe::english("./models").unwrap();

let text = std::fs::read_to_string("essay.md").unwrap();
let analysis = analyze_markdown(&text, &nlp).unwrap();

println!("Sentences: {}", analysis.total_sentences());
println!("Mean length: {:.1}", analysis.mean_sentence_length());
println!("Passive: {:.1}%", analysis.passive_ratio() * 100.0);
```

## Usage (Python)

```python
from pathlib import Path
from vaani import Vaani

# Downloads the English model on first call (~16MB)
model_dir = str(Path.home() / ".vaani" / "models")
v = Vaani.english(model_dir)

result = v.analyze_markdown(Path("essay.md").read_text())
```

## Usage (CLI)

```bash
# Auto-downloads model on first use
vaani analyze essay.md
vaani analyze essay.md --json
vaani analyze essay.md -s    # section breakdown
```

## Metrics

**Per sentence:** word count, POS tags, dependency labels, passive voice, tree depth.

**Per paragraph:** sentence count, readability grade, lexical density, compression ratio.

**Per document:** passive ratio, mean sentence length, vocabulary TTR, nominalization ratio.

## Architecture

Hex architecture. Domain depends on port traits (`Source`, `Decomposer`, `NlpProvider`), not on adapters directly. UDPipe is the default NLP adapter, behind the `udpipe` feature flag.

```text
src/
  domain.rs              # types (zero internal deps)
  source/                # Source port + File/Directory adapters
  decompose/             # Decomposer port + Markdown/Plain adapters
  nlp/                   # NlpProvider port
    udpipe.rs            # UDPipe adapter (only file importing udpipe_rs)
  encoders.rs            # metric pipeline (domain + stopwords only)
  extraction/            # TF-IDF, TextRank, RAKE, YAKE
  lib.rs                 # composition root + PyO3 bindings
```

## License

MIT
