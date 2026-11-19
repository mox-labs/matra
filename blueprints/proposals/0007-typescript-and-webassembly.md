# EPR-0007: TypeScript and WebAssembly

- Feature Name: `typescript_wasm`
- Start Date: 2026-11-18
- Pinned at: `4cac9411d2c856ece4dea2f4f7c0a39cd2d9aa97` (every code citation below opens at this commit)
- Proposal PR: none yet (filled in when the pull request that carries this text is open)
- Tracking issue: none (opened when the proposal is accepted)
- Status: proposed

## Summary

matra gains a TypeScript package for the browser and Node.js. It says what Rust and Python say: load an `Engine`, `analyze` an `Input`, then `summarize` or take `keyphrases` of the `Document` that comes back, typed from the published JSON Schema, with every failure carrying the library's `kind` and `details`. matra's core and its parser both run as WebAssembly, in a worker, so parsing never blocks a page. A spike found that this works: UDPipe's C++ compiled to WebAssembly parses matra's twelve example texts byte for byte as the native build does, in Node and in three browser engines, at about half the native speed, and the core compiled unchanged around it.

The spike also found what a package must settle that a spike could skip. The parser's identity was whatever the calling script wrote. Three pieces of the UDPipe adapter had to be copied. A corrupted model can crash the loader, natively too, and one corrupted model loaded without complaint. A panic in the core reaches the caller as an unexplained trap. And the parsing model is licensed for non-commercial use and served by a host that refuses requests from other sites, so the package cannot carry it and a page cannot fetch it from where it lives. This proposal answers each, asks the owner six questions, and lays out a seventh without a recommendation: who serves the model.

Nothing here is built. The roadmap entry that said UDPipe could not run in a WebAssembly sandbox is corrected in the same pull request.

<pragmatics>

<ask>

Rule on the six decisions under [Unresolved questions](#unresolved-questions). Choose who serves the parsing model to a browser, from the three options under [Who serves the model](#who-serves-the-model); Claude lays them out and recommends none, because the choice turns on a licence and on what the project is willing to host. Then accept the proposal by merging a pull request that changes its status to `accepted`.

</ask>

<will>

- Record each ruling in its decision, as a named step in a pull request.
- Open the tracking issue at acceptance and keep it current with the [milestones](#milestones).
- Build the milestones in order, one pull request each, and repeat in continuous integration every measurement the spike took by hand, so each number below gets a home a reader can open.
- Keep the roadmap saying only what this record says.

</will>

<needs>

- Your rulings, and your choice of who serves the model. Only the playground waits for that choice; the package does not.
- An npm account or organisation that owns the package name, and an `npm` deployment environment on the repository. Only you can create and approve them.
- A reading with one caveat in mind. Every measurement below is one spike on one machine, run and reported by Claude agents. Its working files are not in the repository, so until a milestone repeats a number in continuous integration, that number is a reading and not a test.

</needs>

<wont>

- Build any of this while the proposal is `proposed`, change its status, or merge a pull request that does.
- Publish to npm, or approve a deployment environment.
- Put the parsing model in the package, or have the package fetch it from an address of its own choosing.

</wont>

<silence>

The proposal stays `proposed` and nothing is built. The roadmap says the parser spike returned a go and that the design awaits a decision. Claude raises it again at the start of the next session that touches the 0.4.0 plan.

</silence>

</pragmatics>

## Motivation

The owner wants TypeScript support, and then a playground on the docs site where a visitor pastes text and sees its structure with nothing installed.

### The roadmap said the parser could not run there

<claim id="roadmap-blocked" basis="observed">The roadmap tells readers that "C FFI cannot run in a WASM sandbox", and that a WebAssembly parser would need a different parser or a server [ROADMAP.md:63](../../ROADMAP.md#L63 "C FFI cannot run in a WASM sandbox")</claim>. EPR-0006 planned a spike to find out, ending in a go or a no-go. It ran between 2026-11-14 and 2026-11-18, in three stages, and the answer is a go:

- <claim id="parse-identical" basis="observed">UDPipe 1's C++, compiled with Emscripten, gives output byte-identical to the native build over the twelve texts in `site/inputs/`: 57 paragraphs, 6,529 words [the spike](#what-the-spike-measured)</claim>.
- <claim id="core-unchanged" basis="observed">matra's core compiled for `wasm32-unknown-unknown` with no change to its source, with that parser handed in through the constructor every outside parser uses [lib.rs:365](../../src/lib.rs#L365 "pub fn with_parser(") [the spike](#what-the-spike-measured)</claim>. Three documents came out token for token as the native command line writes them.
- <claim id="browsers-identical" basis="observed">In the Chromium, Firefox and WebKit builds Playwright ships, on the main thread and in a worker, all 18 runs matched the native output [the spike](#what-the-spike-measured)</claim>.

### What the spike had to work around

A spike may copy code and write its own labels. A package may not. Each item below is something the spike did that a published package cannot.

1. **It wrote the parser's identity by hand.** <claim id="identity-stated" basis="observed">The parser's name, model, digest and licence in a document's provenance were whatever the calling script passed: with nothing passed they were `null`, and a script could have passed anything [the spike](#what-the-spike-measured)</claim>. Natively the identity is derived from the model's bytes [udpipe.rs:678](../../src/nlp/udpipe.rs#L678 "fn identity_of(").
2. **It copied three pieces of the adapter.** <claim id="mapping-private" basis="observed">The rule that places a token's span, the function that calls it, and the mapping from one of UDPipe's words to a token are private to the crate [domain.rs:805](../../src/domain.rs#L805 "pub(crate) fn form_end(") [udpipe.rs:768](../../src/nlp/udpipe.rs#L768 "fn locate(") [udpipe.rs:778](../../src/nlp/udpipe.rs#L778 "fn token_of(")</claim>, <claim id="spike-copied" basis="observed">so the spike, written as an outside crate, carried its own copy of each [the spike](#what-the-spike-measured)</claim>. Two copies of a rule drift.
3. **It patched a default.** <claim id="builder-deps-empty" basis="observed">A token built through the public builder has an empty `deps` column [domain.rs:759](../../src/domain.rs#L759 "deps: String::new(),")</claim>, where <claim id="adapter-deps-underscore" basis="observed">the adapter writes `_` [udpipe.rs:803](../../src/nlp/udpipe.rs#L803 "deps: String::from(")</claim>. The spike set `_` by hand to match.
4. **It found three calls that fail only when run.** <claim id="file-dir-runtime" basis="observed">`Input::file`, `Inputs::dir` and `Config::resolve` compile for WebAssembly and fail when called, the first two with "operation not supported on this platform" and the third because no home directory exists [lib.rs:125](../../src/lib.rs#L125 "pub fn file(") [lib.rs:211](../../src/lib.rs#L211 "pub fn dir(") [config.rs:152](../../src/config.rs#L152 "pub fn resolve()") [the spike](#what-the-spike-measured)</claim>.
5. **It found a model that crashes the loader, and one that does not.** <claim id="first-byte" basis="observed">A model whose first byte is changed to `0xff` throws a C++ exception that nothing in UDPipe catches [the spike](#what-the-spike-measured)</claim>. <claim id="native-terminates" basis="observed">The native build does the same, and the process ends with exit code 134 [the spike](#what-the-spike-measured)</claim>. <claim id="flip-accepted" basis="observed">Of 41 corrupted models, 39 failed cleanly, one crashed as above, and one, with a single byte changed at offset 40, loaded and parsed [the spike](#what-the-spike-measured)</claim>.
6. **It saw a panic arrive with no explanation.** <claim id="panic-unreachable" basis="observed">A panic in the core reaches JavaScript as `RuntimeError: unreachable`, with no message, and the instance it left behind later failed in a different way [the spike](#what-the-spike-measured)</claim>.
7. **It served the model from its own machine.** <claim id="model-nc" basis="observed">The model is CC BY-NC-SA 4.0, a separate work from matra with its own terms [udpipe.rs:44](../../src/nlp/udpipe.rs#L44 "CC BY-NC-SA 4.0 (non-commercial)") [udpipe.rs:41](../../src/nlp/udpipe.rs#L41 "this file is a separate work with its own terms")</claim>, and <claim id="host-refuses" basis="observed">its host answered 403 to a request carrying an `Origin` header and 200 to the same request without one, measured with `curl` on 2026-11-18 [udpipe.rs:36](../../src/nlp/udpipe.rs#L36 "lindat.mff.cuni.cz")</claim>. A browser always sends that header to another site, so a page cannot fetch the model from where it is published.

### Why now

0.3.0 settled the surface so that a fourth language could copy it once. The copy is cheapest while the three existing syntaxes are fresh and nothing in TypeScript has been published, because a name on npm is as permanent as one on crates.io.

## Guide-level explanation

### The same API in a fourth syntax

```typescript
import { Engine, Input, summarize, keyphrases } from "matra";

const engine = await Engine.load({ model: "/models/english-ewt-ud-2.5-191206.udpipe" });

const doc = await engine.analyze(Input.text("The river rose. The bridge held."));
doc.metrics.total_words;          // a field, typed from the published schema
doc.provenance.parser.sha256;     // the model that produced this parse

const summary = await summarize(doc);             // three sentences, as in Python
const phrases = await keyphrases(doc, { n: 5 });

engine.close();
```

| Outcome | Rust | Python | TypeScript |
|---|---|---|---|
| Build the engine | `Engine::new()` | `Engine()` | `await Engine.load({ model })` |
| Analyze one | `engine.analyze(Input::text(t, format))` | `engine.analyze(t)` | `await engine.analyze(Input.text(t))` |
| Analyze many | `engine.analyze_all(inputs)` | `engine.analyze_all(texts=...)` | `engine.analyzeAll(inputs)` |
| Summarize | `summarize(&doc, n)` | `summarize(doc)` | `await summarize(doc)` |
| Keyphrases | `keyphrases(&doc, n)` | `keyphrases(doc)` | `await keyphrases(doc)` |

The names are the same words in TypeScript's case. What differs follows from where the code runs:

- **Every call returns a promise.** The parser runs in a worker so that a page stays responsive, and a message to a worker is answered later.
- **The caller says where the model is.** `model` is a URL the page may fetch, or the bytes themselves. The package holds no model and no default address ([Who serves the model](#who-serves-the-model)).
- **A `Document` is data.** It is the JSON the command line prints, as an object, with no methods. So `summarize` and `keyphrases` take a document from anywhere: this engine, a file `matra analyze --json` wrote, a server running Python. A page that only summarizes documents parsed elsewhere never downloads the model.
- **There is no `Input.file`, no directory input and no configuration.** A browser has no file system and no home directory. They are absent from the types, so a call to one fails when the code is compiled, not when it runs. A Node caller reads the file and passes `Input.bytes`.
- **There is no `clusters` yet.** Embeddings have not been measured in WebAssembly.

### When something fails

A failure the library reports rejects with a `MatraError`, which carries the same three facts as the JSON envelope and the Python exception: `kind`, `message` and `details`.

```typescript
try {
  await engine.analyze(Input.bytes(upload));
} catch (e) {
  if (e instanceof MatraError && e.kind === "input_too_large") {
    show(`Too large: ${e.details.actual} bytes, limit ${e.details.limit}`);
  }
}
```

A model that is not the one matra pins is refused before it is loaded, with `model_invalid`. There is no way to load another.

### What it costs a page

| What | Gzipped | Notes |
|---|---|---|
| The parser, compiled | 228 KB | fetched only when an engine is loaded |
| matra's core | 659 KB to 760 KB | the lean and the full spike builds; the size of the package's own build is unmeasured |
| The model | 16.3 MB | does not compress; about 95% of the download |

Loading the model took about one second and left the worker holding about 181 MB, which it keeps until `engine.close()` ends the worker. A 3,347-word essay took about two seconds to analyze, and a page-length text 61 to 69 ms. [What the spike measured](#what-the-spike-measured) has every number and how it was taken.

## Reference-level explanation

### The pieces

<sketch-figure id="pieces" seed="7" title="The pieces at run time: the page holds a thin wrapper, one worker holds both modules">

```text
width 400
state proposed proposed "the caller talks to the package; the package talks to one worker"
box caller "Caller's code\npage or Node" 2 8 110 34
box wrapper "matra package\nEngine, Input,\noutcomes, errors" 2 62 110 48
quiet box types "Types from\nschema 2.x" 2 130 110 34
hex core "matra core\nRust, wasm32\nreads, checks,\nmeasures" 140 30 120 100
box parser "UDPipe 1, C++\nEmscripten build" 286 30 112 36
box model "Model bytes\ndigest checked\nbefore load" 286 88 112 46
edge caller wrapper
edge wrapper types
edge wrapper core
edge core parser
edge model parser
```

</sketch-figure>

Two WebAssembly modules, as the spike built them. The parser is UDPipe 1 compiled by Emscripten. The core is matra itself, compiled for `wasm32-unknown-unknown` without the `udpipe` feature. The core calls the parser once per paragraph through a function the wrapper hands it, and checks what comes back exactly as it checks any parser: <claim id="conform-holds" basis="observed">the calls to `conform::read` and `conform::parse` sit in the engine, not in an adapter [lib.rs:562](../../src/lib.rs#L562 "conform::read(") [lib.rs:576](../../src/lib.rs#L576 "conform::parse(")</claim>, and the spike saw them fire on a word not in the text and on a head out of range. The size cap holds for the same reason.

### The Rust side

The binding is a module of the crate behind a non-default `wasm` feature, as <claim id="python-in-crate" basis="observed">the Python binding is a module of the crate behind `python` [lib.rs:865-866](../../src/lib.rs#L865-L866 "mod python;")</claim> ([decision 1](#decision-binding-home)). Working names, fixed when built:

- **`src/wasm/`** exports what the wrapper calls and nothing else. It is held to boundary rule 7 as `src/python/` is: the public surface, `config` and `domain`, never a port, an adapter or a crate-private module. The rule gets a semgrep check and a fixture with it.
- **A second adapter of the parser port**, beside the UDPipe one, behind the same feature. It holds the function the wrapper supplied and the identity the pin gave it, and implements `NlpProvider`. The composition root wires it, in one constructor compiled only under `wasm`.
- **One copy of the mapping.** The UDPipe adapter does three things after the C++ call returns: it groups words into sentences, builds each token, and walks one cursor through the paragraph to place each span. Those three move, unchanged, into one crate-private function over plain column values. The native adapter turns each `udpipe_rs` word into those values and calls it; the new adapter turns what the worker returned into the same values and calls it. `nlp/udpipe.rs` stays the only file that imports `udpipe_rs`.
- **The pin out from behind `udpipe`.** <claim id="pin-gated" basis="observed">The model's name, size, digest and licence are constants in the UDPipe adapter, which the `udpipe` feature compiles [udpipe.rs:20](../../src/nlp/udpipe.rs#L20 "const ENGLISH_MODEL_SHA256") [udpipe.rs:25](../../src/nlp/udpipe.rs#L25 "const ENGLISH_MODEL_NAME") [udpipe.rs:28](../../src/nlp/udpipe.rs#L28 "const ENGLISH_MODEL_SIZE")</claim>. They move to where both builds compile them, so the WebAssembly build checks the same digest from the same line of source.

<claim id="send-atomics" basis="observed">The parser trait requires `Send` [nlp/mod.rs:94](../../src/nlp/mod.rs#L94 "pub trait NlpProvider: Send"), and the spike's adapter met it with no `unsafe` of its own only because `wasm-bindgen` marks a JavaScript value `Send` on builds without threads [the spike](#what-the-spike-measured)</claim>. A threads build was not tried and is not proposed.

### Parser identity and the digest

The caller states where the bytes are. Nobody states what they are. The worker:

1. Refuses bytes whose length is not the pinned size, and stops reading a response once it runs past that size.
2. Computes the SHA-256 of its own copy of the bytes and compares it with the pin, in the core, beside the constant.
3. Only then hands that same copy to the C++ loader. Nothing is fetched or read a second time.
4. Builds the identity from the pin: the name, the digest and the licence the native adapter reports for the same bytes.

A mismatch rejects with `model_invalid` and nothing reaches the loader. <claim id="digest-shields" basis="inferred">That closes both findings of the corrupted-model run at once: the model that crashes the loader and the model that loads silently are each refused by the digest, and no other check tells the second from a good model</claim>. It is also what the native side does: <claim id="native-verifies" basis="observed">the function that verifies the model returns the verified bytes, and those bytes are what load [udpipe.rs:658](../../src/nlp/udpipe.rs#L658 "fn read_and_verify(") [udpipe.rs:136](../../src/nlp/udpipe.rs#L136 "Model::load_from_memory(data)")</claim>.

Accepting only a pinned model is not a new rule. <claim id="pin-ruling" basis="observed">0.3.0 removed loading a model file of the caller's choosing from Rust and Python, because "the model is a pin" [EPR-0006:408](0006-the-0-3-0-surface.md#L408 "Loading a parser model file of the caller's choosing")</claim>. When the permissively licensed model arrives as a second pin, the digest of the bytes says which pin they are, so TypeScript needs no id parameter.

The digest is computed by the core's own SHA-256, the function that already computes `provenance.source.sha256`, fed in pieces so the core's memory does not grow by the size of the model. The spike used the browser's built-in digest instead and measured 10 to 26 ms; that function exists only on secure origins, which a page served over plain HTTP on a local network is not. The core's own speed over 16 MB is unmeasured.

### The TypeScript surface

```typescript
class Engine {
  static load(options: { model: string | URL | ArrayBuffer | Uint8Array }): Promise<Engine>;
  analyze(input: Input): Promise<Document>;
  analyzeAll(inputs: Iterable<Input> | AsyncIterable<Input>): AsyncIterable<Document | DocumentError>;
  close(): void;
}

class Input {
  static text(text: string, options?: { format?: Format }): Input;
  static bytes(data: ArrayBuffer | Uint8Array, options?: { format?: Format }): Input;
  static blocks(blocks: Block[]): Input;
}

function summarize(doc: Document, options?: { n?: number; method?: SummaryMethod }): Promise<Summary>;
function keyphrases(doc: Document, options?: { n?: number; method?: KeyphraseMethod }): Promise<Keyphrases>;

class MatraError extends Error {
  readonly kind: ErrorKind;
  readonly details: Record<string, string | number | boolean>;
}

const limits: { readonly maxInputBytes: number /* and the rest, as Python's matra.limits */ };
```

- **Types.** `Document`, `Summary`, `Keyphrases`, `Block` and the error body are generated from `spec/schema/v2/`. The generated file is committed and a check regenerates and compares it, as the docsite's `llms.txt` is held. Field names stay as the schema spells them (`total_words`, `byte_start`), because a document is the same JSON in every language.
- **Defaults follow Python.** Text is plain text unless a format is named; bytes are detected unless one is named. <claim id="python-defaults" basis="observed">`summarize` returns three sentences and `keyphrases` ten phrases by default [_core.pyi:437](../../python/matra/_core.pyi#L437 "n: int = 3") [_core.pyi:452](../../python/matra/_core.pyi#L452 "n: int = 10")</claim>. `method` is optional and takes the names Python takes; no function name carries one.
- **A document is data.** <claim id="document-is-data" basis="observed">The domain already says so: a document "can come from `Engine::analyze`, from JSON a caller saved and read back", and every outcome holds on one whose parts contradict each other [domain.rs:2007](../../src/domain.rs#L2007 "A document is data.")</claim>. Methods do not cross into another language, fields do. So TypeScript's `Document` is the parsed JSON, and an outcome function sends it to the core, which reads it back as the Rust type. That round trip is unmeasured ([what was not measured](#what-was-not-measured)).
- **Outcome functions need no engine.** They start the core alone, with no parser and no model. Because they return promises, whether they run on the calling thread or in a worker can change without changing a signature; the milestone that builds them measures both.
- **`analyzeAll`** yields one item per input, in input order, as each finishes. A failed document is an item, `{ path, error }`, not an exception, as in Rust and Python ([decision 3](#decision-analyze-all)).
- **`close`** ends the worker. It is the only way to give the model's memory back: the parser's memory did not shrink when the model was freed.
- **One call at a time.** Calls on one engine queue and run in order. An engine is not shared between workers, as an `Engine` in Rust is not shared between threads.
- **One package.** The browser build starts a module worker; the Node build a worker thread; the caller's code is the same. The entry is chosen by the package's export conditions. Only the browser half of that was measured.
- **`limits`** is generated from the Rust constants when the package is built, so the wrapper can refuse an oversized input before copying it to the worker. <claim id="cap-core" basis="observed">The core refuses it regardless: the spike saw an input one byte over the 8 MiB cap fail with the library's own error [domain.rs:50](../../src/domain.rs#L50 "MAX_INPUT_BYTES") [the spike](#what-the-spike-measured)</claim>.

### Failures

Three classes, told apart by who is at fault.

| What happened | What the caller sees | The engine afterwards |
|---|---|---|
| The library reports a failure (oversized input, unknown format, a parse that breaks the contract, a model that is not the pin) | `MatraError` with the kind and details every other channel carries | usable |
| The parser throws a C++ exception or traps | `MatraError`: `model_invalid` while loading, `parse_failed` while parsing | usable after an exception; closed after a trap |
| matra's own code panics | an `Error` that is not a `MatraError`, carrying the panic's message and location ([decision 5](#decision-core-panic)) | closed |

- **Kinds cross as data.** <claim id="error-serde" basis="observed">`Error` already serializes as its kind, message and details [domain.rs:455](../../src/domain.rs#L455 "impl Serialize for Error")</claim>. The spike passed only the message as a string; the binding passes the serialized error, and the wrapper builds the `MatraError` from it.
- **The parser's failures are the parser's.** <claim id="native-catches-panic" basis="observed">The native adapter catches a panic at the C++ boundary and reports `ParseFailed` [udpipe.rs:695](../../src/nlp/udpipe.rs#L695 "fn catch_parse_panic")</claim>. The wrapper does the same at the one place it calls the parser module. <claim id="cpp-contained" basis="observed">In the spike a C++ exception thrown inside the parser, from inside a call the core was making, arrived as `parse failed`, and the same instance then loaded a good model and parsed [the spike](#what-the-spike-measured)</claim>. A trap is different: memory after an abort is not to be trusted, so the engine closes and the caller loads a new one.
- **A panic cannot be caught in the core.** On this target a panic aborts. A hook records its message and location before it does; the wrapper catches the trap, attaches what the hook recorded, closes the engine, and rejects. A call on a closed engine rejects with an `Error` saying it is closed.

### Building the parser

<claim id="source-pin" basis="observed">The C++ is the copy the `udpipe-rs` crate vendors, at the version and checksum `Cargo.lock` already pins for the native build [Cargo.lock:1781-1784](../../Cargo.lock#L1781-L1784 "udpipe-rs")</claim>, found through `cargo metadata`. One pin, two builds, which is why the output is the same. The spike used Emscripten 7.0.0 with `-O2`; the build pins the compiler by version and digest, as the repository pins its other tools.

Which exception handling the build uses is [decision 4](#decision-exception-handling). The build the spike measured in browsers uses the older instruction, which Firefox reports as deprecated.

The package ships UDPipe compiled, as the crate and the wheels do. What UDPipe's licence asks of a binary on npm (its text, a pointer to the source) is checked in the milestone that builds it, not here.

### The package: where it lives, how it is built and published

- **Where.** `typescript/` at the repository root, beside `python/`: the wrapper's source, the generated types, the tests and `package.json`. Nothing built is committed.
- **Name and version.** One name and one version on every registry ([decision 6](#decision-package-name)). `scripts/check-version-sync.sh` reads `typescript/package.json` with the other version-carrying files.
- **Build.** One `just` recipe: the core through `wasm-bindgen` (the spike used wasm-pack 0.15.0 and wasm-bindgen 0.2.130), the parser through Emscripten, the types from the schema, then the TypeScript compiler. The tarball holds the two `.wasm` files, the JavaScript, the type declarations and the licence texts. It never holds the model.
- **Serving.** <claim id="mime-fallback" basis="observed">With the `.wasm` files served as `application/octet-stream`, streaming compilation was refused in all three engines and both loaders fell back and worked; no cross-origin isolation headers were sent and none were needed [the spike](#what-the-spike-measured)</claim>. So the package does not depend on how its host labels a file.
- **Publishing.** <claim id="release-gates" basis="observed">A release publishes through `release.yml`, where each registry is a job behind an environment that waits for the owner [release.yml:527](../../.github/workflows/release.yml#L527 "environment: crates-io") [release.yml:582](../../.github/workflows/release.yml#L582 "environment: pypi")</claim>. npm joins as a third: a job that builds the tarball, the existing attestation extended to cover it, a dry run in the verify job, and `publish-npm` behind a new `npm` environment. Nothing reaches npm without that approval, and an agent never gives it.

### Conformance and continuous integration

<claim id="spec-expects-ts" basis="observed">The conformance suite was written expecting this: its fixtures are language-agnostic and its README names "a TypeScript package" as the binding to come [spec/README.md:6](../../spec/README.md#L6 "a TypeScript package via")</claim>. A TypeScript runner reads the same files:

- **The parse fixtures** in `spec/tests/`, through `Engine.load` and `analyze`.
- **The outcome fixture**, a whole document and what `summarize` and `keyphrases` return over it. This is the round trip of a document through JSON, held by a fixture that already exists.
- **The error kinds**, against `spec/schema/v2/error.schema.json`.
- **Parity.** The texts of `spec/tests/parity/` through TypeScript, compared with what Rust writes for the same text, field for field.

A new `typescript` job in `ci.yml` builds both modules and runs that suite four ways: in Node, and in Chromium, Firefox and WebKit through Playwright, in a module worker, with the model served from the job's own machine. <claim id="playwright-pinned" basis="observed">The docsite already pins Playwright and installs a browser in its floor [site/package.json:30](../../site/package.json#L30 "playwright-core")</claim>.

Playwright's builds are the engines, not the browsers people install. The job therefore also runs the suite in the Chrome the runner image carries, and on a macOS runner in Safari through its own driver. Neither was tried in the spike. The milestone finds out whether each works, and the package's support statement names only what the job runs. iOS is not reachable from this job and stays unmeasured.

<claim id="wasm-job" basis="observed">The existing `wasm32 check` job, which holds the core to compiling for the target [ci.yml:106](../../.github/workflows/ci.yml#L106 "name: wasm32 check")</claim>, gains a line for the `wasm` feature.

### What the spike measured

One machine: Apple M1 Max, macOS 26.5, Node 26.0.0, Playwright 1.65.1 with its builds of Chromium 159.0.8154.6 (the headless shell), Firefox 159.0 and WebKit 27.2. The spike's files are not in the repository. Each milestone repeats its own part in continuous integration.

**Stage 1: the parser alone, in Node.** UDPipe 1 built with Emscripten 7.0.0, `-fwasm-exceptions -O2`.

| Measure | Native | WebAssembly in Node |
|---|---|---|
| Output over the 12 texts of `site/inputs/` (57 paragraphs, 6,529 words) | reference | byte-identical (`cmp`, and the same SHA-256) |
| Model load | 751 to 757 ms | 826 to 828 ms; 974 to 1,029 ms on a first load |
| Parse, a 3,347-word essay, median of 5, 3 invocations | 1,010 to 1,018 ms | 1,758 to 1,767 ms |
| Words per second | 3,289 to 3,312 | 1,894 to 1,903 |
| Memory | about 233 MB peak resident | 17.1 MB at start; 181.5 MB after one model load, unchanged after parsing, after five loads, and after freeing the model |

Four other builds (optimised for size, no file system, JavaScript exceptions, no exceptions) gave the same bytes. A fixed memory of 128 MB aborted; 192 MB worked. The module imports only the system shims Emscripten supplies: no threads, no shared memory, no `eval`.

Corrupted models: truncated and random models failed cleanly on every build, and the same instance then loaded a good model. Of 41 single-byte changes and truncations, 39 failed cleanly, one (the first byte) threw the uncaught exception, and one (offset 40) loaded and parsed. With the first byte changed, the build with WebAssembly exceptions threw something JavaScript caught; the build with none aborted; in each case the instance then loaded a good model and parsed.

**Stage 2: the core, in Node.** matra 0.3.0 with default features off, wasm-bindgen 0.2.130, the parser handed in from JavaScript.

| Measure | Result |
|---|---|
| Document JSON against the native command line, three texts | identical token for token (219,399, 7,819 and 8,524 tokens) when both read text; against a file read natively, only `provenance.source.path` differs |
| Summary and keyphrases over the same documents | identical |
| The comparer itself | caught a document with the last digit of one number changed |
| Whole process, the long essay | native 1.80 s; Node with WebAssembly 2.88 s |
| Markdown and block readers; the compression metric | work |
| Unknown format, invalid UTF-8, input one byte over the cap | fail with the library's own errors |
| A parser that throws, returns nonsense, a word not in the text, or a head out of range | each `parse failed`, from the core's own checks |
| A panic, with no hook | `RuntimeError: unreachable`; the instance still answered once, then failed when freed |

**Stage 3: browsers.** The same two binaries, by SHA-256. Three runs each on the main thread and in a module worker, a fresh browser process per run, a local server sending only a content type and a length.

| Measure | Chromium | Firefox | WebKit |
|---|---|---|---|
| Output against native, all runs | identical | identical | identical |
| Words per second, median, main thread and worker | 1,715 and 1,688 | 1,586 and 1,600 | 1,734 and 1,723 |
| Model load, cold, median | 1,059 and 1,071 ms | 1,242 and 1,266 ms | 985 and 1,000 ms |
| Both exception probes (older and newer instruction) | run | run, with a deprecation warning for the older | run |

Analyzing the long essay took 1.95 to 2.2 s, and the two short texts 61 to 69 ms. On the main thread the page was blocked for about one second while the model loaded and about two for the essay; in a worker a heartbeat on the page showed no gap. The parser's memory was 17.1 MB at start and 181.5 MB with the model; the core's was 1.9 MB at start and 5.9 MB after three documents. The browser process grew by about 280 to 320 MB.

**Sizes.**

| File | Raw | Gzip | Brotli |
|---|---|---|---|
| The parser, `-O2` with WebAssembly exceptions | 668,753 | 227,843 | 176,778 |
| The parser, no exceptions | 630,390 | 215,128 | not taken |
| The parser's loader script | 59,022 | 16,491 | not taken |
| The core, lean (analyze only) | 1,446,259 | 659,303 | 534,547 |
| The core, with outcomes and the spike's probes | 1,692,251 | 760,247 | not taken |
| The core's glue script | 12,953 | 3,339 | not taken |
| The model | 16,309,608 | 16,314,639 | 16,309,636 |

By code size, the lean core is 32.6% the compressor behind one metric, 13.1% binding shims, 11.9% the markdown reader and 8.0% matra.

### What was not measured

- Safari itself, any browser on iOS, and Chrome with a window. Chromium ran as the headless shell and WebKit as Playwright's macOS build.
- A model fetched over a real network. Every fetch was from the same machine.
- How GitHub Pages labels a `.wasm` or a `.udpipe` file, and which origins it lets fetch one. It was not contacted.
- A parser build on the newer exception instruction. Only a small probe of that instruction ran.
- A worker thread in Node. Node ran both modules on its main thread.
- A document's round trip through JSON into an outcome function, and the outcome functions' time alone.
- The core's own SHA-256 over the model; the size-tuned core's speed; a bundler; a strict content security policy; Deno and Bun.
- Embeddings in WebAssembly, and a threads build.
- Whether memory the engines agree to reserve can be committed on a small device. A phone's limit is unknown.
- Any second machine.

### The boundary test and the resilience floor

The ACES boundary test, on the design as recommended:

| Question | More | Less |
|---|---|---|
| Adaptable | A new target behind a non-default feature. The model's address is the caller's argument, so who hosts it can change with no release. Promises everywhere, so where work runs can change with no signature. The exception flavour is a build setting. | Only a pinned model loads. That is deliberate: a trust anchor is pinned, not configurable. The new adapter relies on a build without threads. |
| Composable | A second adapter of an existing port. One shared function replaces a rule that would otherwise exist twice. The binding is held to rule 7, and the composition root stays the only place adapters are wired. | The crate's manifest gains optional dependencies for a target most Rust callers never build. |
| Extensible | The fourth syntax uses the first three's names, its types come from the schema, and the conformance fixtures tell a contributor when it drifts. | Two toolchains, Emscripten and `wasm-bindgen`, stand between a contributor and a built package. One recipe and one job hide that, and do not remove it. |

The resilience floor, discipline by discipline:

| Discipline | Here |
|---|---|
| Size cap at the entry point | The wrapper refuses an oversized input before copying it; the core refuses it again. A model is refused by length before it is hashed, and a response is cut off at the pinned size. |
| Symlink rejection | Not applicable: the surface reads no file. |
| Atomic writes | Not applicable: the package writes nothing. |
| Verified bytes are the bytes loaded | The digest is taken over the worker's own copy, and that copy is what the loader reads. |
| Panic boundary at foreign code | Weaker than native, and said so. A C++ exception or trap becomes one of the library's kinds, as natively. A panic in the core cannot be caught on this target, so it costs the engine, not only the call; it is loud, and it names itself. |
| Cycle-safe walks | Unchanged: the same Rust runs. A document from anywhere is already an input the outcomes must survive. |

### Milestones

Smallest first. Each is one pull request with its documentation and CHANGELOG entry, and each leaves the tree shipping what it shipped before. None starts before acceptance.

1. **Types.** `typescript/` with the types generated from `spec/schema/v2/`, the check that keeps them current, and `MatraError`. No WebAssembly. A TypeScript program can already read what `matra analyze --json` writes.
2. **One copy of the mapping.** The Rust refactor under [The Rust side](#the-rust-side): the shared function, the pin moved, and the builder's default if [decision 2](#decision-token-deps) says so. Native output is held unchanged by the existing conformance suite.
3. **The core.** The `wasm` feature, `src/wasm/`, rule 7's check, errors as data, the panic hook, and `summarize` and `keyphrases` over a document, with the outcome fixture running in Node and the three engines. Measures the round trip.
4. **The parser.** The Emscripten build pinned to `Cargo.lock`, the digest before load, the worker, `Engine.load`, `analyze`, `analyzeAll` and `close`, with the parse and parity fixtures in Node and the three engines, then in Chrome and Safari. Measures what [decision 4](#decision-exception-handling) rests on, the Node worker, and the core's digest.
5. **The release.** The npm job and environment in `release.yml`, the docsite's install page, guide and reference, the `ffi-surface` skill brought up to date, a newcomer pass, and the first npm release, which publishes only on the owner's approval. The owner may instead release after milestone 1 or 3; each leaves a package that is useful alone.
6. **The playground.** A docs page that loads the package and renders what it returns. It waits on [who serves the model](#who-serves-the-model).
7. **A component that renders a parse.** It needs only a document's JSON, so it works with or without the engine. Its element name and attributes are new public names and come as their own API change proposal.

## Drawbacks

- **About half the native speed, and 181 MB a page keeps.** Both measured. Neither is tuned, and neither may be tunable.
- **The model is 95% of the download and cannot be made smaller by compression.**
- **A panic costs the engine.** Natively a caught panic costs one call.
- **A second toolchain.** Emscripten joins the build, pinned, for one artefact.
- **A release has a third approval,** and a third registry where a mistake is permanent.
- **A third of the core is one metric's compressor.** The proposal keeps it: a document that differs by build is a worse cost than its size.
- **The evidence is one spike on one machine.** The list of what was not measured is longer than the list of engines that were.

## Rationale and alternatives

### Do nothing

The roadmap keeps a statement the spike disproved, and a TypeScript caller keeps reading matra's JSON with hand-written types. The owner has asked for more than that.

### One module, not two

Compile matra with its `udpipe` feature for Emscripten's Rust target, so the native adapter runs as it is and nothing is shared or copied. It was not tried. The `udpipe` feature brings a network client that has no meaning in a browser, and the binding layer the spike used is built for the other target. Two modules are what was measured to work, and the core in that design is also the artefact that serves a page with no parser.

### A `Document` class holding a handle to Rust memory

It avoids sending a document back to the core for each outcome. It also gives every document a lifetime the caller must end, makes a document from a server a different kind of thing from a document from the engine, and cannot cross from the worker to the page without becoming data anyway. The spike saw what a stale handle does after a panic.

### A server that parses

The roadmap named it. It is a product, not a library: it needs hosting, and it sends a caller's text to someone else's machine. A caller who wants one can build it on the Python package, and the TypeScript types and outcome functions here serve its output.

### Let the caller name any model

The caller passes a model and its digest, and the package checks one against the other. That verifies only that the caller hashed their own file. It reopens what 0.3.0 closed in Rust and Python, and it puts the model that loads silently with one byte changed one typo away. A caller with a parser of their own is a future addition, as `Engine::with_parser` is in Rust.

### Fetch the model from a default address

It would make the first call one line shorter. The model's licence forbids the package from deciding that for a commercial caller, its host refuses the request, and a default address is a promise to keep a file online for as long as any released version exists.

## Prior art

- **matra's Python binding.** A module of the crate behind a feature, held to the public surface by a boundary rule, with a typed stub and a conformance runner. This proposal follows it step for step.
- **`udpipe_flutter`** on pub.dev (0.2.1, published 2026-06-14) ships UDPipe 1 as "native FFI on desktop/mobile and WebAssembly on the web". Someone else already runs this parser in a browser, which the roadmap's entry did not know.
- **The conformance suite's own plan.** `spec/README.md` has named a TypeScript binding since the fixtures were written.
- **The `ffi-surface` skill** sketched this as a `wasm` feature and a `src/wasm.rs`, or a separate crate, to be decided when a caller appeared. [Decision 1](#decision-binding-home) decides it.

## Unresolved questions

<decision id="binding-home" title="1. Where does the Rust side of the binding live?" reversible="yes" grounds="mapping-private spike-copied python-in-crate">

<choice key="a" title="In the crate, behind a `wasm` feature">

`src/wasm/` and a second parser adapter, as [The Rust side](#the-rust-side) describes. The mapping is shared inside the crate and no new Rust name is published.

</choice>

<choice key="b" title="In a second crate that uses only the public surface">

A crate beside matra's in a workspace, depending on matra as any caller does, as the spike was built. The span rule and the mapping become public functions so it has one copy to call.

</choice>

<recommendation choice="a">

The crate already holds a binding this way, and the reasons carry over: one version, one composition root, one place the boundary rules are checked. The copies disappear with no addition to the Rust surface, where every name is permanent. A second crate would need a workspace, a second manifest kept in step, and two functions published for one caller.

</recommendation>

<against>

The spike is the proof that the public surface was almost enough: an outside crate built a working binding and needed three private pieces. Choice (b) would keep that proof running on every commit, and would give anyone else writing a parser the span placement the engine checks them against, which today they write themselves. It also keeps `wasm-bindgen` out of matra's manifest.

</against>

</decision>

<decision id="token-deps" title="2. What does a token built through the builder say in a column the parser left out?" reversible="costly" grounds="builder-deps-empty adapter-deps-underscore">

<choice key="a" title="`_` for `deps`, as matra's own adapter writes">

`Token::builder` starts `deps` at `_`. A parser with no enhanced graph then says so the way every document matra has produced says so.

</choice>

<choice key="b" title="Leave the default empty and document the difference">

The contract already allows `_` or empty. Each outside parser sets `deps` itself if it wants to match.

</choice>

<recommendation choice="a">

Two spellings of "nothing here" in one field make two documents over the same text differ for no reason a reader can use, and the spike only matched native output after setting this by hand. It is a one-word change to a default, in 0.x, with a line in the CHANGELOG.

</recommendation>

<against>

It changes what an existing caller's parser emits without that caller changing anything. It also leaves `xpos` and `misc` starting empty, so the builder would then be inconsistent with itself where today it is only inconsistent with the adapter.

</against>

</decision>

<decision id="analyze-all" title="3. What does `analyzeAll` return?" reversible="costly" grounds="python-lazy parse-seconds">

<choice key="a" title="A promise of an array">

`const docs = await engine.analyzeAll(inputs)`, as first sketched. Every result is held until the last is done, each a document or a `{ path, error }` item.

</choice>

<choice key="b" title="An async iterable">

`for await (const item of engine.analyzeAll(inputs))`. Items arrive in input order as each finishes. `await Array.fromAsync(engine.analyzeAll(inputs))` is choice (a) in one line.

</choice>

<recommendation choice="b">

<claim id="python-lazy" basis="observed">Python's `analyze_all` is an iterator, "lazily and in input order", and so is Rust's [_core.pyi:249](../../python/matra/_core.pyi#L249 "Iterator[Document | DocumentError]")</claim>, so (b) is the same contract in a third syntax and (a) is a different one under the same name. <claim id="parse-seconds" basis="observed">A long document takes about two seconds [the spike](#what-the-spike-measured)</claim>, so a page analysing ten wants the first after two seconds, not all ten after twenty, and a Node program walking a folder wants neither every input nor every result in memory at once.

</recommendation>

<against>

The owner was shown (a), and an awaited array is what a JavaScript reader expects. With no directory input in this surface, a caller who wants results one at a time can loop over `analyze`, which makes `analyzeAll` a convenience either way, and the simpler convenience is the array.

</against>

</decision>

<decision id="exception-handling" title="4. Which exception handling does the parser build use, and which browsers does that admit?" reversible="yes" grounds="legacy-only-build firefox-deprecates probe-both digest-shields try-table-build callers-current-engines">

<choice key="a" title="The older instruction, as measured">

Ship the build the spike ran in three engines. It works today everywhere it was tried, and Firefox logs a deprecation warning on every load.

</choice>

<choice key="b" title="The newer instruction">

Build with the instruction Firefox asks for. The package then needs an engine that has it, and the support statement says which, from what the conformance job runs.

</choice>

<choice key="c" title="No exception handling in the binary">

Any C++ throw becomes an abort. The smallest build, with no dependence on either instruction. A throw UDPipe would have caught itself becomes a closed engine.

</choice>

<recommendation choice="b">

<claim id="legacy-only-build" basis="observed">The build the spike measured contains only the older instruction [the spike](#what-the-spike-measured)</claim>, and <claim id="firefox-deprecates" basis="observed">Firefox reports that instruction as deprecated and asks for the newer one [the spike](#what-the-spike-measured)</claim>. <claim id="probe-both" basis="observed">All three engines ran a probe of each [the spike](#what-the-spike-measured)</claim>. <claim id="try-table-build" basis="inferred" likelihood="likely">A parser built on the newer instruction will give the same bytes in those three engines: five builds with different settings already did, and the engines accept the instruction</claim>. That is a prediction, and milestone 4 tests it before anything ships; if it fails, (a) is one build setting away.

The direction matters more than the flavour. Moving from (b) to (a) later widens what the package runs on and breaks nobody. Moving from (a) to (b) later, when an engine drops the older instruction, narrows it under callers who are already relying on it. The digest check means no model can reach the throw the spike found, so what remains is defence in depth, and (c) gives up the one thing exceptions buy: UDPipe behaving as it does natively when it catches its own.

</recommendation>

<against>

(a) is the only flavour that has parsed a document in a browser. (b) rests on a probe and a prediction. <claim id="callers-current-engines" basis="assumed">It also assumes the people who will use a new package run engines recent enough to have the newer instruction</claim>, and the one platform most likely to lag, a phone's browser, is the one nothing here measured. A warning in a console is not a failure, and no date for the older instruction's removal is in evidence.

</against>

</decision>

<decision id="core-panic" title="5. What does a caller see when matra's own code panics?" reversible="costly" grounds="panic-unreachable native-catches-panic kind-open-set">

<choice key="a" title="An error outside the kinds, and a closed engine">

The rejection is an `Error` that is not a `MatraError`, with the panic's message and location and the trap as its `cause`. The engine closes.

</choice>

<choice key="b" title="A new kind, `internal`, on every channel">

`MatraError` with `kind: "internal"`, added to the error schema and to Rust, Python and the command line.

</choice>

<choice key="c" title="Report it as `parse_failed`">

No new name anywhere. A defect in matra reads as a parser failure.

</choice>

<recommendation choice="a">

A kind is a failure the library reports and a caller can act on. A panic is a defect in matra, and each language already has its own word for that which is outside the error contract: an unwinding panic in Rust, an exception that is not a `MatraError` in Python. TypeScript's is an `Error` that is not a `MatraError`. It must be loud, name itself and say where, which the hook provides, and it must close the engine, because the spike saw the instance misbehave afterwards. (c) hides a defect behind a kind that tells the caller to check their input.

</recommendation>

<against>

"Failures reject with an error carrying the library's kind" stops being true of every rejection. <claim id="kind-open-set" basis="observed">The schema calls the kinds "an open set" [error.schema.json:10](../../spec/schema/v2/error.schema.json#L10 "An open set.")</claim>, so (b) is an addition, not a break, and it would give a caller one `catch` that handles everything and a `kind` to log.

</against>

</decision>

<decision id="package-name" title="6. What is the package called on npm?" reversible="costly" grounds="one-name">

<choice key="a" title="`matra`">

The name on crates.io and PyPI. On 2026-11-18 the registry at `registry.npmjs.org/matra` answered 404, so no package holds it; that is not a promise npm will grant it.

</choice>

<choice key="b" title="A scoped name under an organisation the owner controls">

`@scope/matra`. Always available, and it shows who publishes it.

</choice>

<recommendation choice="a">

<claim id="one-name" basis="observed">matra ships under "one name, one version and one attribution" [EPR-0003:13](0003-distribution-and-provisioning.md#L13 "one version and one attribution")</claim>. `import ... from "matra"` in a third language is the same rule.

</recommendation>

<against>

An unscoped name is a claim on a shared namespace that npm may refuse or a similar name may contest, and a scope is the registry's own answer to who a package belongs to. If the name is taken between this text and the first release, (b) is what remains.

</against>

</decision>

### Who serves the model

This is the owner's decision and Claude recommends no option. The facts: the model is CC BY-NC-SA 4.0; the package will not carry it; a browser cannot fetch it from its publisher. So every browser caller needs it at an address their page may fetch, and the playground is a browser caller. The npm package behaves the same under all three options: the caller passes a URL or bytes.

| Option | What it gives | What it costs |
|---|---|---|
| The docs site serves the current model, with attribution | The playground works as soon as the package does. The docs can point a browser caller at a working address. | The project redistributes a work licensed for non-commercial use only, under that licence's attribution and share-alike terms. Whether the project's hosting, and each use made of a public address, is non-commercial is a judgment about a licence, and it is the owner's. 16.3 MB joins every deploy and every first visit to the playground. How GitHub Pages serves the file, and to which other sites, is unmeasured. |
| The playground waits for the permissively licensed model | Nothing the project hosts carries a non-commercial term. One address could then serve every caller. | The playground waits on a model that is not yet trained or measured, with no date. Until then a browser caller hosts the current model themselves. |
| The caller always supplies it, the playground included | The project redistributes nothing. | The playground asks each visitor to download 16 MB by hand and select the file, since the page cannot fetch it for them. That is not "nothing installed". |

<claim id="callers-can-host" basis="assumed">Under every option the package assumes a browser caller is able to put a 16 MB file on an origin of their own</claim>.

What the owner should confirm or strike, collected from the claims above that stand on no grounds yet:

<assumptions />

## Future possibilities

- **`clusters`**, once embeddings are measured in WebAssembly. The embedding adapter already compiles for the target.
- **A caller's own parser**, as `Engine::with_parser` allows in Rust, when a caller has one.
- **`Input.file` for Node**, as a Node-only entry, when a Node caller asks for the path in provenance.
- **A progress notice for the model download**, as the native engine offers one. Today a caller who wants progress fetches the bytes and passes them.
- **The permissively licensed model**, as a second pin, recognised by its digest.
- **A smaller core**, by measuring the size-tuned build's speed.
- **The feed for text arriving over time**, in TypeScript when it lands in the other three syntaxes.
