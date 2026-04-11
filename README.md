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

```rust
use vaani::{analyze_markdown, nlp::udpipe::Udpipe};

// Downloads the English model on first call (~16MB)
let nlp = Udpipe::english("./models").unwrap();

let text = std::fs::read_to_string("essay.md").unwrap();
let analysis = analyze_markdown(&text, &nlp).unwrap();

println!("Sentences: {}", analysis.document.total_sentences);
println!("FK Grade: {:.1}", analysis.document.readability_grade);
println!("Passive: {:.1}%", analysis.document.passive_ratio * 100.0);
```

## Usage (Python)

```python
from vaani import Vaani

# Downloads the English model on first call (~16MB)
v = Vaani.english("~/.vaani/models")
result = v.analyze_markdown(open("essay.md").read())
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

Hex architecture. Domain depends on the `NlpProvider` trait (port), not on UDPipe directly. UDPipe is the default adapter, behind a feature flag.

```
src/
  domain.rs       # types (zero internal deps)
  nlp/mod.rs      # NlpProvider trait (zero external deps)
  nlp/udpipe.rs   # UDPipe adapter (only file importing udpipe_rs)
  encoders.rs     # pipeline (depends on domain + port types)
  markdown.rs     # markdown parser (returns domain types)
  lib.rs          # public API + PyO3 bindings
```

## License

MIT
