# How matra runs

`engine.analyze(Ingest::path("essay.md")?)` yields a `Document` or a `DocumentError`.

## One call, end to end

Take a 40 KB markdown file. Pulling it through the pipeline runs four things in order, and only the last one is expensive.

**Ingest.** When the stream is pulled, `FileSource` calls `symlink_metadata`, which does not traverse. A symlink is refused. Anything that is not a regular file is refused. A file whose metadata size exceeds `MAX_INPUT_BYTES`, 8 MiB, is refused before a byte is read, so a 1 GB file costs one `stat` call rather than a GB of resident memory. Then `read_to_string` pulls all 40 KB in, and the `.md` extension sets `Format::Markdown` on the `RawDocument`.

**Dispatch.** `Engine::annotate` re-checks the same 8 MiB bound on the string it now holds (two gates, same limit, different `what` labels, because text from `Ingest::text` never passed the first one), then looks the format up in the engine's decomposer table. No entry means `Error::UnsupportedFormat`; for `Markdown` the entry is `MarkdownDecomposer`.

**Decompose.** `MarkdownDecomposer` walks the file line by line, once. Frontmatter, fenced code, and table rows are dropped. A `#` line closes the current section and opens a new one. A `>` line marks its paragraph as a blockquote. A blank line flushes the accumulated paragraph. Output is a `Vec<Section>` holding fresh `String`s: a second copy of the prose.

**Parse and measure.** `annotate` walks every paragraph in document order, skips the blockquotes, and calls `nlp.parse` on each remaining paragraph's text. Sixty body paragraphs means sixty calls into UDPipe, and each call's sentences are stored on their paragraph. Then `compose` runs four metric functions over the attached tree; the document-level pair aggregates over the same attachment, so there is no second sentence set to keep in agreement.

<svg class="mx-res" role="img" aria-label="Residency chart: three allocations of the same prose, showing which are alive at each stage of one pipeline call" viewBox="0 0 720 180" width="720" height="180" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>What is alive at each stage of one pipeline call</title>
<style>
.mx-res text{fill:currentColor}
.mx-res .hd{font-size:8.5px;text-anchor:middle;opacity:.55;font-family:inherit}
.mx-res .lb{font-size:9px;text-anchor:end;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-res .nt{font-size:8px;opacity:.55;font-family:inherit}
.mx-res .bd{font-size:8.5px;text-anchor:middle;opacity:.7;font-family:inherit}
.mx-res .bar{fill:currentColor;fill-opacity:.3;stroke:currentColor;stroke-opacity:.6;stroke-width:1px}
.mx-res .sep{stroke:currentColor;opacity:.12;stroke-width:1px}
.mx-res .ret{stroke:currentColor;opacity:.4;stroke-width:1px;stroke-dasharray:4 3}
.mx-res .band{fill:currentColor;fill-opacity:.05}
</style>
<rect class="band" x="430" y="34" width="180" height="118"/>
<text class="hd" x="205" y="26">ingest</text>
<text class="hd" x="295" y="26">dispatch</text>
<text class="hd" x="385" y="26">decompose</text>
<text class="hd" x="475" y="26">parse</text>
<text class="hd" x="565" y="26">measure</text>
<text class="hd" x="655" y="26">returned</text>
<line class="sep" x1="250" y1="34" x2="250" y2="152"/>
<line class="sep" x1="340" y1="34" x2="340" y2="152"/>
<line class="sep" x1="430" y1="34" x2="430" y2="152"/>
<line class="sep" x1="520" y1="34" x2="520" y2="152"/>
<line class="ret" x1="610" y1="34" x2="610" y2="152"/>
<text class="bd" x="520" y="47">all three resident</text>
<rect class="bar" x="160" y="62" width="450" height="16" rx="3"/>
<rect class="bar" x="340" y="92" width="360" height="16" rx="3"/>
<rect class="bar" x="430" y="122" width="270" height="16" rx="3"/>
<text class="lb" x="150" y="74">String, the file text</text>
<text class="lb" x="150" y="104">Vec&lt;Section&gt;, a second copy</text>
<text class="lb" x="150" y="134">Vec&lt;Sentence&gt; per paragraph</text>
<text class="nt" x="160" y="168">bar length is how long an allocation is alive, not how large it is</text>
</svg>

At peak, three representations of the same document are resident at once: the original `String`, the section copies, and the parsed tokens hanging off the paragraphs.

For a 40 KB input none of that matters. For a caller sizing a batch job against a memory ceiling it does, because every `Token` carries nine owned `String`s.

A directory changes the shape by streaming. `Ingest::path` lists the entries up front but reads nothing until pulled, and each pull runs one document through the whole pipeline before the next file is read. One document's representations are resident at a time; only what you retain accumulates, and collecting into `CorpusResult` retains everything.

## The model is the expensive thing

Everything above assumes `nlp` already exists. Constructing it is the one genuinely slow operation matra performs.

`Udpipe::english(dir)` looks for the English UD-EWT model in the directory you name. If it is absent, the bytes are verified in memory and then written to a temporary subdirectory named for this call, and moved into place with a single `rename`, so two processes pointing at the same directory cannot leave each other a half-written file. The name carries more than the process id, because in a container every process is pid 1 and two cold starts would otherwise collide.

`Udpipe::from_config(&cfg)` is the same call with the directory resolved rather than named, and `Engine::with_defaults()` is `Config::resolve()` followed by that. `Config` is read once, at construction; nothing in the call path above consults it, which is why configuration cannot change what a call computes. `Model2Vec::potion_base_8m(dir)` and `Model2Vec::from_config(&cfg)` do the same for the reference embedding model, against a three-file digest rather than a single one.

The cached file is then checked against a size constant, 16,309,608 bytes, and a SHA-256 constant pinned in `nlp/udpipe.rs`. A size mismatch fails before the file is hashed. A hash mismatch downloads once more and replaces the file only if the new bytes verify, giving up with `Error::ModelInvalid` on a second failure and leaving the old file untouched. An unverified model is never loaded.

The verify step returns the bytes it hashed, and those exact bytes go to the loader. Nothing re-reads the disk in between. That ordering is the whole point: a hash-then-reopen sequence leaves a window in which an attacker with write access to the model directory can swap the file after the check and before the read.

`Udpipe::from_path` and `Udpipe::from_bytes` verify nothing. They load what you name. Traceability for those two paths belongs to the caller.

**Load once, reuse.** After construction, the model is a live handle to a C++ object. Parsing does not reload it. The `Engine` owns the provider, so a service builds one engine and keeps it for its lifetime.

**One model per thread, or a lock.** The underlying `Model` is `Send` and deliberately not `Sync`: its parse path mutates internal workspace caches. In Rust this is a compile error rather than a race, because sharing `&Udpipe` across threads will not typecheck. To parallelize you either give each thread its own model, paying another 16 MB load, or put one behind a `Mutex` and serialize the parse calls.

Python gets the same constraint through a different mechanism. `Matra` is declared `unsendable`, so PyO3 records the creating thread and refuses access from any other at runtime. A `ProcessPoolExecutor` is unaffected, because each process loads its own model.

## The unit of work is the paragraph

Parsing per paragraph looks wasteful. Sixty FFI calls where one would do, and a parser that never sees a paragraph's neighbours as context. The reason it is worth that is a defect class the alternative could not avoid.

The earlier implementation joined every paragraph into one string, parsed once, then wired each returned sentence back to a paragraph by matching text prefixes. Three regression tests in `src/lib.rs` hold the current behaviour in place, one per way that matching failed:

- Two paragraphs sharing their first thirty characters. A sentence lands in whichever one matched first, and the other paragraph silently loses it.
- One paragraph containing another paragraph's opening words mid-text. The greedy match steals the sentence from its rightful owner.
- An empty paragraph with trailing whitespace, which confused the match entirely.

Follow that down and the root is not the matching algorithm. It is that the relationship between a sentence and its paragraph was being reconstructed from text, and text does not carry identity. Two paragraphs can be character-for-character similar and still be different paragraphs.

So any reconstruction from that data has to guess, and a guess in this position fails silently. The `Document` that comes out is still well-formed, still serializes, still passes every shape check. It is simply wrong about which paragraph said what, and nothing in the output says so.

<svg class="mx-col" role="img" aria-label="Left: matching sentences back to paragraphs by text is ambiguous when two paragraphs share a prefix. Right: parsing each paragraph separately leaves nothing to match." viewBox="0 0 720 200" width="720" height="200" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Matching back by text against parsing per paragraph</title>
<style>
.mx-col text{fill:currentColor}
.mx-col .ti{font-size:9px;font-family:inherit}
.mx-col .nt{font-size:8px;opacity:.55;font-family:inherit}
.mx-col .sn{font-size:7.5px;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-col .pg{font-size:9px;text-anchor:middle;font-family:inherit}
.mx-col .fl{font-size:8px;text-anchor:middle;font-family:inherit;paint-order:stroke;stroke:var(--bg,transparent);stroke-width:3px;stroke-linejoin:round}
.mx-col .box{fill:none;stroke:currentColor;opacity:.4;stroke-width:1px}
.mx-col .gone{stroke-dasharray:4 3}
.mx-col .ar{stroke:currentColor;opacity:.7;stroke-width:1.1px;fill:none}
.mx-col .stop{stroke:currentColor;opacity:.7;stroke-width:1.1px}
.mx-col .bar{stroke:currentColor;opacity:.9;stroke-width:2.2px}
.mx-col marker path{fill:currentColor;opacity:.7}
</style>
<defs><marker id="mx-col-a" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0 L8,4 L0,8 z"/></marker></defs>
<text class="ti" x="8" y="22">join, parse once, then match sentences back by text</text>
<text class="nt" x="14" y="38">sentences from the joined text, in order</text>
<rect class="box" x="14" y="46" width="156" height="20" rx="3"/>
<rect class="box" x="14" y="74" width="156" height="20" rx="3"/>
<rect class="box gone" x="14" y="102" width="156" height="20" rx="3"/>
<rect class="box" x="14" y="130" width="156" height="20" rx="3"/>
<text class="sn" x="20" y="60">The system processes input now.</text>
<text class="sn" x="20" y="88">Tail one.</text>
<text class="sn" x="20" y="116">The system processes input now.</text>
<text class="sn" x="20" y="144">Tail two.</text>
<rect class="box" x="250" y="46" width="94" height="48" rx="3"/>
<rect class="box" x="250" y="102" width="94" height="48" rx="3"/>
<text class="pg" x="297" y="74">paragraph A</text>
<text class="pg" x="297" y="130">paragraph B</text>
<line class="ar" x1="174" y1="56" x2="244" y2="66" marker-end="url(#mx-col-a)"/>
<line class="ar" x1="174" y1="56" x2="244" y2="116" marker-end="url(#mx-col-a)"/>
<line class="ar" x1="174" y1="84" x2="244" y2="74" marker-end="url(#mx-col-a)"/>
<line class="ar" x1="174" y1="140" x2="244" y2="132" marker-end="url(#mx-col-a)"/>
<line class="stop" x1="174" y1="112" x2="212" y2="112"/>
<line class="bar" x1="214" y1="105" x2="214" y2="119"/>
<text class="fl" x="212" y="44">matches both</text>
<text class="nt" x="14" y="176">the dashed sentence has no owner, and nothing reports it</text>
<text class="ti" x="370" y="22">parse each paragraph on its own</text>
<rect class="box" x="376" y="46" width="94" height="34" rx="3"/>
<rect class="box" x="376" y="110" width="94" height="34" rx="3"/>
<text class="pg" x="423" y="66">paragraph A</text>
<text class="pg" x="423" y="130">paragraph B</text>
<rect class="box" x="530" y="44" width="176" height="20" rx="3"/>
<rect class="box" x="530" y="68" width="176" height="20" rx="3"/>
<rect class="box" x="530" y="108" width="176" height="20" rx="3"/>
<rect class="box" x="530" y="132" width="176" height="20" rx="3"/>
<text class="sn" x="536" y="58">The system processes input now.</text>
<text class="sn" x="536" y="82">Tail one.</text>
<text class="sn" x="536" y="122">The system processes input now.</text>
<text class="sn" x="536" y="146">Tail two.</text>
<line class="ar" x1="472" y1="63" x2="526" y2="54" marker-end="url(#mx-col-a)"/>
<line class="ar" x1="472" y1="63" x2="526" y2="78" marker-end="url(#mx-col-a)"/>
<line class="ar" x1="472" y1="127" x2="526" y2="118" marker-end="url(#mx-col-a)"/>
<line class="ar" x1="472" y1="127" x2="526" y2="142" marker-end="url(#mx-col-a)"/>
<text class="nt" x="376" y="176">each sentence is the return value of the call on that paragraph</text>
</svg>

The two identical sentences are the ones from the regression fixture in `src/lib.rs`. On the left nothing in the text distinguishes them, so the assignment is a coin toss and one sentence is left over. On the right the arrows run the other way: the call produced the sentences, so the owner is known before any text is compared.

Per-paragraph parsing deletes the wiring step instead of improving it. A paragraph's sentences are the return value of the call made on that paragraph's text. The relationship comes from the call graph, so there is nothing left to get wrong.

Blockquote paragraphs are skipped at this stage, which is why they reach the end with no sentences and all three metric slots at `None`. And a parse failure in any single paragraph aborts that document with `Error::ParseFailed`; the partial document is dropped, not returned, and in a stream the failure travels as that document's `DocumentError` while the next document proceeds.

## Measure fills slots, extract is a separate call

The measure stage is four functions with one signature, `Box<dyn Fn(&mut Document)>`, run in sequence by `compose`. Readability, lexical density, and compression write per-paragraph slots. The document pass writes `vocabulary_ttr` and `nominalization_ratio`. None of the four reads another's output, so the suite is a list, not a chain.

Each carries its own applicability condition, and this is where unexplained `None` values come from:

| Slot | Written when |
|---|---|
| `readability_grade` | more than 10 tokens, not a blockquote |
| `lexical_density` | at least one token, not a blockquote |
| `compression_ratio` | more than 50 tokens, not a blockquote, paragraph under 256 KiB |
| `vocabulary_ttr`, `nominalization_ratio` | at least one non-punctuation token in the document |

`None` is not zero. A three-word paragraph has no meaningful Flesch-Kincaid grade, and the slot says so rather than reporting a number nobody should use. The compression cap is a CPU bound: the brotli window is 2^18 bytes, and a paragraph larger than one window is skipped instead of pegging a core on adversarial input.

Summarization and keyphrase extraction are not part of this. The pipeline never calls them. They take `&[Sentence]` and are invoked directly by the caller, on sentences read back off the tree:

```rust
let mut doc = engine.annotate(&raw)?;
let sentences: Vec<_> = doc.sentences().cloned().collect();
let summary = matra::extraction::tfidf_summarize(&sentences, 3)?;
let phrases = matra::extraction::rake_keyphrases(&sentences, 10)?;
```

Keeping them out of the pipeline is a cost decision. TextRank builds a dense sentence-similarity matrix that is quadratic in sentence count, roughly 32 MB of `f64` at its 2,000-sentence ceiling. Folding it into the analysis pass would charge that to every caller who only wanted a readability grade. Inputs above a cap return `Error::InputTooLarge` rather than allocating.

## What can fail, and where it surfaces

Every guard sits at the earliest point where the check is still cheap.

<svg class="mx-grd" role="img" aria-label="Three separate call paths with their guards: model construction, the pipeline, and the extractors" viewBox="0 0 720 245" width="720" height="245" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Where each guard fires, on which call path</title>
<style>
.mx-grd text{fill:currentColor}
.mx-grd .en{font-size:8.5px;text-anchor:end;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-grd .out{font-size:8px;opacity:.55;font-family:inherit}
.mx-grd .gd{font-size:8px;text-anchor:middle;font-family:inherit}
.mx-grd .er{font-size:8px;text-anchor:middle;opacity:.7;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-grd .lane{stroke:currentColor;opacity:.55;stroke-width:1.4px}
.mx-grd .tick{stroke:currentColor;opacity:.55;stroke-width:1px}
.mx-grd .cap{stroke:currentColor;opacity:.55;stroke-width:1.4px}
</style>
<text class="en" x="162" y="43">Udpipe::english(dir)</text>
<line class="lane" x1="170" y1="40" x2="470" y2="40"/>
<line class="cap" x1="170" y1="34" x2="170" y2="46"/>
<line class="cap" x1="470" y1="34" x2="470" y2="46"/>
<text class="out" x="478" y="43">once per process</text>
<line class="tick" x1="330" y1="40" x2="330" y2="52"/>
<text class="gd" x="330" y="64">size constant, then SHA-256</text>
<text class="er" x="330" y="75">Error::ModelInvalid</text>
<text class="en" x="162" y="133">analyze(Ingest::path(p)?)</text>
<line class="lane" x1="170" y1="130" x2="630" y2="130"/>
<line class="cap" x1="170" y1="124" x2="170" y2="136"/>
<line class="cap" x1="630" y1="124" x2="630" y2="136"/>
<text class="out" x="638" y="133">Document</text>
<line class="tick" x1="200" y1="130" x2="200" y2="144"/>
<text class="gd" x="200" y="156">symlink or non-regular file</text>
<text class="er" x="200" y="167">Error::Io</text>
<line class="tick" x1="270" y1="130" x2="270" y2="178"/>
<text class="gd" x="270" y="190">file size, on metadata</text>
<text class="er" x="270" y="201">Error::InputTooLarge</text>
<line class="tick" x1="350" y1="130" x2="350" y2="116"/>
<text class="gd" x="350" y="108">text size, in annotate</text>
<text class="er" x="350" y="97">Error::InputTooLarge</text>
<line class="tick" x1="490" y1="130" x2="490" y2="116"/>
<text class="gd" x="490" y="108">catch_unwind, once per paragraph</text>
<text class="er" x="490" y="97">Error::ParseFailed</text>
<text class="en" x="162" y="231">tfidf_summarize(..)</text>
<line class="lane" x1="170" y1="228" x2="470" y2="228"/>
<line class="cap" x1="170" y1="222" x2="170" y2="234"/>
<line class="cap" x1="470" y1="222" x2="470" y2="234"/>
<text class="out" x="478" y="231">ScoredSentence</text>
<line class="tick" x1="390" y1="228" x2="390" y2="214"/>
<text class="gd" x="390" y="206">per-extractor cap</text>
<text class="er" x="390" y="195">Error::InputTooLarge</text>
</svg>

Three separate lanes, because these are three separate call paths. Only the middle one runs during a pipeline call. The model check fires once when you construct the provider, and the extractor caps fire only when you call an extractor, which is why neither shows up in a pipeline stack trace.

The `catch_unwind` is the one that matters. A panic crossing an FFI boundary does not unwind into your code, it aborts the process: interpreter death in Python, a trap in WASM, from a library that promised typed errors.

The boundary that converts such a panic into an `Error` lives in `nlp/udpipe.rs`, and that file is the only one permitted to import the UDPipe crate. A second import path would be a second process-abort surface with no boundary in front of it. That boundary covers parsing only; model loading translates the loader's own error and is not wrapped.

Two guards live in the domain types rather than at an edge, because a parse arrives from outside and its shape is not guaranteed. `Sentence::tree_depth` walks head references with a visited set and returns `usize::MAX` when they form a cycle. `Sentence::subtree` carries the same visited set so it terminates. The sentinel is deliberate: an earlier version capped depth at a magic 20, which silently truncated malformed parses and legitimate deep ones alike.

Directory reads fail differently. The stream yields successes and per-document failures side by side, so one unreadable file does not cost you the other ninety-nine; `Ingest::path` is `Err` only when the listing itself fails. Full detail on every gate, every variant, and the Python exception each maps to is in [Errors](../reference/errors.md).

## What you can replace

<svg class="mx-mod" role="img" aria-label="Module map of the matra crate: the launchers and the command line above lib.rs and config.rs, those above the adapters, adapters above the ports, ports above domain.rs, with every dependency arrow pointing down" viewBox="0 0 720 345" width="720" height="345" style="max-width:100%;height:auto;display:block;margin:1.7em auto">
<title>Which module may import which, in the matra crate</title>
<style>
.mx-mod text{fill:currentColor}
.mx-mod .fp{font-size:9px;text-anchor:middle;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-mod .tr{font-size:8px;text-anchor:middle;opacity:.75;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.mx-mod .sub{font-size:7.5px;text-anchor:middle;opacity:.55;font-family:inherit}
.mx-mod .nt{font-size:8px;opacity:.55;font-family:inherit}
.mx-mod .box{fill:none;stroke:currentColor;opacity:.42;stroke-width:1px}
.mx-mod .band{fill:currentColor;fill-opacity:.05;stroke:currentColor;stroke-opacity:.42;stroke-width:1px}
.mx-mod .dep{stroke:currentColor;opacity:.6;stroke-width:1.1px;fill:none}
.mx-mod marker path{fill:currentColor;opacity:.6}
</style>
<defs><marker id="mx-mod-a" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0 L8,4 L0,8 z"/></marker></defs>
<rect class="box" x="16" y="8" width="196" height="30" rx="4"/>
<text class="fp" x="114" y="22">src/bin/matra.rs</text>
<text class="sub" x="114" y="33">launcher, feature: cli</text>
<text class="nt" x="215" y="19">calls run()</text>
<line class="dep" x1="212" y1="23" x2="264" y2="23" marker-end="url(#mx-mod-a)"/>
<rect class="box" x="268" y="8" width="212" height="30" rx="4"/>
<text class="fp" x="374" y="22">src/cli/</text>
<text class="sub" x="374" y="33">the command line, feature: cli</text>
<line class="dep" x1="114" y1="38" x2="114" y2="54" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="374" y1="38" x2="374" y2="54" marker-end="url(#mx-mod-a)"/>
<rect class="band" x="16" y="58" width="700" height="36" rx="4"/>
<text class="fp" x="264" y="73">src/lib.rs</text>
<text class="sub" x="264" y="85">declares every module, and is the only file that names every adapter and every port</text>
<rect class="box" x="524" y="60" width="188" height="32" rx="4"/>
<text class="fp" x="618" y="73">src/config.rs</text>
<text class="sub" x="618" y="85">locations and defaults, never behavior</text>
<line class="dep" x1="264" y1="94" x2="264" y2="102"/>
<line class="dep" x1="80" y1="102" x2="648" y2="102"/>
<line class="dep" x1="80" y1="102" x2="80" y2="116" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="222" y1="102" x2="222" y2="116" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="364" y1="102" x2="364" y2="116" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="506" y1="102" x2="506" y2="116" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="648" y1="102" x2="648" y2="116" marker-end="url(#mx-mod-a)"/>
<rect class="box" x="16" y="120" width="128" height="26" rx="4"/>
<text class="fp" x="80" y="137">source/directory.rs</text>
<rect class="box" x="40" y="160" width="104" height="26" rx="4"/>
<text class="fp" x="92" y="177">source/file.rs</text>
<line class="dep" x1="90" y1="146" x2="90" y2="158" marker-end="url(#mx-mod-a)"/>
<text class="nt" x="96" y="156">delegates</text>
<rect class="box" x="158" y="120" width="128" height="26" rx="4"/>
<text class="fp" x="222" y="136">decompose/markdown.rs</text>
<rect class="box" x="182" y="160" width="104" height="26" rx="4"/>
<text class="fp" x="234" y="177">decompose/plain.rs</text>
<rect class="box" x="300" y="120" width="128" height="46" rx="4"/>
<text class="fp" x="364" y="134">nlp/udpipe.rs</text>
<text class="sub" x="364" y="146">the only importer of udpipe_rs</text>
<text class="sub" x="364" y="158">feature: udpipe</text>
<rect class="box" x="442" y="120" width="128" height="46" rx="4"/>
<text class="fp" x="506" y="134">embed/model2vec.rs</text>
<text class="sub" x="506" y="146">sole importer of safetensors,</text>
<text class="sub" x="506" y="157">tokenizers · feature: model2vec</text>
<rect class="box" x="584" y="120" width="132" height="26" rx="4"/>
<text class="fp" x="650" y="137">metrics/</text>
<rect class="box" x="600" y="160" width="116" height="26" rx="4"/>
<text class="fp" x="658" y="177">extraction/</text>
<line class="dep" x1="28" y1="146" x2="28" y2="206" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="92" y1="186" x2="92" y2="206" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="170" y1="146" x2="170" y2="206" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="234" y1="186" x2="234" y2="206" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="364" y1="166" x2="364" y2="206" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="506" y1="166" x2="506" y2="206" marker-end="url(#mx-mod-a)"/>
<rect class="box" x="16" y="210" width="128" height="46" rx="4"/>
<text class="fp" x="80" y="226">source/mod.rs</text>
<text class="tr" x="80" y="238">trait Source</text>
<text class="sub" x="80" y="250">chosen statically in lib.rs</text>
<rect class="box" x="158" y="210" width="128" height="46" rx="4"/>
<text class="fp" x="222" y="226">decompose/mod.rs</text>
<text class="tr" x="222" y="238">trait Decomposer</text>
<text class="sub" x="222" y="250">chosen statically in lib.rs</text>
<rect class="box" x="300" y="210" width="128" height="46" rx="4"/>
<text class="fp" x="364" y="226">nlp/mod.rs</text>
<text class="tr" x="364" y="238">trait NlpProvider</text>
<text class="sub" x="364" y="250">&amp;dyn, chosen at runtime</text>
<rect class="box" x="442" y="210" width="128" height="46" rx="4"/>
<text class="fp" x="506" y="226">embed/mod.rs</text>
<text class="tr" x="506" y="238">trait Embedder</text>
<text class="sub" x="506" y="250">&amp;dyn, chosen at runtime</text>
<text class="sub" x="650" y="232">no port, no trait</text>
<line class="dep" x1="592" y1="146" x2="592" y2="286" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="708" y1="186" x2="708" y2="286" marker-end="url(#mx-mod-a)"/>
<text class="sub" x="650" y="270">domain and stopwords only</text>
<line class="dep" x1="80" y1="256" x2="80" y2="286" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="222" y1="256" x2="222" y2="286" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="364" y1="256" x2="364" y2="286" marker-end="url(#mx-mod-a)"/>
<line class="dep" x1="506" y1="256" x2="506" y2="286" marker-end="url(#mx-mod-a)"/>
<rect class="band" x="16" y="290" width="700" height="36" rx="4"/>
<text class="fp" x="366" y="306">src/domain.rs</text>
<text class="sub" x="366" y="319">serde · thiserror · std, and nothing else</text>
</svg>

Every arrow points down, and that is the rule: a module imports from the row below it, never the row above. Two arrows run sideways. `source/directory.rs` reaches into `source/file.rs`, inside a single port, so a directory read inherits the same symlink and size guards rather than reimplementing them. And `src/bin/matra.rs` reaches into `src/cli/`, which is the whole of that file: the command line lives in the library so both launchers run one program, and the binary only collects arguments, locks the streams, and returns the code.

`src/config.rs` sits in the composition-root row beside `lib.rs`, and it is the one box an adapter is allowed to import from the row above. `Udpipe::from_config` and `Model2Vec::from_config` take a `&Config`, which is the traffic ADR-0011 intends and [boundary rule 7](../reference/boundary-rules.md) records: `Config` imports no port and no adapter, so nothing travels back down with it, and an adapter that reads it is reading a resolved path rather than reaching for the environment itself.

`Config` is not one of the extension points below, and the distinction is worth keeping. An extension point substitutes an implementation; `Config` only supplies values a caller could have passed as arguments. That is why it carries locations and defaults and never behavior: the moment a file selects which metrics run, configuration stops being adaptability and becomes dispatch that nothing type-checks.

Three extension points are dispatched at runtime. `NlpProvider` has a single method, and the engine holds it as `Box<dyn NlpProvider>`. Implement it and nothing downstream changes, because nothing downstream of parse knows which provider ran. `Embedder` is the same shape one tier over: `embed_and_cluster` takes it as `&dyn Embedder`, and its `identity` travels into every result so the scores stay attributable to the model that produced them. That is what makes UDPipe an implementation detail rather than the library. And `Decomposer` dispatch is a value: the engine's `Decomposers` table maps each `Format` to a boxed decomposer, `standard_decomposers()` is merely the table this build ships, and `Decomposers::new().with(format, decomposer)` builds a different one. Registering a format is data flow, not a code change.

`Source` stays static: `Ingest`'s constructors name the file and directory adapters. To ingest from somewhere else, construct `RawDocument` values yourself and feed them to `Engine::analyze` as `Ok` items; the pipeline does not care where they came from.

The metric suite is data as well. `Metric`, `run_suite`, and `Document::new` are all public, so a caller can assemble a different suite and run it over a document. `compose` always runs the default four.

What you gain from the arrangement is concrete and mostly shows up in test suites. Nothing under `metrics/` or `extraction/` imports a port, which is the one column above that skips the port row entirely, so those functions run with no model loaded. A new metric is testable against hand-built `Sentence` values, with no 16 MB download and no C++ toolchain. The rules that keep it that way, and how weakly some of them are enforced, are listed in [Boundary rules](../reference/boundary-rules.md).

## Crossing into Python copies everything

The Rust side runs the same pipeline, then hands the `Document` to `pythonize`, which walks the serde representation and builds a Python dict. That is a full deep copy. Every token becomes a dict of eleven keys. For a document with seven thousand tokens, that is seven thousand dicts allocated on the Python heap.

Fields cross. Methods do not. `Document::passive_ratio()`, `mean_sentence_length()`, and every other computed value is a Rust method with no serde representation, so it has nothing to cross with. A Python caller recomputes them from the `sections` data already in hand. [Domain types](../reference/domain-types.md#what-crosses-the-language-boundary) draws that boundary member by member.

Errors cross by type. The conversion is a match with no wildcard arm, so adding a variant to `domain::Error` fails to compile until someone decides which Python exception class it becomes. A wildcard would let new failure modes fall through to `RuntimeError` unnoticed.

Every Python method that takes text routes through the same pipeline, so the 8 MiB text cap fires uniformly. The extraction methods add their per-extractor caps on top.

## Where to look next

[What matra gives you](../capabilities.md) is the output side: every value the pipeline produces and what each one measures.

[Domain types](../reference/domain-types.md) is the type graph: what each type owns, which values are stored and which are computed on demand.

[Boundary rules](../reference/boundary-rules.md) is the enforcement side: the eight rules, why each exists, and which of them have a mechanical gate.
