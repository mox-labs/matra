# Playground

🛠️ **Planned v0.0.x alpha. Wires once the WASM crust ships.**

The playground is the visceral demonstration of vaani's conviction: paste text, see structure illuminate. No install. No setup. Just text in, parsed structure out, in your browser.

## What it will demonstrate

Five panels, all live in your browser:

1. **Structured parse.** Collapsible Document → Section → Paragraph → Sentence → Token tree, plus the dependency-tree visualization per sentence. Hover a token for its CoNLL-U fields (lemma, POS, dep, head); click for the subtree it governs.

2. **Document metrics dashboard.** Readability grade, lexical density, vocabulary TTR, nominalization ratio, passive ratio, mean sentence length. Computed per-paragraph and per-document, with reference baselines (academic register vs conversational register) for context.

3. **Summarization comparison.** Same input, both algorithms side-by-side: TF-IDF (sentence-frequency coverage) and TextRank (graph-coherence ranking). See where they agree, where they diverge, and why.

4. **Keyphrase comparison.** Same input, both algorithms side-by-side: RAKE (fast, rule-based) and YAKE (positional + statistical context). Ranked keyphrases with scores.

5. **Markdown awareness.** Toggle `analyze` vs `analyze_markdown` on the same source. See how section boundaries change the parse.

Plus a few preset example texts (technical doc snippet, literary excerpt, passive-heavy bureaucratic blurb, multilingual paragraph) so you can compare vaani's output against texts whose structure you can already feel.

## Technical posture

The playground runs entirely client-side. Vaani's Rust core compiles to WebAssembly via the UDPipe-in-emscripten path (validated [in the WASM-B spike](https://github.com/mox-labs/vaani/tree/alpha/.claude/spikes/wasm-udpipe)). The UDPipe English model (~16 MB) is fetched from LINDAT once, cached in IndexedDB, and reused across visits. First-paint latency is one model download; subsequent visits load instantly.

## Why the wait

The 🛠️ marker is honest. The WASM crust is two engineering steps away:

- **Step 3:** `wasm-bindgen` surface mirroring the PyO3 API (methods don't cross FFI; fields do). Targets the npm package `vaani`.
- **Step 4:** IndexedDB caching + SHA-256 verify for the model fetch.

Both are tracked in the alpha roadmap. When they land, this page wires up and the markers flip ✅.

Meanwhile, the same capabilities are available today via:

- [Rust](../guides/rust.md): `cargo add vaani`
- [Python](../guides/python.md): `pip install vaani`
- [CLI](../guides/cli.md): quick scripted analysis

The conviction is the same. The browser-without-install access is what comes with the WASM crust.
