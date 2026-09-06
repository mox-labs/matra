# matra 0.2.0, cold-start user experience report

> **Point-in-time record.** This is the report of one exploratory pass against
> commit `7197ca0`, kept as the evidence behind
> `.claude/skills/e2e-validation/SKILL.md`. It describes what matra was on
> 2026-09-06, not what it is now, and several findings below were fixed before
> 0.2.0 shipped. The `[0.2.0]` section of `CHANGELOG.md` records what was done
> about them. Nothing here should be read as a description of current
> behaviour.

**Scope.** A first-time user installs matra from nothing, follows every shipping
document literally, then hands only `matra --skill` to a fresh agent and asks it
to do a real task. Report only; nothing in the repo was edited.

**Commit under test.** `7197ca0` (main), in an isolated worktree. Worktree clean
at the end (`git status --short` empty).

**Environment.** macOS 15 / darwin 25.5.0, arm64. Rust 1.95.0, Python 3.14.5,
uv 0.9.13, maturin 1.13.1.

**Sandbox.** Everything ran under a launcher setting `HOME`, `XDG_CONFIG_HOME`,
`XDG_DATA_HOME`, `XDG_CACHE_HOME` and `CARGO_HOME` inside
`scratchpad/e2e/`, with `MATRA_CONFIG_FILE`, `MATRA_DATA_DIR` and
`MATRA_MODEL_DIR` unset. This mattered: the real machine already has a populated
legacy `~/.matra/models`, which `installation.md:62` documents as a fallback,
so without the override the "cold start" would silently have been warm.

**Verified untouched at the end.** `~/.config/matra` and `~/.local/share/matra`
still do not exist; `~/.matra/models` mtimes unchanged (`english-ewt-...udpipe`
2 Aug 12:22, `potion-base-8M` 4 Sep 22:40); no `matra` in the real
`~/.cargo/bin`. All deletions used `rip`.

The running first-person log kept during the pass was not retained. This file is the
findings.

---

## Summary

| | Count |
|---|---|
| **Blocks release** | 3 |
| **Hurts first impression** | 6 |
| **Cosmetic** | 5 |

Two headline facts. First, the product itself is in good shape: install to
working binary is **0.78 s**, and essentially every command in every guide ran
as printed, including two `jq` one-liners, every Python snippet, and every Rust
snippet transcribed into a real crate. Second, the *first ninety seconds* are
worse than the product deserves, and one page (`guides/python.md`) contains a
statement that is simply false.

---

## Timings that matter

Build time is excluded, a published wheel arrives prebuilt.

| Measurement | Result |
|---|---|
| Wheel size (`matra-0.2.0-cp314-cp314-macosx_11_0_arm64.whl`) | **4.3 MB** |
| `uv pip install <wheel>` | **0.105 s** |
| ...to a working `matra --version` | **0.78 s total** |
| `uv tool install <wheel>` (the `uvx` shape) | **0.049 s** |
| **First `matra analyze` in a cold environment** | **22.7 s** |
| The same command, warm | **1.06 s** |
| ⇒ silent network wait on first run | **~21.6 s** |
| `Model2Vec.potion_base_8m()` cold (29 MB) | **1.2 s** |

The last two lines together are the finding. The 29 MB embedding model arrives
in 1.2 s on this connection; the 16 MB UDPipe model takes 21.6 s. The wait is
the UDPipe host being slow, and the user is told nothing while it happens. A
second cold run measured 22.7 s and a third ~14 s, so call it **14-23 s, highly
variable**.

---

## Blocks release

Ranked by how early a new user hits it.

### B1. `guides/python.md:190` states something false, and teaches 20 lines of unnecessary code

`book/src/guides/python.md:190`:

> Methods do not cross the FFI boundary, only fields do. Rust's `Document` has
> `passive_ratio()`, `mean_sentence_length()`, `total_sentences()`,
> `total_words()`, and `sentence_length_std()` as methods, and **none of them are
> reachable from Python.** Compute what you need from the fields you already have.

Four of those five are indeed absent. `passive_ratio` is a field:

```
$ python -c "...print(sorted(result.keys()))"
top-level Document keys: ['nominalization_ratio', 'passive_ratio', 'sections', 'vocabulary_ttr']
  passive_ratio: PRESENT -> 0.5
  mean_sentence_length: absent
  total_sentences: absent
  total_words: absent
  sentence_length_std: absent
```

The page then spends `python.md:192-212` teaching the reader to hand-roll a
`passive_ratio` computation, including a warning to "keep the tuple in sync if
you copy this into your own code", for a value that is one dict lookup away.

Two other shipping pages get it right and contradict this one:
`book/src/introduction.md:24` does `print(result["passive_ratio"])` (verified:
prints `0.5`), and `book/src/capabilities.md:63` correctly lists `passive_ratio`
in the Python-crossing fields column *and* `passive_ratio()` in the Rust-only
methods column. Commit `e68e52d` fixed `reference/methodology.md` for exactly
this; `guides/python.md` was missed.

This is on the page a Python user reads first, and it costs them real code.

### B2. Published wheels are cp312-only, so `pip install matra` on Python 3.13/3.14 builds from source

`book/src/tutorials/installation.md:11`:

> **Python 3.12 or later** ... Wheels ship for Linux x86_64 and macOS (Intel and
> Apple Silicon); on any other platform `pip` builds the sdist, which
> additionally needs the Rust toolchain.

and `installation.md:56`:

> On the platforms with wheels this downloads a prebuilt binary and installs in
> seconds.

`README.md:41` says the same, with "Anything else builds from the sdist", which
reads as *platform*, but the gating dimension is *Python version*.

Evidence, direct: `.github/workflows/publish-pypi.yml:111` pins
`python-version: "3.12"` for all three wheel targets, and there is no `abi3`
anywhere (`grep -n abi3 Cargo.toml pyproject.toml` → no match; pyo3 is declared
with `features = ["extension-module"]` only). So wheels are version-locked
`cp312`.

Observed consequence on the already-published 0.1.0, whose wheels are the same
shape (`matra-0.1.0-cp312-cp312-macosx_11_0_arm64.whl` per PyPI):

```
$ uvx matra --skill
   Building matra==0.1.0
      Built matra==0.1.0
```

This is macOS Apple Silicon, a platform the docs say ships a wheel, and uv
built from the sdist anyway, because the interpreter is 3.14. Homebrew's default
`python3` is 3.14 today. A reader on a documented-supported platform gets a
source build requiring a Rust toolchain, contradicting "installs in seconds".

Either the wheel matrix needs to cover 3.12-3.14, or pyo3's `abi3-py312` feature
needs enabling so one wheel covers 3.12+, or both sentences need to name the
Python-version constraint explicitly.

### B3. `reference/errors.md:189` promises a message format the two most common errors do not follow

`book/src/reference/errors.md:189`:

> On exit code 2 the command writes `matra: ` followed by the error's `Display`
> string to standard error.

with the `Display` strings tabulated at `errors.md:111-119`, e.g.
`Io(e)` → `io error: {e}`, and `errors.md:91` specifying `not a regular file:
<path>` for a non-regular-file rejection.

Observed:

```
$ matra analyze nope.md
matra: no such file: nope.md            EXIT=2
$ matra analyze corpus
matra: corpus is a directory; pass a file. Directory analysis is on the roadmap.   EXIT=2
```

Neither is a `Display` string from the table. (The other two I triggered *are*
verbatim: `matra: big.md: file_source input too large: 9360000 > limit 8388608`
matches `errors.md:116`, and `matra: link.md: io error: refusing to read
symlink: link.md` matches `errors.md:90`.)

The CLI's actual messages are **better** than the documented ones, so the fix is
to the doc, not the code. But "missing file" and "directory passed" are the two
errors a new user hits first, and `errors.md` is the page an agent integrator
builds a parser against. As written, the contract is false for its two most
important cases. Worth noting `errors.md:189` also omits that the CLI prefixes
the path (`matra: link.md: io error: ...`), which is `DocumentError`'s `Display`
(`errors.md:99`), not `Error`'s.

---

## Hurts first impression

### H1. 14-23 seconds of a completely blank terminal on the very first run

The single worst moment in the experience, and the earliest.

```
$ matra analyze essay.md
                                      <- nothing. at all. for ~21 seconds.
essay.md
  sentences          14
  ...
1.08s user 0.17s system 5% cpu  22.718 total
```

I captured a cold run on a pty and polled the tty byte count once a second:
**zero bytes for thirteen consecutive polls**, then the entire table at once. No
spinner, no "downloading the English model (16 MB)", no byte counter, no host
name. The `5% cpu` is the tell, the process is asleep on a socket.

Honestly reported: at ~8 s I re-checked I had not typo'd the filename. At ~15 s
I was deciding whether to Ctrl-C. And I had *read* `installation.md` and knew a
download was coming. A user who typed the README's own line has been told the
opposite, `README.md:17` titles the section "**No setup**" and says "Nothing
installed, nothing configured, no flags."

`installation.md:91` softens it to "can take several seconds depending on your
connection". Twenty-two seconds on a link that pulls 29 MB in 1.2 s is not
"several", and on hotel wifi this is a minute-plus of dead screen. One line on
stderr before the fetch would close this entirely, and stderr keeps `--json`
clean.

### H2. `README.md:13`, the first instruction an agent is given, currently fails

> If you are an agent, run `uvx matra --skill`.

```
$ uvx matra --skill
Installed 6 packages in 8ms
Usage: matra [OPTIONS] COMMAND [ARGS]...
Try 'matra --help' for help.

Error: No such option '--skill'.
```

Not a name squat, PyPI `matra` is the owner's own 0.1.0 from 2026-09-04, whose
CLI is a Click program with no `--version`, no `--skill` and no `config`. This
resolves itself the moment 0.2.0 publishes, and I verified the mechanism works:
`uv tool install <0.2.0 wheel>` then `matra --version` prints
`matra 0.2.0 / features: udpipe model2vec python cli`. **I can only approximate
the real `uvx` path**, I cannot test PyPI resolution of an unpublished version.

Flagging it because it is a live trap *right now* for anyone reading the repo
before publish, and because B2 means the uvx path will still be slow after
publish on 3.13/3.14.

### H3. Exit code 1 prints nothing at all

```
$ matra keyphrases empty.md
                                      <- no output
EXIT=1
```

Documented (`cli.md:238`, `errors.md:186`) and correct. But a human sees a
command that produced no output and no message and must go read a table to learn
that silence means "found nothing" rather than "crashed quietly". A single line
on stderr ("no keyphrases found") would cost nothing and would not disturb
`--json` or the exit code.

### H4. Keyphrase output is hard to trust and the scales are undocumented

On a document titled "Choosing indexes", entirely about database indexes:

```
$ matra keyphrases essay.md
14.500  cheapest performance work available
9.000   full business cycle
8.000   particular read pattern
```

The word "index" appears nowhere in the top ten. Switching method:

```
$ matra keyphrases essay.md -n 20 --method yake
73.001  bet
71.541  bet particular
54.287  index bet
```

`cli.md:107` explains both mechanisms honestly and `cli.md:111` warns about
lemmatisation and tie instability. What no page says is (a) that RAKE's top
phrase routinely is not the document's topic, and (b) that RAKE scores (3-15
here) and YAKE scores (6-73 here) are on unrelated scales, or which direction is
"better" for each. `capabilities.md:98` mentions YAKE's "score inverted so higher
is more relevant", but that sentence is on the capabilities page, not on the CLI
guide or in the skill, so the reader most likely to need it will not see it. I
could not have told you whether `bet` at 73.001 was a strong or a weak result.

### H5. `config show` cannot tell you where your config file is

`matra config show` prints `data_dir` and `model_dir` with origins, but no
config-file path. `cli.md:29` documents the location and `cli.md:35` says
"`matra config show` prints which rung each value actually came from", but for
"where is my config", the reader has to know to run `config show --json` (the
path is the envelope's `input`) or `config init`. `installation.md:93` sends a
user to `config show` when something is wrong with directories, which works for
the model dir and not for the config.

### H6. `SemanticClusters.threshold` is not the value you passed

```python
result = v.semantic_clusters(text, 0.85, model)
print(result["threshold"])   # 0.8500000238418579
```

Both `book/src/guides/semantic-clusters.md:13` and the **shipped skill**
(`skills/matra/references/semantic.md:25`) describe this field as "the cutoff you
supplied". It is the f32 round-trip of what you supplied, so
`result["threshold"] == 0.85` is `False`. An agent following the skill and
round-tripping the threshold through a result will get a surprise. Either the
doc should say "the cutoff you supplied, as f32", or the field should carry the
f64.

---

## Cosmetic

### C1. `installation.md:41-44` prints a `--version` output the Python route does not produce

The page shows

```
matra 0.2.0
features: udpipe cli
```

which is exactly right for the `cargo install matra --features cli` route I
verified. The Python package prints `features: udpipe model2vec python cli`
(also verified). The block sits under "The CLI binary" so it is not false, but
`installation.md:54` immediately tells the reader the pip route gives "the same
Rust CLI", and `installation.md:56` points at the verify step; a reader who runs
`matra --version` after `pip install` sees a different second line than the only
`--version` sample on the page.

### C2. `input input too large`

```
builtins.ValueError: input input too large: 9360000 > limit 8388608
```

Faithful to the documented format at `errors.md:116` (`{what} input too large`)
with `what = "input"`, so the doc is correct and the message still reads like a
bug. Every other gate label reads fine (`file_source input too large`, `tfidf
input too large`).

### C3. `capabilities.md:51` lists two passive relations where the other pages list three

`capabilities.md:51` describes `is_passive()` as "whether the sentence carries
`nsubj:pass` or `aux:pass`". `rust.md:269` and `python.md:214` both say
`nsubj:pass`, `nsubjpass`, or `aux:pass`. `nsubjpass` (the UD v1 spelling) is
missing from the capabilities page.

### C4. `rust.md:196`'s builder example gives no types

`Token::builder(id, text, lemma, pos, head, dep).build()`, the string arguments
are `String`, not `&str`. This was one of only two compile errors in my
transcription of the entire Rust guide, and rustc's suggestion fixes it
instantly, so it costs seconds. Mentioned only because `rust.md:196` frames this
as the thing "that matters most when you are building fixtures for tests", i.e.
its readers are copy-pasting it.

### C5. `matra summarize` rounds three distinct scores into three identical ones

```
0.357  Column order in a composite index is not a detail.
0.357  The counterpart to adding an index is removing one.
0.357  Databases record how often each index has been read.
```

`--json` shows `0.3567448211624095` and neighbours, so the scores do differ. The
human table's 3-decimal rounding makes the ranker look broken. Also, TF-IDF
picked the three shortest sentences in the document, which reads like a bug until
you know the algorithm. Documented behaviour (`cli.md:84`, `cli.md:86`, order is
document order, verified correct), just an unfortunate first look.

---

## What was clean, explicitly, nothing found

These were walked command by command and are accurate. Saying so is part of the
report.

- **`book/src/guides/cli.md` end to end.** `--help`'s first line matches
  `cli.md:3` verbatim. `analyze`, `--sections`, `-s`, `--json`, both `jq`
  one-liners at `cli.md:60` and `cli.md:66`, `summarize -n 5 --method textrank`,
  `keyphrases -n 20 --method yake`, `completions zsh` and `fish`,
  `analyze - --stdin-filename notes.md`, `keyphrases -` from a pipe. All ran, all
  as described.
- **`matra config init` / `config show`.** Wrote to
  `$XDG_CONFIG_HOME/matra/config.toml` exactly as `cli.md:29` says, printed the
  path, refused to overwrite (exit 2 with the right message), overwrote with
  `--force`, and re-attributed every key's origin to the file afterwards. The
  `--json` form matches `cli.md:137` field for field (`origin`, `source`,
  `value`).
- **The JSON envelope.** `format_version: 1`, `command`, `input`, `result`, 
  matches `cli.md:223-230` on every command tested. `input` is `null` for
  `skill` and `result.name` is `"SKILL"`, exactly as `cli.md:171-178` prints.
  `ScoredSentence` is `{text, score, position}` (`cli.md:89`); `Keyphrase` is
  `{phrase, score}` with no position (`cli.md:109`).
- **Exit codes.** 0 / 1 / 2 all as `cli.md:238` and `errors.md:183-187`.
  Broken pipe verified 0 (`matra analyze essay.md --json | head -1` →
  `matra=0`). `-r` without `--skill` → 2 with a message naming the fix.
  `--skill -r nosuchref` → 2 listing the six real names. `--skill` outranks a
  subcommand and never reads the file (`cli.md:167`).
- **Every Python snippet ran exactly as printed.** `installation.md:72-80`
  produced `sections: 1` / `vocabulary_ttr: 0.8571428571428571`, matching
  `installation.md:87-88` digit for digit. `introduction.md:14-26`,
  `python.md:98-103` (`analyze_path`), `matra.ERROR_KINDS` in the documented
  order (`python.md:105`), `semantic-clusters.md:82-91`.
- **Python exception classes are exactly as documented** (`errors.md:152-160`,
  `python.md:220-228`): `FileNotFoundError` for a missing model with the message
  `model not found: models/english-ewt-ud-2.5-191206.udpipe` verbatim from
  `errors.md:170`; `ValueError` for the 8 MiB cap and for the 2000-sentence
  `tfidf` cap; `OSError` for a missing directory; `RuntimeError` for an
  unloadable model.
- **Every Rust snippet compiles and runs.** A scratch crate depending on the
  worktree by path, containing `rust.md`'s snippets plus `README.md:48-62` and
  `introduction.md:30-46`, compiled with two errors, **both mine, neither the
  docs'**. `Engine::with_defaults`, `Config::resolve().with_model_dir`,
  `cfg.sources()`, `Udpipe::english`, `Engine::new` + `standard_decomposers`,
  `Ingest::text`/`path`, `annotate`/`compose`, `tfidf_summarize`/
  `rake_keyphrases` over a shared slice, `CorpusResult` partitioning, the
  `Error` match at `rust.md:171-190`, `root_token`/`children_of`/`subtree`/
  `head_of`/`tree_depth`/`word_count`/`is_passive`, `Token::builder`.
- **The apparent README-vs-introduction conflict is not one.** `README.md:59`
  uses `analysis.passive_ratio()` as a method and `introduction.md:45` uses
  `doc.passive_ratio` as a field. Both compile, Rust permits a field and a
  method of the same name. Both pages are correct. (Only `python.md:190` is
  wrong; see B1.)
- **Size, symlink and TOCTOU behaviour.** 9.4 MB file rejected at the
  `file_source` gate with the documented string; symlink refused with
  `refusing to read symlink` (`errors.md:90`).
- **Both install routes produce identical output.** `cargo install --path .
  --features cli,udpipe` and the wheel's console script give byte-identical
  `analyze` output, supporting `README.md:39` and `cli.md:13`.
- **Model provenance.** `model_hash` came back as
  `81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4`, exactly the
  constant printed at `semantic-clusters.md:41`.
- **`AGENTS.md`.** Contributor-facing, correctly redirects a *using* agent to
  `matra --skill` at `AGENTS.md:5-6`. Nothing to report.
- **`book/src/explanation/*.md`.** No shell commands to execute. Claims spot-
  checked against behaviour (notably the configuration resolution order at
  `programming-model.md:38-58`) held.

---

## Phase 3, the agent door

I read only `matra --skill`, then `matra --skill -r semantic` and
`matra --skill -r python` when the top level told me to drill down. Task: of
three markdown files, name the two most semantically redundant, then give the
top three keyphrases of the most redundant one.

**Result produced, and it is correct:**

```
0.9020  oncall-rotation.md  <->  paging-policy.md
0.4527  index-design.md     <->  paging-policy.md
0.4521  index-design.md     <->  oncall-rotation.md
```

The two on-call documents are near-paraphrases of each other; the index one is
unrelated. matra separated them decisively.

```
$ matra keyphrases corpus/oncall-rotation.md -n 3 --json | jq -c .result
[{"phrase":"weekly planning meeting","score":9.0},
 {"phrase":"outgoing engineer record","score":6.1},
 {"phrase":"outgoing engineer","score":4.1}]
```

### Where the skill was sufficient

- **The CLI half was frictionless.** Commands, flags, envelope, exit codes, and
  the fact that a directory is refused, are all stated plainly at the top level.
- **The one sentence that saved the most time**, in `-r semantic`: *"There is no
  command line for this. It is a library and Python call."* Without it I would
  have hunted for a `matra semantic` subcommand. This is the skill doing its job.
- **The caveat table stopped me giving a wrong answer.** Asked which single
  document is "most redundant", the obvious reaches are `compression_ratio` and
  `vocabulary_ttr`. The skill's "Does not mean" column says compression ratio is
  *not* "Redundancy of meaning", and that TTR is not comparable across documents
  of different lengths (mine are 16 vs 12 sentences). I declined to answer rather
  than answering wrongly. That is a designed-in win and it worked.
- **`model_hash` discipline landed.** The skill insists provenance travels with
  the score; I recorded it without being prompted.

### Where I had to guess, three specific gaps

1. **There is no cross-document primitive, and no sentence says so.**
   `-r semantic` describes clustering over "the sentences of one document"
   throughout. The task is across documents. The skill neither offers a
   cross-document route nor states that one does not exist, so I had to invent
   one: embed each whole document as a single "text" and cluster the three
   vectors.
   *Missing sentence:* in `semantic`, something like, "Clusters are over the
   sentences of one document. To compare whole documents, embed each document as
   one text with `Model2Vec.embed` and cluster the resulting vectors with the
   module-level `semantic_clusters`; note the useful threshold band for
   document-level vectors is higher than for sentences."

2. **`Model2Vec.embed`, the actual unlock, is unreachable from where you need
   it.** `-r semantic` names the module-level
   `semantic_clusters(embeddings, threshold, model_hash)` but never says how to
   obtain embeddings; it only says "See `python` for the signatures". `-r python`
   does document `embed`, but the top-level reference table describes that file
   as "The Python API, the `Embedder` protocol, `analyze_path`, and exception
   mapping", nothing suggesting raw vectors. I found it by reading the whole
   file, not by being routed there.
   *Note:* `book/src/guides/semantic-clusters.md:93` **does** have the sentence
   ("`model.embed(texts)` returns the raw vectors when you want to do something
   else with them"). It is in the docsite and not in the shipped skill. That one
   sentence, copied into `-r semantic`, closes this gap.

3. **The 0.85 threshold guidance is sentence-calibrated and silently wrong at
   document level.** The skill says "Start at 0.85 with the reference model".
   At document level my *unrelated* pair scored 0.45 and my near-duplicate pair
   0.90, the whole scale has moved. I only found this because I set the
   threshold to `0.0` to force every pair to emit an edge so I could read raw
   cosines, which is a trick the skill neither describes nor sanctions.
   *Missing example:* one worked example in `semantic` showing raw pairwise
   scores being read off edges, and a note that thresholds are not portable
   across granularities.

### Where I would have gone to the wrong answer without repo access

Being precise, because this is the question that matters: **on the main result,
nowhere.** The pair answer is correct and I reached it using only documented
surface. I did not need the repo.

Two places I would have gone wrong on *presentation* rather than substance:

- **Threshold 0.85 taken literally at document level.** An agent that followed
  the skill's starting value without probing would have clustered all three
  documents together on a slightly tighter corpus, or none of them, and reported
  either as fact. Nothing in the skill flags that the number does not transfer.
  This is the highest-value fix on the list.
- **`result["threshold"]`.** Had I echoed it back as "clustered at the 0.85 you
  asked for", I would have printed `0.8500000238418579` (see H6).

And one thing I could not have known without the repo, stated plainly: **a
`cargo install matra --features cli` user cannot do any of this.** That binary
reports `features: udpipe cli`, no `model2vec`. The skill's `-r semantic` says
"only the shipped adapter sits behind the `model2vec` feature" but never tells a
CLI user that the standard install line at `installation.md:28` omits it. I
learned that from `Cargo.toml`, which a real user has no reason to open.

---

## What I could not verify, and why

- **Real `uvx matra` against a published 0.2.0.** The version is unpublished, and
  I do not publish. Approximated with `uv tool install <local wheel>`, which
  works (0.049 s install, `matra --version` correct). B2's conclusion about
  cp312-only wheels is drawn from `publish-pypi.yml:111` plus the absence of
  abi3, corroborated by observing uv build 0.1.0 from sdist on this machine.
- **Linux and Intel macOS wheels.** Only arm64 macOS available here.
- **Model hash-mismatch and re-download paths** (`errors.md:31`,
  `semantic-clusters.md:41`). Would require corrupting a cached model to force;
  I chose not to fabricate that state.
- **`pyo3_runtime.PanicException` on cross-thread access** (`python.md:40`). Not
  triggered, it is a documented crash path and I did not want a hung process in
  a sandbox mid-run.
- **The `## References` truncation rule** (`cli.md:55`, `rust.md:163`). Read but
  not exercised.
- **Load/scale behaviour.** Everything ran on ~1.6 KB documents plus one 9.4 MB
  file used solely to trip the cap.

## What I decline to claim

- That the keyphrase or summary *quality* is good or bad. H4 and C5 report how
  the output reads to a first-time user, not whether RAKE, YAKE, TF-IDF or
  TextRank are correctly implemented. I did not verify the algorithms.
- That B2 will manifest for every user. It depends on their default interpreter.
  A user on 3.12 gets the fast path the docs describe.
- That the 14-23 s download window is representative. It is one machine, one
  network, three cold runs. The *silence* is deterministic; the duration is not.
- Any judgment about whether the semantic clustering result is "right" beyond
  the fact that it separated my hand-written near-duplicates from my unrelated
  document. Three documents is not an evaluation.

---

## The first ten minutes, honestly

**0:00.** README, second line of the pitch: "If you are an agent, run `uvx matra
--skill`." I am an agent. I run it. It installs six packages and errors: `No
such option '--skill'`. My first thought is that someone squatted the name. It
takes a PyPI lookup to learn this is the owner's own older release. Bad thirty
seconds, and it evaporates on publish, but anyone reading the repo today lands
here first.

**0:02.** Install the real wheel. This part is genuinely excellent: 0.1 s to
install, 0.7 s to a working `matra --version`, zero runtime dependencies. Nothing
to complain about, and the tool has bought some goodwill.

**0:03.** `matra analyze essay.md`.

And then nothing happens.

Five seconds of blank terminal. I check the filename. Ten. I check I am in the
right directory. Fifteen, and I am hovering over Ctrl-C, genuinely unsure
whether this thing has hung on a network call or is quietly parsing something
enormous. At twenty-two seconds a five-line table appears, instantly, and it is
correct and nicely formatted and I have already lost the thread of what I was
doing. I *knew* a download was coming, because I had read the installation page.
A user who typed the README's line has been told the section is called "No
setup" and that there are no flags to worry about; they have been given no
reason to expect a pause at all.

The goodwill from 0:02 is spent, and it was spent on a progress line that does
not exist.

**0:05.** Second run: 1.06 s. Everything is fast forever after. `matra config
show` prints every resolved path with its origin, and the paths are real, I
checked. This is the moment the tool earns the trust back, and it is also the
first time anything has told me what machine state matra is using.

**0:06-0:10.** Working through the CLI guide is a pleasure. Every command runs.
`--sections`, `--json`, both `jq` one-liners, stdin, completions. The errors are
better than the docs promise, `matra: corpus is a directory; pass a file.
Directory analysis is on the roadmap.` is a genuinely good error message, and
`-r` without `--skill` tells you exactly which flag to add.

Two things make me squint rather than stop. `matra summarize` gives me three
sentences all scored `0.357`, identical to three decimals, so the ranker looks
broken until I check `--json` and find they differ in the fourth. And `matra
keyphrases` on a document about database indexes hands me "cheapest performance
work available" and never once says "index". I believe the docs when they
explain RAKE. I still would not put that output in front of a colleague without
apologising for it first.

**Where I would have stopped if I were less patient:** at minute four, staring
at a blank terminal. Everything after that point is good. The product is better
than its first ninety seconds.
