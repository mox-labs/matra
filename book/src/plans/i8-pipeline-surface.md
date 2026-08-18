# I8: One pipeline, not six entry points

**Boundary:** pre-publish surface freeze. Must land before 0.1.0 or never.

**Origin:** a maintainer question ("why do we have so many entry points?") followed by a formal review. Two live defects were found by taking the stage types seriously; one is fixed, one is documented with its root cause still open.

---

## Why this iteration exists

Six public entry points are partial applications of one chain:

```
analyze(text)             = decompose(Plain)    then run_analysis
analyze_markdown(text)    = decompose(Markdown) then run_analysis
analyze_file(path)        = read then dispatch-on-format then one of the above
analyze_directory(path)   = read_many then map(dispatch) then collect errors
parse(text)               = guard then nlp.parse
analyze_from(secs, sents) = metrics only
```

Three vary along two axes, source kind and format, enumerated as named functions. Adding PDF and DOCX grows the enumeration again. The project already has abstractions for exactly those axes, the `Source` and `Decomposer` ports, and no entry point accepts them.

That is the aesthetic complaint. The correctness argument is stronger, and it is what justifies the work.

### The two defects

**Defect A, fixed in 368bac5.** `MAX_INPUT_BYTES` is the bound past which UDPipe's per-token allocations cross roughly a gigabyte resident. `check_input_size` guarded three Rust entry points; a separate mechanism in `FileSource` covered two more, reporting a different `what` discriminator for the same logical failure; and four PyO3 extraction methods called `nlp.parse` with no gate at all. `Matra.analyze(huge)` raised while `Matra.tfidf_summarize(huge, 3)` did not.

**Defect B, documented, root cause open.** `analyze_from` builds `Document::new(sections)` and runs the suite without ever filling `Paragraph::sentences`. All three paragraph metrics gate on `Paragraph::word_count`, which sums over that field, so `readability_grade`, `lexical_density` and `compression_ratio` are unconditionally `None` while the document-level pair is `Some`. The docs presented it as equivalent to `analyze_markdown`.

It is not fixable at that layer. The function receives one flat sentence slice with no record of which sentence came from which paragraph, and recovering the mapping means matching text, which this crate removed after two paragraphs sharing an opening substring were assigned each other's sentences.

The root cause is representational: `run_suite` carries the sentence set twice, flattened in a slice and attached to paragraphs, with nothing enforcing agreement. Milestone 1 removes the redundancy.

**Both defects have one shape: N entry points means N restatements of each invariant, and the compiler checks none of them.** That is the argument. Elegance is a side effect.

---

## The vocabulary question

The maintainer proposed `ingest -> decompose -> abstract -> compose`, everything being `data -> transform -> data`.

`abstract` **cannot name code: it is a reserved keyword in Rust.**

Two readings were considered. As a name for today's NLP parse it fails, because `Decomposer::decompose` and `NlpProvider::parse` have the same representational shape, string in and latent structure out, and differ by dependency and failure mode, which is what ports are factored by rather than what stages are. As the currently empty tier between structure and purpose-fitted output it holds: that is where rule evaluation lands, and its representation would be `Document -> Vec<Finding>`.

Recommended vocabulary: **`ingest -> decompose -> compose`**, three stages, with `abstract` reserved in an ADR as the named empty seam and documented as unoccupied at 0.1.0. Do not name a stage that has no code.

This **supersedes ADR-0002**, whose five verbs enumerate calling conventions rather than transformations. `measure` and `extract` are one operation with two output projections: `Metric` mutates and returns unit, extractors return values, and that difference is why extract was called a peer. It is not a peer.

---

## Milestones

Each leaves the tree green. Milestones 1 to 3 are worth doing whether or not the surface changes.

| M | What | Commits you to the redesign? |
|---|---|---|
| 1 | `Metric: Fn(&mut Document)` | no |
| 2 | Domain additions | no |
| 3 | `Decomposers` registry | no |
| 4 | `Ingest` and `Engine`, alongside the six | no, both surfaces coexist |
| 5 | Migrate CLI, examples, PyO3 | yes |
| 6 | Delete the six | yes, and this is the gate |
| 7 | Documentation | follows |
| 8 | ADRs | follows |

### M1: remove the redundant sentence set

`Metric` becomes `Box<dyn Fn(&mut Document)>` and `run_suite` drops its slice parameter. The flat set is derived from `Document::sentences()`, which already exists. Passing it separately is what lets a caller supply a set that disagrees with the paragraphs, which is Defect B.

**Rubric.** Paragraph and document metrics read one sentence set, derivable in one way. No metric signature accepts a sentence slice. The Defect B regression test still passes or is updated to assert the new postcondition. `document::compute` already collects into owned locals, so the borrow reshuffle is local.

Breaks any external `Metric` implementation. Free now.

### M2: domain additions

`DocumentError { path: Option<PathBuf>, error: Error }`, `CorpusResult { corpus, errors }`, `impl FromIterator<Result<CorpusEntry, DocumentError>> for CorpusResult`, and `Format: PartialEq + Eq`.

`Option<PathBuf>` closes a real hole: `lib.rs` currently fabricates an empty `PathBuf` via `unwrap_or_default()` for a path-less document, which collides with a genuinely empty path.

**Rubric.** Purely additive, no existing signature changes. Partition holds: entries plus errors equals items consumed. No new dependency.

**Known blocker.** `Error` is neither `Serialize` nor `Clone`, since it wraps `io::Error`. So `DocumentError` and `CorpusResult` inherit both gaps while `Corpus` has neither. Crossing to Python needs a projection with stable kind strings, not `Debug` of `io::ErrorKind`. This blocks M5, not M2.

### M3: format dispatch as a table

`Decomposers` in `decompose/mod.rs`, importing only `domain::{Format, Section}`. Population lives in `lib.rs`, the only place naming both adapters, so rule 7 holds.

Makes `Error::UnsupportedFormat` true: its doc comment already claims "no registered decomposer in this build", describing a registry that does not exist.

**Rubric.** `Decomposer::decompose` stays total; partiality lives in lookup returning `Option`. `with` replaces on duplicate key, tested. `standard_decomposers()` is an exhaustive `match` over `Format`, so a new variant stays a compile error rather than a silent `UnsupportedFormat`. Object safety preserved.

### M4: `Ingest` and `Engine`, added alongside

```
pub type Ingested = Result<RawDocument, DocumentError>;
pub struct Ingest;                    // concrete, NOT impl Iterator
impl Iterator for Ingest { type Item = Ingested; }
Ingest::text(text, format)            // n = 1, never fails
Ingest::path(p) -> Result<Ingest>     // n = 1 or N, file or directory

pub struct Engine { nlp, decomposers }
Engine::analyze<I: IntoIterator<Item = Ingested>>(&self, I)
    -> impl Iterator<Item = Result<CorpusEntry, DocumentError>>
Engine::analyze_one(&self, RawDocument) -> Result<CorpusEntry, DocumentError>
Engine::annotate(&self, &RawDocument) -> Result<Document>   // sole nlp.parse caller
Engine::compose(&self, &mut Document)                        // total
```

`Ingest` must be a concrete named type. With `impl Iterator` the constructors return distinct opaque types and "a string and a directory are the same call" is false at the type level. Concrete and `'static` also removes I5 Risk 2 outright, since it owns its path list and borrows nothing.

**Rubric.** These laws are the acceptance test, not prose:

```
L1  analyze(a.chain(b))        = analyze(a).chain(analyze(b))
L2  analyze(empty())           = empty()
L3  analyze(once(Ok(raw)))     = once(analyze_one(raw))
L4  analyze_one(r).analysis    = { let mut d = annotate(&r)?; compose(&mut d); d }
L5  |entries| + |errors|       = |input|
L6  Err input item             => identical Err output, analyze_one not called
L7  no text over MAX_INPUT_BYTES reaches NlpProvider::parse
```

L1 to L3 are the formal content of "a single document is a collection of one": `once` is the singleton injection and the pipeline commutes with it, so n=0, n=1 and n=N are one function at three lengths. L7 becomes provable rather than empirical once `annotate` is the unique path from text to the parser.

Per-paragraph parse is untouched. The stream element is the document, which is also the granularity floor, because `vocabulary_ttr` and `nominalization_ratio` aggregate over the whole document.

### M5: migrate internals

CLI (3 call sites), examples (3 files), PyO3 surface. Defect A closes structurally rather than by patch, since `annotate` becomes the only route to the parser.

**Rubric.** No caller inside the repository uses a deleted entry point. The Python size-cap test still passes. The `Error` serialization blocker from M2 is resolved or the Python surface is explicitly scoped out.

### M6: delete the six

`analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`, `analyze_from` go. `parse` is replaced by `annotate`, which does not diverge: today `parse(t)` and `analyze(t)` yield different sentence sets for the same input, because whole-text parse can merge across blank lines and includes blockquotes where `analyze` skips them.

**Rubric.** No function name mentions a format or a source kind. The variation lives in data constructors.

**This is the gate. Free now, a breaking change after publish.**

### M7 and M8: documentation and ADRs

13 files reference the old surface. Supersede ADR-0002; write one ADR for the pipeline shape and the reserved `abstract` seam.

---

## Costs, named

1. **Not fewer names.** Roughly nine against six. What is bought is one implementation and closure under format growth, not a smaller namespace.
2. **Callers hold an `Engine`.** Someone must own the decomposer table; free functions cannot.
3. **Work moves to consumption time.** `Ingest::path(dir)?` reads nothing until pulled, so "it returned Ok, therefore every file was read" stops holding.
4. **The result stream is not `Send`.** It captures `&Engine`, and `NlpProvider` is `Send` without `Sync`. The streaming shape invites parallelism the port cannot support.
5. **`Document` stays two-phase.** A Structure/Annotated/Measured split was considered and declined: the `Option` slots also encode inapplicability, so the split would not remove them, and it would only prove "compose has run", which L4 answers more cheaply.
6. **Compile-time exhaustiveness over `Format` is lost** unless `standard_decomposers()` is written as an exhaustive match. Do not skip that.

## Risks

**The whole design assumes no reactor.** Two lazy `analyze` streams interleaved on one thread are safe only because `analyze_one` runs to completion inside a single `next()`. That breaks the moment a stage can yield inside a parse, which is exactly what a reactor introduces. ADR-0004's deferral is load-bearing for this proof and the new ADR should say so.

**Grep care on M6.** `parse` has far more hits in `src/` than the other five combined, because most are `NlpProvider::parse` rather than the free function.

**Relationship to I5.** Subsumes Tasks A, B, C and D. Contradicts I5 only on keeping `analyze_directory` deprecated: deprecate-and-keep was justified by protecting a consumer who adopts it between 0.1.0 and 0.1.x, and pre-publish there is no such consumer.

**Relationship to I7.** I7 M1 asks whether structural primitives are fields or methods. That question is entangled with this surface and should be decided after M4, not before.
