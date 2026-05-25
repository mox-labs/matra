# HTML Report Reference

🛠️ Planned v0.2

The HTML report is vaani's visual inspection surface. It renders a structured analysis as a self-contained HTML document, suitable for embedding in a paper's supplementary materials, displaying in a Jupyter notebook, or writing to disk for review.

---

## Locked surface names

These names are settled. Downstream documentation and tooling should use them exactly.

**Rust**

```rust
let html: String = document.to_html_report();
```

`Document::to_html_report()` returns a `String` containing the full HTML. No file is written; the caller decides where the output goes.

**Python**

```python
vaani_instance.report(text, format="html")
```

`Vaani.report(text, format)` accepts `format` values of `"html"`, `"json"`, and `"conllu"`. Returns a string in the requested format.

**CLI**

```
vaani report essay.md --format html
```

Writes the HTML report to stdout by default; redirect with `> report.html`.

**Jupyter integration**

The object returned by `Vaani.report(text, format="html")` implements `_repr_html_`, so it renders inline in a Jupyter cell output without any additional display call.

---

## Output shape (planned v0.2)

The report contains:

- Document-level metrics table: readability grade, lexical density, vocabulary TTR, nominalization ratio, passive ratio, compression ratio
- Section and paragraph hierarchy with per-paragraph metric slots
- Sentence view with dependency arc visualization
- TF-IDF and TextRank summary blocks
- RAKE and YAKE keyphrase tables

The exact HTML structure is not locked; the surface names above are locked. Callers should not parse the HTML output programmatically; use `format="json"` for machine-readable output.

---

*For the domain types the report renders, see [reference/domain-types.md](domain-types.md). For metric definitions, see [reference/methodology.md](methodology.md).*
