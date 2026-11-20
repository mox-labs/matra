# EPR-0008: The limits as one value with a default

- Feature Name: `limits_value`
- Start Date: 2026-11-19
- Pinned at: `beb07edc5967be0e0a7ed6cfee1fc9602ffbfb04` (every code citation below opens at this commit)
- Proposal PR: [#195](https://github.com/mox-labs/matra/pull/195)
- Tracking issue: [#196](https://github.com/mox-labs/matra/issues/196)
- Status: proposed

## Summary

matra holds its work to eleven bounds, each a constant compiled into the crate, and no caller can change one. This proposal makes six of them one value, `Limits`, whose default is today's numbers: the input cap and the five caps the outcome functions check. An engine carries a `Limits`, and the outcome functions take one in their `_with` forms, so a server can ask for less, a batch job on a large machine for more, and a browser build can start lower, while every enforcement point stays where it is and every refusal stays the error it is today. The other five bounds are not limits a caller should move: three decide whether a metric is measured, one guards a download whose size is pinned, and one guards the file the configuration would come from. They stay fixed and leave the public surface.

The first milestone changes nothing a caller sees: one value, its default, and every enforcement point reading it. Publishing it, in Rust and Python and then in the configuration file, is the owner's decision, and the first question below.

<pragmatics>

<ask>

Rule on the seven decisions under [Unresolved questions](#unresolved-questions), the first of which is whether the value is published now or kept inside the crate. Then accept the proposal by merging a pull request that changes its status to `accepted`.

</ask>

<will>

- Record each ruling in its decision, as a named step in a pull request.
- Keep the tracking issue, [#196](https://github.com/mox-labs/matra/issues/196), current with the [milestones](#milestones).
- After acceptance, build milestone 1 whatever decision 1 says, since it changes nothing a caller sees, and the later milestones only as decision 1 rules.
- Repeat the measurements under [What was measured](#what-was-measured) in a test or a recorded script when the milestone that relies on them lands.

</will>

<needs>

- Your rulings. Decision 1 decides how much of the rest is built.
- A reading with one caveat in mind: the measurements below are single runs of the command line on one machine, taken by Claude for this proposal. Until a milestone records them where a reader can rerun them, they are a reading, not a test.

</needs>

<wont>

- Build anything while the proposal is `proposed`, change its status, or merge a pull request that does.
- Change a default value. Every default stays the number it is today; a new default is its own measured change.
- Edit EPR-0007. Where this proposal changes what EPR-0007 sketches, it says so here, and the change is made in EPR-0007 when the owner has ruled on both.

</wont>

<silence>

The proposal stays `proposed` and nothing is built. The limits stay constants, and EPR-0007's TypeScript sketch keeps its `limits` constant. Claude raises it again at the start of the next session that works on EPR-0007's milestones 4 or 5, which build the TypeScript core and engine, because those would publish the constants in a fourth language.

</silence>

</pragmatics>

## Motivation

### The bounds today

Every bound is a constant in one module, `domain::limits`. <claim id="constants-not-settings" basis="observed">Its documentation says so in as many words: "Constants, not settings: no configuration, argument or environment variable changes any of them" [domain.rs:24](../../src/domain.rs#L24 "Constants, not settings: no configuration, argument or environment")</claim>. <claim id="error-shape" basis="observed">A cap that is exceeded returns `InputTooLarge`, whose details are `what`, `limit` and `actual`, with `what` a label fixed in the source [domain.rs:303](../../src/domain.rs#L303 "what: &'static str,") [domain.rs:425](../../src/domain.rs#L425 ".integer(\"limit\", *limit)")</claim>.

| Bound | Value | Defined at | Checked at | Over it |
|---|---|---|---|---|
| `MAX_INPUT_BYTES` | 8 MiB (8,388,608) | [domain.rs:50](../../src/domain.rs#L50 "pub const MAX_INPUT_BYTES") | the engine, on the bytes handed in and on the analysed text [lib.rs:46](../../src/lib.rs#L46 "if len > MAX_INPUT_BYTES"), called from [lib.rs:545](../../src/lib.rs#L545 "check_input_size(text.len())?;"), [lib.rs:561](../../src/lib.rs#L561 "check_input_size(text.len())?;") and [lib.rs:602](../../src/lib.rs#L602 "check_input_size(bytes.len())?;"); the file source, on the file's size before reading [file.rs:62](../../src/source/file.rs#L62 "if len > MAX_INPUT_BYTES as u64"); the command line, on stdin as it arrives [cli/mod.rs:1160](../../src/cli/mod.rs#L1160 ".take(MAX_INPUT_BYTES as u64 + 1)") | error, `what` is `input` or `file_read` |
| `MAX_TFIDF_SENTENCES` | 2,000 | [domain.rs:78](../../src/domain.rs#L78 "pub const MAX_TFIDF_SENTENCES") | [tfidf.rs:24](../../src/extraction/tfidf.rs#L24 "if sentences.len() > MAX_SENTENCES") | error, `tfidf` |
| `MAX_TEXTRANK_SENTENCES` | 2,000 | [domain.rs:85](../../src/domain.rs#L85 "pub const MAX_TEXTRANK_SENTENCES") | [textrank.rs:32](../../src/extraction/textrank.rs#L32 "if sentences.len() > MAX_SENTENCES") | error, `textrank` |
| `MAX_CLUSTER_SENTENCES` | TextRank's, by definition | [domain.rs:94](../../src/domain.rs#L94 "pub const MAX_CLUSTER_SENTENCES: usize = MAX_TEXTRANK_SENTENCES;") | before anything is embedded [lib.rs:811](../../src/lib.rs#L811 "texts.len() > domain::limits::MAX_CLUSTER_SENTENCES"), and on the vectors [semantic.rs:47](../../src/extraction/semantic.rs#L47 "if embeddings.len() > MAX_SENTENCES") | error, `clusters` |
| `MAX_RAKE_TOKENS` | 200,000 | [domain.rs:103](../../src/domain.rs#L103 "pub const MAX_RAKE_TOKENS") | [rake.rs:37](../../src/extraction/rake.rs#L37 "if total_tokens > MAX_TOKENS") | error, `rake` |
| `MAX_YAKE_TOKENS` | 200,000 | [domain.rs:112](../../src/domain.rs#L112 "pub const MAX_YAKE_TOKENS") | [yake.rs:31](../../src/extraction/yake.rs#L31 "if total_tokens > MAX_TOKENS") | error, `yake` |
| `MAX_CONFIG_FILE_BYTES` | 64 KiB (65,536) | [domain.rs:58](../../src/domain.rs#L58 "pub const MAX_CONFIG_FILE_BYTES") | on the file's size before reading [config.rs:765](../../src/config.rs#L765 "if metadata.len() > MAX_CONFIG_BYTES") | error, `config_file_read` |
| `MAX_DOWNLOAD_BYTES` | 64 MiB (67,108,864) | [domain.rs:69](../../src/domain.rs#L69 "pub const MAX_DOWNLOAD_BYTES") | the parsing model's download [udpipe.rs:325](../../src/nlp/udpipe.rs#L325 "if bytes.len() > MAX_MODEL_BYTES") and each embedding artifact's [model2vec.rs:999](../../src/embed/model2vec.rs#L999 "if bytes.len() > MAX_ARTIFACT_BYTES") | error, `parser_model_download` or `embedding_model_download` |
| `MAX_COMPRESSION_PARAGRAPH_BYTES` | 256 KiB (262,144) | [domain.rs:121](../../src/domain.rs#L121 "pub const MAX_COMPRESSION_PARAGRAPH_BYTES") | [compression.rs:35](../../src/metrics/compression.rs#L35 "para.text.len() <= MAX_COMPRESSION_PARAGRAPH_BYTES") | no error: `compression_ratio` stays `None` |
| `MIN_READABILITY_WORDS` | 11 | [domain.rs:127](../../src/domain.rs#L127 "pub const MIN_READABILITY_WORDS") | [readability.rs:11](../../src/metrics/readability.rs#L11 "para.word_count() >= MIN_READABILITY_WORDS") | no error: `readability_grade` stays `None` |
| `MIN_COMPRESSION_WORDS` | 51 | [domain.rs:133](../../src/domain.rs#L133 "pub const MIN_COMPRESSION_WORDS") | [compression.rs:33](../../src/metrics/compression.rs#L33 "para.word_count() >= MIN_COMPRESSION_WORDS") | no error: `compression_ratio` stays `None` |

<claim id="python-limits" basis="observed">Python re-exports all eleven as `matra.limits`, read from the extension so a value cannot drift [mod.rs:59](../../src/python/mod.rs#L59 "fn add_limits(") [limits.py:20](../../python/matra/limits.py#L20 "MAX_INPUT_BYTES: Final[int] = _core.MAX_INPUT_BYTES")</claim>. The command line has no flag for any of them and the configuration file no key: <claim id="config-keys" basis="observed">the file's sections are `models`, `summary`, `keyphrases` and `clusters`, and an unknown key fails to load [config.rs:491-497](../../src/config.rs#L491-L497 "deny_unknown_fields")</claim>. <claim id="skill-limits" basis="observed">The agent skill states the caps as fixed numbers [SKILL.md:121](../../skills/matra/SKILL.md#L121 "Input is capped at 8 MiB")</claim>.

### Why the caps exist, and what is recorded about their cost

<claim id="floor-bounded" basis="observed">The resilience floor's rule is that unbounded input is unbounded resource use, and "the cap is a feature, not a limitation" [SKILL.md:174](../../.claude/skills/resilience-floor/SKILL.md#L174 "The cap is a feature, not a limitation.")</claim>. Each constant's documentation gives the cost model behind its value: <claim id="cost-models" basis="observed">TF-IDF about 2 MiB at its cap, TextRank's dense matrix about 32 MB, RAKE about 3 MiB and YAKE about 40 MiB [domain.rs:75](../../src/domain.rs#L75 "60k entries (~2 MiB) at this size") [domain.rs:83](../../src/domain.rs#L83 "quadratic in sentences: about 32 MB of") [domain.rs:102](../../src/domain.rs#L102 "about 3 MiB") [domain.rs:110](../../src/domain.rs#L110 "about 40 MiB")</claim>. These are estimates written beside the code. No measurement of them, with a method and a machine, was found in the repository.

<claim id="gib-stated" basis="observed">The input cap's reason is stated in three places, each saying that past 8 MiB UDPipe's allocations cross about a gigabyte resident [domain.rs:49](../../src/domain.rs#L49 "per-token allocations cross ~1 GiB resident at this size).") [0008-pipeline-surface.md:31](../legacy/eps/0008-pipeline-surface.md#L31 "roughly a gigabyte resident") [test_size_cap.py:10-11](../../python/tests/test_size_cap.py#L10-L11 "cross roughly a gigabyte resident")</claim>. None of them records how it was measured, on what, or when, so this proposal does not rest on it. It measured instead ([What was measured](#what-was-measured)): <claim id="rss-one-mib" basis="observed">analysing an 18 KB input through the command line peaked at 219 MB resident with the model loaded, a 1 MiB input at 463 MB, and an input just under 8 MiB at 2,663 MB, taking 522 seconds [measured](#what-was-measured)</claim>. <claim id="rss-rate" basis="inferred">That is about 245 to 305 MB for each MiB of input beyond the model, close to linear, so the cost at the cap is more than twice the gigabyte the code states. The command line holds the document and writes its JSON, so part of the difference may be the command line's own; how much the library alone holds was not measured</claim>.

<claim id="raise-deferred" basis="observed">EPR-0006 published the constants and left raising one out of 0.3.0, because "the resilience floor keeps safe defaults until its own reviewer rules" [EPR-0006:532](0006-the-0-3-0-surface.md#L532 "Raising a cap is left out of this release")</claim>. This proposal runs that review ([The boundary test and the resilience floor](#the-boundary-test-and-the-resilience-floor)). It also changes two things EPR-0006 decided: that the bounds are constants, and that <claim id="published-ruling" basis="observed">every one of them is published, which the owner ruled because "a caller needs them before the call" [EPR-0006:150](0006-the-0-3-0-surface.md#L150 "The limits are published because a caller needs them before the call")</claim>. Decision 3 keeps that reason for the six caps and argues it does not hold for the other five.

### A document matra analyses and will not summarise

<claim id="novel-promise" basis="observed">The input cap is sized so that "8 MiB holds book-length English" [domain.rs:46](../../src/domain.rs#L46 "8 MiB holds book-length English")</claim>. The outcome caps are not. <claim id="summarize-refused" basis="observed">A 1,037,098-byte text, analysed without complaint, was refused by `matra summarize` with `tfidf input too large: 5104 > limit 2000`, at the end of a 66-second run [measured](#what-was-measured)</claim>, and <claim id="keyphrases-refused" basis="observed">a 1,072,860-byte text was refused by `matra keyphrases` with `rake input too large: 200820 > limit 200000` [measured](#what-was-measured)</claim>. <claim id="cap-in-bytes" basis="inferred">Over the twelve example texts in `site/inputs/` (34,236 bytes, 195 sentences, 6,529 tokens) a sentence averages about 176 bytes and a byte about 0.19 tokens, so the summary caps are reached near 350 KB of such prose and the keyphrase caps near 1 MB, against an input cap of 8 MiB</claim>. A caller who has measured their machine and can afford more has no way to say so: only matra can raise a cap.

### Who needs a different limit

Use cases first. Each was tested against the question "could the caller do this without matra's help?"

| Caller | Wants | Without matra's help |
|---|---|---|
| A server analysing text from strangers | A lower input cap, so one request costs at most what a worker's memory and time allow; lower outcome caps for the same reason | Mostly possible: it can check a text's length, a file's size or a document's sentence count before the call. But it restates matra's own measures (tokens counted with punctuation, sentences across every paragraph) in its own code, and its refusal is not matra's error. Over `Inputs::dir` it cannot check a file before the engine reads and parses it, because <claim id="input-path-private" basis="observed">an `Input`'s path is private [lib.rs:171](../../src/lib.rs#L171 "fn path(&self) -> Option<&Path>")</claim>; it would have to list the directory itself. |
| A batch job on a large machine | Higher caps, to summarise and extract keyphrases from book-length documents | Impossible. A cap only matra checks only matra can raise. |
| A browser build (EPR-0007) | A lower default, because a page's memory is tighter than a workstation's | Impossible for the package's own default; a page could check lengths itself. How much lower is not measured ([What was not measured](#what-was-not-measured)). |
| Someone running the command line on one large file | A higher input cap for that run or that machine | Impossible today. |

Nothing in the repository's issues asks for any of these: <claim id="no-issue" basis="observed">none of the repository's issues, open or closed, asks for a different limit [issues](https://github.com/mox-labs/matra/issues?q=is%3Aissue)</claim>. The batch row is the measured gap above. The other two are reasoned: <claim id="server-case" basis="assumed">a caller serving strangers would set a lower cap through matra if it could, rather than checking sizes in its own code</claim>, and <claim id="browser-case" basis="assumed">a browser build will need a lower default than a workstation's</claim>.

### Why now

<claim id="epr7-constants" basis="observed">EPR-0007, proposed, sketches TypeScript's `limits` as constants "generated from the Rust constants when the package is built" [EPR-0007:319](0007-typescript-and-webassembly.md#L319 "const limits: { readonly maxInputBytes: number") [EPR-0007:330](0007-typescript-and-webassembly.md#L330 "is generated from the Rust constants when the package is built")</claim>. If the limits are going to become a value, the cheapest moment is before a fourth language publishes them as constants.

## Guide-level explanation

### One value, with a default

`Limits` is the set of caps matra checks before it does work whose cost grows with the input. `Limits::default()` is the set every engine and every outcome function uses unless told otherwise, and its numbers are today's.

```rust
use matra::domain::Limits;
use matra::{Engine, Input, SummaryMethod};

// A server: less than the default, for every input it reads.
let mut limits = Limits::default();
limits.input_bytes = 1024 * 1024;
let engine = Engine::new()?.with_limits(limits);

// A batch job: more than the default, for a book.
let mut book = Limits::default();
book.input_bytes = 32 * 1024 * 1024;
book.tfidf_sentences = 50_000;
let engine = Engine::new()?.with_limits(book);
let doc = engine.analyze(Input::file("novel.txt"))?;
let summary = matra::summarize_with(&doc, 10, SummaryMethod::TfIdf, engine.limits())?;
```

A cap is a field named for the gate it holds and the unit it counts, and the gate is the `what` an error already names: over `tfidf_sentences`, the error says `what: "tfidf"` and `limit` is the cap in force, not the default.

| Field | Default | `what` when exceeded | Counts |
|---|---|---|---|
| `input_bytes` | 8,388,608 | `input`, `file_read` | bytes of the input and of the analysed text; a file's size before it is read |
| `tfidf_sentences` | 2,000 | `tfidf` | sentences in the document |
| `textrank_sentences` | 2,000 | `textrank` | sentences in the document |
| `cluster_sentences` | 2,000 | `clusters` | sentences in the document, and the vectors |
| `rake_tokens` | 200,000 | `rake` | tokens across every sentence, punctuation included |
| `yake_tokens` | 200,000 | `yake` | tokens, as for RAKE |

### Where the value arrives

The engine's limits govern everything the engine reads: every `Input`, every file a source reads for it, every analysed text. The outcome functions take no engine, so they cannot read the engine's: the plain forms (`summarize`, `keyphrases`, `clusters`) use the default, and the `_with` forms take the limits as their last argument. A caller who raised the engine's caps and then calls a plain form is refused by the plain form's default cap, with an error naming that gate; passing `engine.limits()` to the `_with` form is the remedy.

<sketch-figure id="arrival" seed="8" title="Where the one value is read: the engine for what it reads, the outcome functions for what they rank">

```text
width 400
state proposed proposed "one value, passed to the engine and to each outcome call"
box caller "Caller's code,\nor the config\nfile's [limits]" 2 52 104 56
box limits "Limits\nsix caps,\ndefault today's" 132 52 104 56
box engine "Engine\ninput_bytes:\nevery input, file\nand analysed text" 262 2 136 62
box outcomes "_with forms\nsummary, keyphrase\nand cluster caps" 262 92 136 56
edge caller limits
edge limits engine
edge limits outcomes
```

</sketch-figure>

### What a limit never changes

A limit decides whether a call runs, never what it returns. A document analysed under raised limits is the same document, byte for byte, as one analysed under the default, whenever both succeed, and a summary is the same summary. That is why a document does not record the limits it was made under, and why the bounds that would change a document's content are not limits.

### What stays fixed

| Bound | Why it is not a limit you set |
|---|---|
| The fewest words for a readability grade (11) and for a compression ratio (51), and the largest paragraph measured for compression (256 KiB) | They decide whether a metric has a value. Moving one changes what a document says, not whether it is made. The methodology reference states each as the metric's applicability. |
| The largest model download (64 MiB) | It bounds what a misbehaving server can make the process hold, for artifacts whose size is pinned. No caller's work moves it. |
| The largest configuration file (64 KiB) | It guards the file the configuration would come from, so it cannot be set there. |

They stop being public names. Their values are documented where they apply, in the methodology and errors references.

### Python

```python
import matra

engine = matra.Engine(limits=matra.Limits(input_bytes=1024 * 1024))
doc = engine.analyze(text)
summary = matra.summarize(doc, 10, method="tfidf", limits=engine.limits)

matra.Limits().input_bytes   # 8388608, the default, read from the extension
```

`matra.Limits` takes each cap as an optional keyword and reads the rest from the default. `Engine.from_config` reads the configuration file's `[limits]`. `matra.limits`, the module of eleven constants, is gone: the six caps are `matra.Limits()`'s attributes, and the five fixed bounds are documented, not exported.

### The configuration file and the command line

```toml
[limits]
input_bytes = 33554432
tfidf_sentences = 50000
```

Each key is optional and overrides its default, as the other sections do. The command line reads the file it already reads, applies the caps to stdin as it arrives and to every outcome command, and `matra config show` lists each cap with where its value came from. There is no flag and no environment variable.

### TypeScript

If EPR-0007 is accepted, its package inherits the same value: an optional `limits` on `Engine.load` and on each outcome's options, its fields the Rust names in TypeScript's case (`inputBytes`), and the default exported as one frozen object generated from `Limits::default()` when the package is built, in place of EPR-0007's `limits` constant.

## Reference-level explanation

### The signatures

```rust
// domain.rs: beside the types the library hands back, so extraction may read it (rule 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    pub input_bytes: usize,
    pub tfidf_sentences: usize,
    pub textrank_sentences: usize,
    pub cluster_sentences: usize,
    pub rake_tokens: usize,
    pub yake_tokens: usize,
}
impl Default for Limits { /* today's values, each with its cost model in its doc */ }

// lib.rs
impl Engine {
    pub fn with_limits(self, limits: Limits) -> Engine;
    pub fn limits(&self) -> Limits;
}
pub fn summarize(doc: &Document, n: usize) -> Result<Summary>;                    // the default
pub fn summarize_with(doc: &Document, n: usize, method: SummaryMethod, limits: Limits) -> Result<Summary>;
pub fn keyphrases(doc: &Document, n: usize) -> Result<Keyphrases>;               // the default
pub fn keyphrases_with(doc: &Document, n: usize, method: KeyphraseMethod, limits: Limits) -> Result<Keyphrases>;
pub fn clusters(doc: &Document, embedder: &dyn Embedder, threshold: f64) -> Result<Clusters>;   // the default
pub fn clusters_with(doc: &Document, embedder: &dyn Embedder, threshold: f64, limits: Limits) -> Result<Clusters>;

// source/mod.rs: the port is told the cap it may refuse at (decision 7).
pub trait Source: Send {
    fn read(&self, path: &Path, limit: usize) -> Result<Vec<u8>>;
}

// config.rs
impl Config { pub fn limits(&self) -> Limits; }   // the [limits] table over the default
```

```python
@final
class Limits:
    def __init__(self, *, input_bytes: int = ..., tfidf_sentences: int = ..., textrank_sentences: int = ...,
                 cluster_sentences: int = ..., rake_tokens: int = ..., yake_tokens: int = ...) -> None: ...
    @property
    def input_bytes(self) -> int: ...   # and the other five, each read-only

class Engine:
    def __new__(cls, *, limits: Limits | None = None) -> Engine: ...
    @property
    def limits(self) -> Limits: ...

def summarize(doc, n=3, *, method=None, limits: Limits | None = None) -> Summary: ...
def keyphrases(doc, n=10, *, method=None, limits: Limits | None = None) -> Keyphrases: ...
def clusters(doc, embedder, threshold=0.85, *, limits: Limits | None = None) -> Clusters: ...
```

The fields are public and the struct is `#[non_exhaustive]`, so a caller outside the crate reads and sets fields on a default and cannot build one from a literal, and a seventh cap is an addition. No setters: a public field already is one. `Limits` carries no method a caller needs across a language boundary; its fields are the whole of it, which is what lets Python and TypeScript mirror it as attributes and an object.

Names carry no implementation: each cap is named for the gate a caller selects and the unit the caller can count. The method names in four of them are the selectors' names, which a caller already sends (decision 4).

### Where each cap is read

| Cap | Read by | How it gets there |
|---|---|---|
| `input_bytes` | the engine's size check, at the three places it runs today | the engine's own field |
| `input_bytes` | the file source, before reading | the engine passes it to `Source::read` |
| `input_bytes` | the command line's stdin read | `Config::limits()`, the same value it builds the engine from |
| the five outcome caps | the four method modules and both cluster checks | an argument from the `_with` form, which the plain form fills with the default |

<claim id="annotate-only-route" basis="observed">The engine's reading stage is the only route from text to the parser [lib.rs:541](../../src/lib.rs#L541 "pub(crate) fn annotate(")</claim>, so the input cap stays a property of the pipeline: one value, read at one choke point. Nothing moves; each check reads a value it is handed instead of a constant it imports. <claim id="epr7-new-check" basis="observed">EPR-0007 now proposes one more check, holding the paragraphs' texts together to the input cap [EPR-0007:271](0007-typescript-and-webassembly.md#L271 "The cap on what reaches the parser.")</claim>; if it is accepted, it reads `input_bytes` too, and whichever of the two lands second moves it.

### The invariant, and its test

A limit decides whether, never what. Two tests hold it from milestone 1:

- Each gate, under a `Limits` lowered below a fixture's size, fails with that gate's `what` and with `limit` equal to the lowered value. This is the test that every enforcement point reads the value: a check still reading a constant fails it.
- The same input analysed and then summarised, under the default and under every cap raised, gives equal documents and equal summaries.

The second is why the three metric bounds are outside `Limits`. <claim id="compression-is-content" basis="observed">The compression bound and the two minimums decide whether a paragraph's metric has a value [compression.rs:33-35](../../src/metrics/compression.rs#L33-L35 "MIN_COMPRESSION_WORDS") [readability.rs:11](../../src/metrics/readability.rs#L11 "MIN_READABILITY_WORDS")</claim>, so moving one would make two documents of one text differ, and the invariant would not hold.

### The fixed bounds

`MAX_CONFIG_FILE_BYTES`, `MAX_DOWNLOAD_BYTES`, `MAX_COMPRESSION_PARAGRAPH_BYTES`, `MIN_READABILITY_WORDS` and `MIN_COMPRESSION_WORDS` stay constants, made crate-private, with their documentation and the value test kept. <claim id="methodology-states" basis="observed">The methodology reference already states the metric bounds as each metric's applicability [methodology.md:74](../../site/content/reference/methodology.md#L74 "whose `word_count` is above 10") [methodology.md:110](../../site/content/reference/methodology.md#L110 "whose `word_count` is above 50, and whose text is at most 262,144 bytes")</claim>, and the errors reference states the download and configuration caps with their gates.

One defect found on the way, corrected in milestone 2 when these docs move: <claim id="minimum-doc-wrong" basis="observed">the two minimums' documentation says the words are "counted by whitespace" [domain.rs:124](../../src/domain.rs#L124 "(more than 10), counted by whitespace."), but both checks call `word_count`, which counts the parse's non-punctuation tokens [domain.rs:1474](../../src/domain.rs#L1474 "let word_count = tokens.iter().filter(|t| !t.is_punct).count();"), as the methodology reference says [methodology.md:58](../../site/content/reference/methodology.md#L58 "A gate stated in `word_count` terms is not a gate on whitespace words.")</claim>.

### The source port

<claim id="source-contract" basis="observed">The `Source` contract tells an implementation that the engine refuses bytes over `MAX_INPUT_BYTES` after `read` returns, "so a source that can learn a size first should refuse there" [source/mod.rs:33-35](../../src/source/mod.rs#L33-L35 "The engine refuses bytes over")</claim>, and <claim id="file-source-const" basis="observed">the file source does, against the constant [file.rs:62](../../src/source/file.rs#L62 "if len > MAX_INPUT_BYTES as u64")</claim>. Once the cap is the engine's, a source can only keep that contract if it is told the cap. `read` gains a `limit` argument, the value of `input_bytes`, and the contract reads: a source that can learn a document's size before reading refuses one over `limit` with `InputTooLarge`, its own `what` and `limit` set to the argument; the engine refuses past it regardless. A source that cannot learn a size first ignores the argument and loses nothing.

### The configuration file and the command line

`config/default.toml` gains a `[limits]` table holding the six defaults, and a test holds the table to `Limits::default()`, as <claim id="default-test" basis="observed">a test already holds the outcome defaults to the same file [extraction/mod.rs:89](../../src/extraction/mod.rs#L89 "a test holds the two")</claim>. `Config::limits()` returns the table over the default, key by key. `Engine::from_config` applies it; `Engine::new()`, which reads no configuration file, uses the default. A value that is not a non-negative integer fails to load, as a malformed key does today.

The command line reads stdin under `cfg.limits().input_bytes` and passes `cfg.limits()` to every outcome call, through the `_with` forms it already calls. No flag: the use cases are properties of a machine or a deployment (a server, a batch machine, a browser build), and the configuration file is where those live. No environment variable: an ambient variable would let a process's environment change a safety bound without the caller's code or file saying so, and the library reads its environment only to find its files.

Boundary rule 7's list of the public surface the command line and Python may use gains `clusters_with`, in `site/content/reference/boundary-rules.md`, the boundary check's allowlist and the project instructions. `Limits` is in `domain`, which the list already allows.

### Python

`matra.Limits` is a final extension class over `domain::Limits`, its constructor taking every cap as an optional keyword, its attributes read-only. `Engine(limits=...)` and the `limits=` keyword on `summarize`, `keyphrases` and `clusters` take one; `None` is the default. `engine.limits` is the engine's, so a caller of `Engine.from_config` can pass the configured caps on. `python/matra/limits.py`, its test and the stub's constants are removed; the guide's "Size limits" section is rewritten around `matra.Limits()`.

### TypeScript and a browser's default

EPR-0007's wrapper refuses an oversized input before copying it to the worker, then the core refuses it again. Under this proposal the wrapper checks against the engine's `limits`, which it holds because the caller passed them to `Engine.load` or took the default. The package's default is `Limits::default()`, the same as every other build's, until a measurement says a browser needs less. A different default for the WebAssembly build is then one line where the default is defined. If a browser held a document at the native rate measured here, an input at the cap would need about 2 GB or more beyond the model; whether it does is not measured. <claim id="browser-unmeasured" basis="observed">EPR-0007 measured a 181.5 MB parser with the model loaded and a 3,347-word essay, and lists as unmeasured whether memory an engine agrees to reserve can be committed on a small device [EPR-0007:389](0007-typescript-and-webassembly.md#L389 "181.5 MB after one model load") [EPR-0007:443](0007-typescript-and-webassembly.md#L443 "A phone's limit is unknown.")</claim>.

### Errors

No kind, detail or label changes. `limit` is the cap in force for the call, which is the default unless the caller set one. The message is unchanged: `{what} input too large: {actual} > limit {limit}`, except at the main gate, whose label is `input` already, so it reads `input too large: {actual} > limit {limit}`.

### What was measured

One machine: Apple M1 Max, 64 GB, macOS 26.5. matra 0.3.0 at commit `fc15104`, whose source is the pinned commit's, built with `cargo build --release --features cli`, with the pinned English model on disk. Each input is `site/inputs/federalist-10.txt` (17,880 bytes) repeated, each copy followed by one newline, which leaves a blank line between copies, read from a file, with `--json`; time and peak resident memory from the system's `time -l`. One run each.

| Input | Command | Result | Wall time | Peak resident |
|---|---|---|---|---|
| 17,881 bytes (1 copy), 88 sentences | `analyze` | a document | 2.1 s | 219 MB |
| 1,037,098 bytes (58 copies), 5,104 sentences | `analyze` | a document | 70 s | 463 MB |
| 1,037,098 bytes (58 copies) | `summarize` | `tfidf input too large: 5104 > limit 2000` | 66 s | 305 MB |
| 1,072,860 bytes (60 copies) | `keyphrases` | `rake input too large: 200820 > limit 200000` | 68 s | 361 MB |
| 8,386,189 bytes (469 copies), 41,272 sentences | `analyze` | a document | 522 s | 2,663 MB |

The peak is the command line's, which holds the document and writes its JSON; the library alone may hold less, by an amount not measured. The `summarize` and `keyphrases` runs ran beside the 8 MiB run, which affects their times and not their memory. Over the twelve texts in `site/inputs/`: 34,236 bytes, 195 sentences, 5,855 words and 6,529 tokens, by the command line's own counts.

### What was not measured

- What the library holds, as distinct from the command line, at any size.
- Memory and time on a second machine, or under the Python binding.
- Any input's memory in WebAssembly beyond EPR-0007's essay, and what a phone's browser allows.
- The outcome methods' memory at their caps or above them. The figures beside the constants (2 MiB, 32 MB, 3 MiB, 40 MiB) are estimates.
- Text other than one essay repeated: sentence length, and so the bytes at which a sentence cap is reached, varies by text.

### The boundary test and the resilience floor

The ACES boundary test, on the design as recommended:

| Question | More | Less |
|---|---|---|
| Adaptable | Six hardcoded constants become one value with a default, set in code or in the configuration file, so a deployment changes a cap with no release, and a build can carry its own default from one line. | Two caps that were one constant (TextRank's and clustering's) become two fields that can drift, by the caller's choice. |
| Composable | The source port is told its cap rather than importing a constant from the domain, so a source of your own keeps the contract under any engine. Every check stays at its choke point and the composition root still wires the only engine. | Every caller's `Source` changes signature. The `_with` forms take one more argument. |
| Extensible | One type documents every cap, its unit, its gate and its cost model; a seventh cap is a field, additive under `#[non_exhaustive]`. Five names a caller could not act on leave the surface. | A caller who raised the engine's caps must learn that the outcome functions take the value separately. |

The resilience floor, discipline by discipline:

| Discipline | Here |
|---|---|
| Size caps at the entry point | Unchanged in place: every cap is checked where it is today, before the work it bounds. What changes is who sets the number. Every cap is a number (decision 5), so the gate always exists. A raised cap removes protection the caller has to have measured for themselves, and the documentation says so beside each field's cost model. |
| Symlink rejection | Unchanged. |
| Atomic writes | Unchanged. `matra config init` writes the `[limits]` table with the rest of the defaults, through the path it already writes by. |
| Verified bytes are the bytes loaded | Unchanged, and the download cap that guards those bytes stays fixed. |
| Panic boundary at foreign code | Unchanged, and one honest limit of it: a cap raised past what the machine holds turns an `InputTooLarge` into memory exhaustion, and a failed allocation in Rust aborts the process, which no panic boundary converts. That is the cost the default exists to keep away, and why raising one is the caller's explicit act. |
| Cycle-safe walks | Unchanged. |
| Trust anchors pinned, not configurable | Not affected: a cap is not a trust anchor. The pinned digest and size stay constants. |

### Milestones

Smallest first. Each is one pull request with its documentation and CHANGELOG entry, and each leaves the tree shipping what it shipped before. None starts before acceptance.

1. **One value, no behaviour change.** `Limits` in the domain, crate-private, its `Default` reading the six public constants, so the public surface is unchanged. The engine holds one, set to the default; the engine's size check, the file source (built with the engine's cap, the port unchanged), the four method modules and both cluster checks read the value they are handed. The two invariant tests above. Every existing test passes unchanged. Built whatever decision 1 says.
2. **The value, in Rust and Python.** `Limits` public, `Engine::with_limits` and `Engine::limits`, the `_with` forms and `clusters_with` taking it, `Source::read` taking `limit`, the six constants removed and the five fixed ones crate-private; `matra.Limits`, the `limits` keywords and `engine.limits` in Python, `matra.limits` removed; rule 7's list; the errors, domain-types and design references and the Rust and Python guides. Waits on decisions 1 to 5 and 7.
3. **The configuration file and the command line.** `[limits]` in `config/default.toml` and its test, `Config::limits`, `Engine::from_config` applying it, the command line's stdin read and outcome calls, `matra config show`, the CLI guide and the agent skill. Waits on milestone 2.

TypeScript inherits the value when EPR-0007's milestones build it; nothing here builds TypeScript.

## Drawbacks

- **A caller who raises a cap owns its cost.** The default is safe on a workstation; a raised one is only as safe as the caller's measurement.
- **One value in two places.** An outcome function cannot see the engine, so a caller passes the limits twice, or is refused by the default and told which gate.
- **Every outside `Source` changes.** A port is the most expensive kind of signature to change.
- **The `_with` forms grow.** A caller who wants only another method now also passes `Limits::default()`.
- **Python loses a module that shipped in 0.3.0,** and five names in Rust and Python that some caller may read.
- **The evidence is single runs on one machine,** through the command line, not the library.

## Rationale and alternatives

### Do nothing

The limits stay constants. The batch caller stays unable to summarise a book the engine accepts, the browser build has no seam for its own default, and EPR-0007 publishes the constants in TypeScript.

### Raise the defaults instead

Make the linear methods' defaults large enough for anything the input cap admits, and leave the quadratic ones. It serves every caller with no new name. It needs the cost models measured first, which they are not, and it still serves no server that wants less or browser that needs less: one default does not fit three machines. It is complementary, not an alternative: once the value exists, a better default is a one-line, measured change.

### A memory budget

One number, `max_memory_bytes`, from which each method derives its own cap. Every use case above is about memory, so it is the unit the callers think in. It would promise a bound matra cannot check: the cost models are estimates, and the parser's memory per input byte is measured here once, through the command line, on one machine.

### The limits recorded on the document

The engine writes its limits into the document, and the outcome functions read them there, so one value set once governs everything. <claim id="document-is-data" basis="observed">A document is data: it can come from the engine, from JSON a caller saved, or from the public constructors, "and nothing tells those apart" [domain.rs:2007-2010](../../src/domain.rs#L2007-L2010 "nothing tells those")</claim>. A document from a stranger would then carry the caps it is summarised under, which moves a resource bound from the process into its input, the one place the resilience floor says it must not be. Skipping the field when serialising instead makes a document behave differently after a round trip through JSON.

### A process-wide setting

A static the caller sets once, as Python's integer limit is (below). Two engines in one process, a server's tenants or a test suite, could not hold different caps, and a library's behaviour would change from a distance.

### Keep every bound a limit

Make all eleven settable. The metric bounds would break the invariant that a limit never changes a document; the download cap guards pinned bytes no caller's work moves; the configuration cap cannot live in the file it guards.

## Prior art

- **matra itself.** EPR-0006 published the constants so a caller can size work before a call, and left raising one for the resilience floor's review; the outcome defaults already live both in the signatures and in `config/default.toml`, held together by a test. This proposal follows both patterns.
- **Python's integer conversion limit.** Checked on Python 3.14.5: converting an integer of more than 4,300 digits to a string raises `ValueError`, whose message names the call that raises the limit; the limit is process-wide; 0 means no limit; and any other value below 640 is refused. A safety bound with a default, a floor, and an explicit way to remove it, with the remedy in the error.
- **serde_json's nesting limit.** matra's own dependency, version 1.0.149: parsing stops past 128 levels of nesting, and the only way past it is to remove the limit entirely, through a method that exists only when a Cargo feature is enabled, documented with the stack overflow it risks. A bound that is either on at a fixed number or off by a deliberate, compiled-in act.

Neither is a reason to adopt anything here. Both show the question decision 5 answers being answered both ways by careful projects.

## Unresolved questions

<decision id="publish" title="1. Is the value published now, or kept inside the crate?" reversible="costly" grounds="summarize-refused keyphrases-refused no-issue server-case browser-case epr7-constants raise-deferred">

<choice key="a" title="Published now">

Milestones 1 to 3: `Limits` in Rust and Python, the engine and the outcome functions taking it, the configuration file and the command line reading it. TypeScript inherits it.

</choice>

<choice key="b" title="Inside the crate, published later">

Milestone 1 only. One value, its default and every check reading it, all crate-private; the constants stay published as today. A browser build can still carry its own default. A later API change proposal publishes the value when a caller asks for a different limit.

</choice>

<recommendation choice="a">

The one thing a caller cannot do without matra is raise a cap, and the measured gap is a raise: a document the engine accepts that the summary and keyphrase caps refuse at an eighth of the input cap or less. (b) leaves that caller with no route at all, and leaves a server and a browser each restating matra's measures in their own code. It also leaves EPR-0007 to publish the limits as constants in a fourth language, which this proposal would then replace in four. Before 1.0 that replacement costs little; what decides it is that under (b) no caller can do anything they cannot do today.

</recommendation>

<against>

No caller has asked: the repository's issues hold no request for a different limit, and the server and browser cases are reasoned, not reported. The owner's own instinct is (b), and it fits the rule that a capability waits for its consumer. (b) delivers everything matra itself needs (a per-build default, one place for every cap, checks that read a value) and keeps every new public name unwritten until a caller's case shapes it. The gap could also be closed for everyone by better defaults, once measured, with no surface at all.

</against>

</decision>

<decision id="outcome-arrival" title="2. Where do the outcome functions get their caps?" reversible="costly" depends="publish" grounds="document-is-data summarize-refused">

<choice key="a" title="An argument on the `_with` forms">

`summarize_with(&doc, n, method, limits)`, the same for `keyphrases_with`, and a new `clusters_with(&doc, embedder, threshold, limits)`. The plain forms use the default. In Python, a `limits=` keyword on each function; in TypeScript, a field of each options object.

</choice>

<choice key="b" title="Recorded on the document">

The engine writes its limits into the document it returns, and the outcome functions read them there.

</choice>

<choice key="c" title="Fixed">

Only the engine's input cap is a setting; the outcome caps stay constants.

</choice>

<recommendation choice="a">

(b) is the one a caller would least have to think about and the one that fails worst: a document is data from anywhere, so a document from a stranger would choose its own caps, and a resource bound would move from the process into its input. (c) leaves the measured gap open, since the refusals are the outcome caps', not the input cap's. (a) is explicit, and a caller who forgets it is refused loudly by the default with the gate named, not served a wrong answer. A caller who changes a cap named for a method already knows the method, so naming it in the `_with` form costs nothing new.

</recommendation>

<against>

The value is set in two places, and the common mistake (raise the engine's caps, call `summarize`) costs a full analysis before the refusal: the 1 MiB run parsed for 66 seconds before TF-IDF refused it. `clusters` gains a `_with` form that has no method to select, which stretches what `_with` has meant.

</against>

</decision>

<decision id="tunable-set" title="3. Which bounds can a caller set?" reversible="costly" grounds="compression-is-content methodology-states constants-not-settings python-limits published-ruling">

<choice key="a" title="The six caps that refuse a call; the other five fixed and no longer public">

`Limits` holds the input cap and the five outcome caps. The metric bounds, the download cap and the configuration cap become crate-private, documented in the methodology and errors references.

</choice>

<choice key="b" title="The six caps; the other five fixed and still published">

As (a), with the five kept as public constants, in Rust and in a smaller `matra.limits`.

</choice>

<choice key="c" title="All eleven">

Every bound a field of `Limits`.

</choice>

<recommendation choice="a">

A limit decides whether a call runs and never what it returns; the metric bounds decide what a document says, so (c) breaks the one invariant that lets a document omit its limits. The download cap guards pinned bytes and the configuration cap guards the file the configuration comes from, so neither has a caller who could set it usefully. Between (a) and (b): EPR-0006 published the bounds so a caller can size work before the call, and none of the five sizes a caller's work. A paragraph too short to grade already says so with a `None`, and the methodology reference states the rule. Five names a caller cannot act on are surface with no use.

</recommendation>

<against>

<claim id="python-sizing-use" basis="observed">Python's own module documents a use for a minimum: "skipping a paragraph too short to grade" [limits.py:9](../../python/matra/limits.py#L9 "skipping a paragraph too short to grade")</claim>. A caller explaining a `None` in code, or a tool that shows the rule beside the value, reads the number, and under (a) it reads it from documentation instead of the library. This also undoes part of what the owner ruled in EPR-0006, that the bounds are published.

</against>

</decision>

<decision id="cap-division" title="4. Is there one cap per method, or one per outcome?" reversible="costly" depends="tunable-set" grounds="cost-models error-shape">

<choice key="a" title="One per method">

`tfidf_sentences`, `textrank_sentences`, `rake_tokens`, `yake_tokens` and `cluster_sentences`, each named for the gate its error already names.

</choice>

<choice key="b" title="One per outcome">

`summary_sentences`, `keyphrase_tokens` and `cluster_sentences`, whatever method runs.

</choice>

<recommendation choice="a">

The methods' costs differ in kind, not degree: TF-IDF is linear in sentences and TextRank quadratic, so a summary cap raised for TF-IDF would raise TextRank's to the same number, and at ten times the sentences TextRank's matrix grows a hundredfold, from the estimated 32 MB to about 3.2 GB, where TF-IDF's working set grows tenfold. The error's `what` already names the method, so the field to raise is the one the refusal names. And only (a) lets each default later follow its own method's measured cost, which today's identical defaults do not.

</recommendation>

<against>

Four of the six names are algorithm names, against the rule that the API names use cases, not implementations. A caller of plain `summarize` never chose TF-IDF and has to learn that it is the default before raising its cap. (b) is three fields where (a) is five, and a caller who raises the summary cap to 20,000 sentences has asked for what TextRank at 20,000 sentences costs.

</against>

</decision>

<decision id="no-limit" title="5. Can a caller remove a cap?" reversible="yes" depends="publish" grounds="floor-bounded rss-one-mib rss-rate">

<choice key="a" title="No: every cap is a number">

A field is a `usize` with no sentinel, so a gate always exists. A caller may write a very large number; nothing names "unbounded". Zero is allowed, and refuses every input, loudly.

</choice>

<choice key="b" title="Yes: a field may say none">

`Option<usize>`, or a named constant, meaning no cap.

</choice>

<choice key="c" title="No, and matra refuses a value above a ceiling">

Each cap has a highest value matra accepts.

</choice>

<recommendation choice="a">

The resilience floor's rule is a cap at every entry; (b) writes the opposite into the type and documents it as an option. An `Option` also reads two ways, "use the default" or "no cap", and a reader who takes the other reading is wrong about a safety bound. (c) would need a ceiling for each cap that is right for every machine, and the one measurement here is one machine.

</recommendation>

<against>

(a) does not stop a caller removing a cap; `usize::MAX` does it, so (a) only makes the act less obvious to the next reader than a named "none" would. (c) is the only choice that actually enforces the floor, and Python's integer limit shows a careful project choosing a floor and an explicit "no limit" together.

</against>

</decision>

<decision id="provenance" title="6. Does a document record the limits it was made under?" reversible="yes" depends="tunable-set" grounds="compression-is-content document-is-data">

<choice key="a" title="No">

A limit decides only whether a call runs, which the invariant test holds, so two successful results under different limits are equal and record nothing that differs.

</choice>

<choice key="b" title="Yes">

`provenance` records the caps in force, as it records `n` and the method.

</choice>

<recommendation choice="a">

Provenance records what made a result what it is, so that a reader can tell two results apart. Limits make no result different: a run under other limits either gives the same bytes or no result. Recording them would make two equal documents compare unequal, against the determinism the conformance suite relies on, and would invite a reader to treat a recorded cap as a fact about the content.

</recommendation>

<against>

A reader trying to reproduce a run of a large document cannot tell from the document that it needs raised limits; they learn it from a refusal, and only after the parse it took to get there.

</against>

</decision>

<decision id="source-port" title="7. How does a source learn the engine's cap?" reversible="costly" depends="publish" grounds="source-contract file-source-const">

<choice key="a" title="`Source::read` takes it">

`read(&self, path, limit)`. The contract says a source that can learn a size first refuses above `limit`.

</choice>

<choice key="b" title="The engine builds its file source with the cap; the port is unchanged">

`with_limits` replaces the cap on the shipped file source when it is the engine's source. A caller's source is never told.

</choice>

<recommendation choice="a">

The contract already asks a source to refuse early; once the cap is the engine's, a source can only do that if the engine tells it, and the argument is the smallest way to tell it. (b) works only for the source matra ships, makes `with_limits` and `with_source` depend on the order they are called, and leaves the port's documentation naming a constant that no longer decides anything.

</recommendation>

<against>

<claim id="provider-freezes" basis="observed">The engine's own design notes that a provider interface "freezes once implemented" [lib.rs:296](../../src/lib.rs#L296 "freezes once implemented")</claim>, and every outside `Source` must change for an argument most of them ignore. (b) costs nothing outside the crate, and a caller's source that reads too much is still refused by the engine after it returns.

</against>

</decision>

What the owner should confirm or strike, collected from the claims above that stand on no grounds yet:

<assumptions />

## Future possibilities

- **Measured defaults.** With the value in place, each default can follow a measurement of its method's cost, and the WebAssembly build its own, each one line and its own pull request.
- **A flag for one run,** if a command-line caller asks for a cap that differs from their file's for a single command.
- **The field in the error's message.** Python's limit names its remedy in the error; `InputTooLarge`'s message could name the field to raise. It is text for a person, so it changes no contract.
- **A cap on time.** Every cap here is on size, and the 1 MiB run took 70 seconds. A server's real budget may be time; matra has no cancellation today.
