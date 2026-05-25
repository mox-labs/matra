# HTML report

🛠️ Planned v0.1

The HTML report renders a `Document`'s parse and metrics as a self-contained HTML page. It is the visual inspection surface for vaani's output: dependency trees, metric values, summarization results, and keyphrases in a readable layout. The report is suitable for Jupyter notebooks, supplementary materials in papers, and standalone inspection during development.

---

## API surface (locked)

The naming below is final. It will not change between stub publication and v0.1 ship.

### Rust

```rust
let doc: Document = vaani::analyze(source, &parser)?;
let html: String = doc.to_html_report();
```

`Document::to_html_report()` returns a `String` containing a self-contained HTML page. No external dependencies are required to render it.

### Python

```python
from vaani import Vaani

v = Vaani()
html = v.report(text, format="html")
```

`Vaani.report(text, format="html")` returns a `str` containing the same HTML. The `format` parameter also accepts `"json"` and `"conllu"` for structured output.

### CLI

```
vaani report essay.md --format html > report.html
```

`vaani report` writes the HTML to stdout. Redirect to a file or open directly in a browser.

### Jupyter

When `format="html"` is used inside a Jupyter notebook context, the result type implements `_repr_html_`. Assigning the result to the last expression of a cell renders the report inline:

```python
v.report(text, format="html")  # renders inline in Jupyter
```

---

## What the report contains

The report structure is not yet finalized. The v0.1 report will include:

- Document metadata (character count, sentence count, paragraph count)
- All six metric values with brief definitions inline
- The dependency tree for each sentence rendered as an SVG or HTML table
- Extractive summary (TF-IDF or TextRank, configurable)
- Top keyphrases (RAKE and/or YAKE, configurable)

The exact layout is a follow-up design task. The API surface above is fixed; the contents of the HTML string will be defined when the implementation lands.

---

## Planned for v0.1

This page is a stub. The implementation is planned for the v0.1 release. See [future direction](../architecture/future-direction.md) for the trigger condition. The API surface above is locked and will not change; the visual design of the report output may evolve up to the v0.1 release.

For the current output surface, see [guides/rust.md](../guides/rust.md), [guides/python.md](../guides/python.md), and [guides/cli.md](../guides/cli.md) for JSON and CoNLL-U output today.
