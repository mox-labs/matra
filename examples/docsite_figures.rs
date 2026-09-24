//! Generate the docsite's figure data (EP-0012).
//!
//! Runs the released pipeline, through the public surface only (`config`,
//! `Engine`, `Ingest`, `extraction`, `embed_and_cluster`, and
//! `embed::model2vec::Model2Vec` for the embedder it takes), over every input in
//! `site/inputs/` and writes one JSON file per figure:
//! `site/src/lib/figures/<input>/<figure>.json`. The site reads those files at
//! build time; the browser never recomputes matra's output, so there is no
//! second implementation to drift from this one.
//!
//! The output is deterministic, so a regeneration diffs clean unless matra's
//! output changed: keys are sorted, every floating-point value is rounded to
//! four decimal places, ties are broken by text rather than by hash order,
//! there are no timestamps, and each file records the matra version and the
//! models (name and SHA-256) that produced it. Figure parameters (the summary
//! length, the default cluster threshold) are matra's shipped defaults, never
//! a contributor's own config file.
//!
//! Figures, one file each per input:
//!
//!   parse        every sentence's tokens: id, text, lemma, pos, head, dep
//!   primitives   every sentence's structural primitive fields, as matra
//!                serialises them
//!   metrics      every paragraph's measures, and the document's
//!   keyphrases   RAKE and YAKE rankings of the same text
//!   textrank     every sentence's TextRank score, and the summary it picks
//!   clusters     semantic clusters at each threshold of a fixed grid
//!                (needs the `model2vec` feature and the embedding model)
//!   pipeline     one document at each stage: ingested, annotated, composed
//!
//! An input's sidecar may name the figures it gets (`figures = [...]`);
//! without that it gets the first five. `format = "markdown"` in the sidecar
//! reads the input as Markdown; plain text otherwise.
//!
//! The models are provisioned the way every caller's are: the configured
//! model directory (or `MATRA_MODEL_DIR`), downloaded on first use and
//! verified against the SHA-256 compiled into matra. A model that cannot be
//! had is an error, never a skip.
//!
//! Run from anywhere in the repository:
//!
//!   cargo run --features model2vec --example docsite_figures
//!   cargo run --features model2vec --example docsite_figures -- --out DIR
//!
//! The docsite floor's figures-current gate writes to a temporary directory
//! and diffs it against the committed files.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use matra::config::Config;
use matra::domain::{Corpus, CorpusEntry, Document, Format, Keyphrase, Sentence};
use matra::extraction::{rake_keyphrases, textrank_summarize, yake_keyphrases};
use matra::{Engine, Ingest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[cfg(feature = "model2vec")]
use matra::embed::model2vec::Model2Vec;

/// One input, analysed, with what its figures may need.
struct Input<'a> {
    text: &'a str,
    format: Format,
    doc: &'a Document,
    engine: &'a Engine,
    /// matra's shipped defaults, for figure parameters.
    shipped: &'a Config,
    #[cfg(feature = "model2vec")]
    embedder: Option<&'a Model2Vec>,
}

/// Turns one input into one figure's data.
type FigureData = fn(&Input) -> Result<Value, Box<dyn Error>>;

/// Every figure kind this generator writes.
const FIGURES: &[(&str, FigureData)] = &[
    ("parse", parse_figure),
    ("primitives", primitives_figure),
    ("metrics", metrics_figure),
    ("keyphrases", keyphrases_figure),
    ("textrank", textrank_figure),
    ("clusters", clusters_figure),
    ("pipeline", pipeline_figure),
];

/// What an input gets when its sidecar names no figures.
const DEFAULT_FIGURES: &[&str] = &["parse", "primitives", "metrics", "keyphrases", "textrank"];

/// How many phrases each ranking shows.
const KEYPHRASES_SHOWN: usize = 12;

/// The cluster thresholds: 0.50 to 0.95 in steps of 0.05, as twentieths so
/// each is the nearest `f32` to its decimal.
#[cfg(feature = "model2vec")]
const CLUSTER_GRID: std::ops::RangeInclusive<u8> = 10..=19;

/// A sidecar: its string fields, published with the figure, and the figures
/// and format it asks for.
struct Sidecar {
    fields: BTreeMap<String, String>,
    figures: Vec<String>,
    format: Format,
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let inputs = root.join("site/inputs");
    let out = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => root.join("site/src/lib/figures"),
        [flag, dir] if flag == "--out" => PathBuf::from(dir),
        other => return Err(format!("usage: docsite_figures [--out DIR], got {other:?}").into()),
    };

    let mut names: Vec<String> = fs::read_dir(&inputs)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            name.strip_suffix(".txt").map(str::to_owned)
        })
        .collect();
    names.sort();
    if names.is_empty() {
        return Err(format!("no inputs in {}", inputs.display()).into());
    }
    let sidecars: Vec<Sidecar> = names
        .iter()
        .map(|n| read_source(&inputs, n))
        .collect::<Result<_, _>>()?;

    // Where models live comes from the caller's configuration; what the
    // figures show is fixed by matra's shipped defaults.
    let cfg = Config::resolve()?;
    let shipped = Config::from_sources(|k| std::env::var(k).ok(), None)?;
    let notice = |n: &matra::domain::ProvisionNotice| {
        eprintln!(
            "docsite_figures: fetching {} ({} bytes) into {}",
            n.artifact,
            n.bytes,
            n.destination.display()
        );
    };
    let engine = Engine::from_config_with_notice(&cfg, notice)?;
    // The engine has loaded this file and verified it against the pinned
    // digest. Hashing it again names the model in the output without
    // reaching into matra's internals.
    let model_path = cfg
        .model_dir()
        .join(format!("{}.udpipe", cfg.udpipe_model()));
    let model_sha256 = sha256_hex(&fs::read(&model_path).map_err(|e| {
        format!(
            "cannot read the loaded model at {}: {e}",
            model_path.display()
        )
    })?);
    let generator = json!({
        "matra": env!("CARGO_PKG_VERSION"),
        "udpipe_model": { "name": cfg.udpipe_model(), "sha256": model_sha256 },
    });

    let wants_clusters = sidecars
        .iter()
        .any(|s| s.figures.iter().any(|f| f == "clusters"));
    #[cfg(feature = "model2vec")]
    let embedder = if wants_clusters {
        Some(Model2Vec::from_config(&cfg)?)
    } else {
        None
    };
    #[cfg(not(feature = "model2vec"))]
    if wants_clusters {
        return Err("an input asks for the clusters figure: run with --features model2vec".into());
    }

    for (name, sidecar) in names.iter().zip(&sidecars) {
        let text = fs::read_to_string(inputs.join(format!("{name}.txt")))?;
        let doc = engine
            .analyze(Ingest::text(text.clone(), sidecar.format.clone()))
            .next()
            .ok_or("the pipeline returned no document")?
            .map_err(|e| format!("{name}: {}", e.error))?
            .analysis;
        let input = Input {
            text: &text,
            format: sidecar.format.clone(),
            doc: &doc,
            engine: &engine,
            shipped: &shipped,
            #[cfg(feature = "model2vec")]
            embedder: embedder.as_ref(),
        };

        for figure in &sidecar.figures {
            let build = FIGURES
                .iter()
                .find(|(f, _)| f == figure)
                .map(|(_, b)| *b)
                .ok_or_else(|| format!("{name}: no figure called {figure:?}"))?;
            // Only the clusters figure adds to it, and only with model2vec built in.
            #[cfg_attr(not(feature = "model2vec"), allow(unused_mut))]
            let mut gen_block = generator.clone();
            #[cfg(feature = "model2vec")]
            if figure == "clusters"
                && let Some(m) = &embedder
            {
                gen_block["embedding_model"] =
                    json!({ "name": cfg.embedding_model(), "sha256": m.model_hash() });
            }
            let value = json!({
                "figure": figure,
                "input": name,
                "source": sidecar.fields,
                "generator": gen_block,
                "data": build(&input).map_err(|e| format!("{name}: {figure}: {e}"))?,
            });
            let dir = out.join(name);
            fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{figure}.json"));
            let mut body = serde_json::to_string_pretty(&value)?;
            body.push('\n');
            fs::write(&path, body)?;
            eprintln!("docsite_figures: wrote {}", path.display());
        }
    }
    Ok(())
}

/// The dependency parse: every sentence, in document order, with the CoNLL-U
/// columns the figure and its twin table show, under matra's own field names.
fn parse_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    let doc = input.doc;
    let mut sentences = Vec::new();
    for (p, paragraph) in doc.sections.iter().flat_map(|s| &s.paragraphs).enumerate() {
        for sentence in &paragraph.sentences {
            let tokens: Vec<Value> = sentence
                .tokens
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "text": t.text,
                        "lemma": t.lemma,
                        "pos": t.pos,
                        "head": t.head,
                        "dep": t.dep,
                    })
                })
                .collect();
            sentences.push(json!({
                "paragraph": p + 1,
                "text": sentence.text,
                "tokens": tokens,
            }));
        }
    }
    Ok(json!({ "sentences": sentences }))
}

/// The structural primitives: every sentence with the fields matra reads off
/// its dependency tree, serialised as matra serialises them, and the tokens
/// they point into.
fn primitives_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    let doc = input.doc;
    let mut sentences = Vec::new();
    for (p, paragraph) in doc.sections.iter().flat_map(|s| &s.paragraphs).enumerate() {
        for s in &paragraph.sentences {
            let tokens: Vec<Value> = s
                .tokens
                .iter()
                .map(|t| json!({ "id": t.id, "text": t.text }))
                .collect();
            let root = s.tokens.iter().find(|t| t.head == 0).map(|t| t.id);
            sentences.push(json!({
                "paragraph": p + 1,
                "text": s.text,
                "tokens": tokens,
                "root_id": root,
                "negations": serde_json::to_value(&s.negations)?,
                "modals": serde_json::to_value(&s.modals)?,
                "bare_assertion": s.bare_assertion,
                "reportings": serde_json::to_value(&s.reportings)?,
                "root_adverbials": serde_json::to_value(&s.root_adverbials)?,
                "hearst_pairs": serde_json::to_value(&s.hearst_pairs)?,
            }));
        }
    }
    Ok(json!({ "sentences": sentences }))
}

/// The measures: every paragraph's three, and the document-level values
/// matra computes. Readability's document value is `Corpus::mean_readability`
/// over a corpus of this one document; matra has no document-level lexical
/// density or compression ratio, so none is written.
fn metrics_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    let doc = input.doc;
    let mut paragraphs = Vec::new();
    for (i, p) in doc.sections.iter().flat_map(|s| &s.paragraphs).enumerate() {
        let opening: Vec<&str> = p.text.split_whitespace().take(8).collect();
        paragraphs.push(json!({
            "index": i + 1,
            "opening": opening.join(" "),
            "words": p.word_count(),
            "in_blockquote": p.in_blockquote,
            "readability_grade": p.readability_grade.map(round4),
            "lexical_density": p.lexical_density.map(round4),
            "compression_ratio": p.compression_ratio.map(round4),
        }));
    }
    let measured = doc
        .sections
        .iter()
        .flat_map(|s| &s.paragraphs)
        .any(|p| p.readability_grade.is_some());
    let corpus = Corpus::new(vec![CorpusEntry::new(None, doc.clone())]);
    Ok(json!({
        "paragraphs": paragraphs,
        "document": {
            "mean_readability": measured.then(|| round4(corpus.mean_readability())),
            "vocabulary_ttr": doc.vocabulary_ttr.map(round4),
            "nominalization_ratio": doc.nominalization_ratio.map(round4),
            "passive_ratio": doc.passive_ratio.map(round4),
        },
    }))
}

/// RAKE and YAKE over the same sentences. Both of matra's scores run higher
/// is more relevant (matra's YAKE reports the reciprocal of the published
/// score), and neither is comparable with the other, so the figure compares
/// ranks. Every phrase is requested, then ranked here on the rounded score
/// with ties sharing a rank and broken by the phrase text, so the output does
/// not depend on hash order.
fn keyphrases_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    let doc = input.doc;
    let sentences: Vec<Sentence> = doc.sentences().cloned().collect();
    let rake = ranked(rake_keyphrases(&sentences, usize::MAX)?);
    let yake = ranked(yake_keyphrases(&sentences, usize::MAX)?);
    let top = |list: &[(String, f64, usize)]| -> Vec<Value> {
        list.iter()
            .take(KEYPHRASES_SHOWN)
            .map(|(phrase, score, rank)| json!({ "phrase": phrase, "score": score, "rank": rank }))
            .collect()
    };
    let lookup = |list: &[(String, f64, usize)], phrase: &str| -> Value {
        list.iter().find(|(p, _, _)| p == phrase).map_or(
            Value::Null,
            |(_, score, rank)| json!({ "score": score, "rank": rank }),
        )
    };
    let mut shown: Vec<String> = rake
        .iter()
        .take(KEYPHRASES_SHOWN)
        .chain(yake.iter().take(KEYPHRASES_SHOWN))
        .map(|(p, _, _)| p.clone())
        .collect();
    shown.sort();
    shown.dedup();
    let phrases: Vec<Value> = shown
        .iter()
        .map(|p| json!({ "phrase": p, "rake": lookup(&rake, p), "yake": lookup(&yake, p) }))
        .collect();
    Ok(json!({
        "shown": KEYPHRASES_SHOWN,
        "total": { "rake": rake.len(), "yake": yake.len() },
        "rake": top(&rake),
        "yake": top(&yake),
        "phrases": phrases,
    }))
}

/// Sort by rounded score, highest first, then by phrase; rank competition
/// style, so equal scores share a rank.
fn ranked(list: Vec<Keyphrase>) -> Vec<(String, f64, usize)> {
    let mut rows: Vec<(String, f64)> = list
        .into_iter()
        .map(|k| (k.phrase, round4(k.score)))
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = Vec::with_capacity(rows.len());
    for (i, (phrase, score)) in rows.iter().enumerate() {
        let rank = match out.last() {
            Some((_, prev, r)) if *prev == *score => *r,
            _ => i + 1,
        };
        out.push((phrase.clone(), *score, rank));
    }
    out
}

/// Four decimal places: enough for every figure, and few enough that the last
/// bits of platform floating point cannot reach the output.
fn round4(x: f64) -> f64 {
    let r = (x * 10_000.0).round() / 10_000.0;
    if r == 0.0 { 0.0 } else { r }
}

/// TextRank: every sentence's score, in document order, and the summary
/// matra picks at its shipped length. `textrank_summarize` asked for every
/// sentence returns every score; asked for the shipped `n`, it returns the
/// summary. The similarity edges between sentences are internal to matra and
/// not written: drawing them here would be a second implementation.
fn textrank_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    let doc = input.doc;
    let sentences: Vec<Sentence> = doc.sentences().cloned().collect();
    let n = input.shipped.summarize_n();
    let all = textrank_summarize(&sentences, sentences.len())?;
    let picked: Vec<usize> = textrank_summarize(&sentences, n)?
        .into_iter()
        .map(|s| s.position)
        .collect();
    // The paragraph each sentence came from, in the order doc.sentences()
    // yields them.
    let mut paragraph_of = Vec::new();
    for (p, paragraph) in doc.sections.iter().flat_map(|s| &s.paragraphs).enumerate() {
        for _ in &paragraph.sentences {
            paragraph_of.push(p + 1);
        }
    }
    let rows: Vec<Value> = all
        .iter()
        .map(|s| {
            json!({
                "position": s.position,
                "paragraph": paragraph_of.get(s.position).copied(),
                "text": s.text,
                "score": round4(s.score),
                "summary": picked.contains(&s.position),
            })
        })
        .collect();
    Ok(json!({ "n": n, "sentences": rows }))
}

/// Semantic clusters of the input's sentences at every threshold of the grid,
/// each from `embed_and_cluster`, the call a caller makes. The figure's
/// control snaps to these values; the browser never recomputes a cluster.
#[cfg(feature = "model2vec")]
fn clusters_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    let embedder = input
        .embedder
        .ok_or("the clusters figure needs the embedding model")?;
    let default = input.shipped.semantic_threshold();
    let mut grid = Vec::new();
    let mut default_on_grid = false;
    for step in CLUSTER_GRID {
        let threshold = f32::from(step) / 20.0;
        if (threshold - default).abs() < 1e-6 {
            default_on_grid = true;
        }
        let result = matra::embed_and_cluster(input.doc, embedder, threshold)?;
        let clusters: Vec<Value> = result
            .clusters
            .iter()
            .map(|c| {
                let edges: Vec<Value> = c
                    .edges
                    .iter()
                    .map(|e| json!({ "a": e.a, "b": e.b, "score": round4(f64::from(e.score)) }))
                    .collect();
                json!({ "members": c.members, "edges": edges })
            })
            .collect();
        grid.push(json!({ "threshold": round4(f64::from(threshold)), "clusters": clusters }));
    }
    if !default_on_grid {
        return Err(format!("the shipped threshold {default} is not on the figure's grid").into());
    }
    let sentences: Vec<Value> = input
        .doc
        .sentences()
        .enumerate()
        .map(|(i, s)| json!({ "index": i, "text": s.text }))
        .collect();
    Ok(json!({
        "default_threshold": round4(f64::from(default)),
        "grid": grid,
        "sentences": sentences,
    }))
}

#[cfg(not(feature = "model2vec"))]
fn clusters_figure(_: &Input) -> Result<Value, Box<dyn Error>> {
    Err("the clusters figure needs --features model2vec".into())
}

/// One document through the pipeline's stages, each the public call: the
/// `RawDocument` `Ingest::text` yields, then `Engine::annotate` (decompose and parse),
/// then `Engine::compose` (the measures).
fn pipeline_figure(input: &Input) -> Result<Value, Box<dyn Error>> {
    // The figure names `Ingest::text` as the first stage, so that is the
    // call: a stream of one, whose only item is the document.
    let raw = Ingest::text(input.text.to_owned(), input.format.clone())
        .next()
        .ok_or("Ingest::text yielded no document")??;
    let annotated = input.engine.annotate(&raw)?;
    let mut composed = annotated.clone();
    input.engine.compose(&mut composed);
    Ok(json!({
        "raw": {
            "format": format_name(&input.format),
            "bytes": input.text.len(),
            "text": input.text,
        },
        "annotated": stage(&annotated),
        "composed": stage(&composed),
    }))
}

/// A document's tree, paragraph by paragraph, with what each stage fills in.
fn stage(doc: &Document) -> Value {
    let mut index = 0;
    let sections: Vec<Value> = doc
        .sections
        .iter()
        .map(|s| {
            let paragraphs: Vec<Value> = s
                .paragraphs
                .iter()
                .map(|p| {
                    index += 1;
                    let opening: Vec<&str> = p.text.split_whitespace().take(8).collect();
                    json!({
                        "index": index,
                        "opening": opening.join(" "),
                        "in_blockquote": p.in_blockquote,
                        "sentences": p.sentences.len(),
                        "tokens": p.sentences.iter().map(|x| x.tokens.len()).sum::<usize>(),
                        "readability_grade": p.readability_grade.map(round4),
                        "lexical_density": p.lexical_density.map(round4),
                        "compression_ratio": p.compression_ratio.map(round4),
                    })
                })
                .collect();
            json!({ "heading": s.heading, "level": s.level, "paragraphs": paragraphs })
        })
        .collect();
    json!({
        "sections": sections,
        "document": {
            "vocabulary_ttr": doc.vocabulary_ttr.map(round4),
            "nominalization_ratio": doc.nominalization_ratio.map(round4),
            "passive_ratio": doc.passive_ratio.map(round4),
        },
    })
}

fn format_name(format: &Format) -> &'static str {
    match format {
        Format::Markdown => "markdown",
        _ => "plaintext",
    }
}

/// The input's sidecar: where the text came from and under what licence,
/// and optionally which figures it gets and how it is read. Every input has
/// one; the docsite floor checks that too.
fn read_source(dir: &Path, name: &str) -> Result<Sidecar, Box<dyn Error>> {
    let path = dir.join(format!("{name}.source.toml"));
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("{name}.txt has no readable sidecar {}: {e}", path.display()))?;
    let table: toml::Table =
        toml::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut fields = BTreeMap::new();
    let mut figures: Vec<String> = DEFAULT_FIGURES.iter().map(|s| (*s).to_owned()).collect();
    let mut format = Format::PlainText;
    for (key, value) in table {
        match (key.as_str(), value) {
            ("figures", toml::Value::Array(list)) => {
                figures = list
                    .into_iter()
                    .map(|v| {
                        v.as_str().map(str::to_owned).ok_or_else(|| {
                            format!(
                                "{}: figures lists names, found {}",
                                path.display(),
                                v.type_str()
                            )
                        })
                    })
                    .collect::<Result<_, _>>()?;
                if figures.is_empty() {
                    return Err(format!("{}: figures is empty", path.display()).into());
                }
            }
            ("format", toml::Value::String(f)) => {
                format = match f.as_str() {
                    "markdown" => Format::Markdown,
                    "plaintext" => Format::PlainText,
                    other => {
                        return Err(format!("{}: unknown format {other:?}", path.display()).into());
                    }
                };
            }
            // The two keys the generator reads are never provenance: a
            // mistyped one fails here, not as a missing file at site build.
            (k @ ("figures" | "format"), other) => {
                let expected = if k == "figures" {
                    "a list of figure names"
                } else {
                    "a string"
                };
                return Err(format!(
                    "{}: {k} is {other}, expected {expected}",
                    path.display(),
                    other = other.type_str()
                )
                .into());
            }
            (_, toml::Value::String(s)) => {
                fields.insert(key, s);
            }
            (_, other) => {
                return Err(format!(
                    "{}: {key} is {other}, expected a string",
                    path.display(),
                    other = other.type_str()
                )
                .into());
            }
        }
    }
    for key in ["source", "licence"] {
        if fields.get(key).is_none_or(|v| v.trim().is_empty()) {
            return Err(format!("{} has no {key}", path.display()).into());
        }
    }
    Ok(Sidecar {
        fields,
        figures,
        format,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
