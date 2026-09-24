//! Generate the docsite's figure data (EP-0012, M3).
//!
//! Runs the released pipeline, through the public surface only (`config`,
//! `Engine`, `Ingest`), over every input in `site/inputs/` and writes one JSON
//! file per figure: `site/src/lib/figures/<input>/<figure>.json`. The site
//! reads those files at build time; the browser never recomputes matra's
//! output, so there is no second implementation to drift from this one.
//!
//! The output is deterministic, so a regeneration diffs clean unless matra's
//! output changed: keys are sorted, every floating-point value is rounded to
//! four decimal places, ties are broken by text rather than by hash order,
//! there are no timestamps, and each file records the matra version and the
//! UDPipe model (file name and SHA-256) that produced it.
//!
//! Figures, one file each per input:
//!
//!   parse        every sentence's tokens: id, text, lemma, pos, head, dep
//!   primitives   every sentence's structural primitive fields, as matra
//!                serialises them
//!   metrics      every paragraph's measures, and the document's
//!   keyphrases   RAKE and YAKE rankings of the same text
//!
//! The model is provisioned the way every caller's is: the configured model
//! directory (or `MATRA_MODEL_DIR`), downloaded on first use and verified
//! against the SHA-256 compiled into matra. A model that cannot be had is an
//! error, never a skip.
//!
//! Run from anywhere in the repository:
//!
//!   cargo run --example docsite_figures                 write the figures
//!   cargo run --example docsite_figures -- --out DIR    write them elsewhere
//!
//! The docsite floor's figures-current gate writes to a temporary directory
//! and diffs it against the committed files.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use matra::config::Config;
use matra::domain::{Corpus, CorpusEntry, Document, Format, Keyphrase, Sentence};
use matra::extraction::{rake_keyphrases, yake_keyphrases};
use matra::{Engine, Ingest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Turns one analysis into one figure's data.
type FigureData = fn(&Document) -> Result<Value, Box<dyn Error>>;

/// The figure kinds this generator writes, each a function of one analysis.
const FIGURES: &[(&str, FigureData)] = &[
    ("parse", parse_figure),
    ("primitives", primitives_figure),
    ("metrics", metrics_figure),
    ("keyphrases", keyphrases_figure),
];

/// How many phrases each ranking shows.
const KEYPHRASES_SHOWN: usize = 12;

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let inputs = root.join("site/inputs");
    let out = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => root.join("site/src/lib/figures"),
        [flag, dir] if flag == "--out" => PathBuf::from(dir),
        other => return Err(format!("usage: docsite_figures [--out DIR], got {other:?}").into()),
    };

    let cfg = Config::resolve()?;
    let engine = Engine::from_config_with_notice(&cfg, |n| {
        eprintln!(
            "docsite_figures: fetching {} ({} bytes) into {}",
            n.artifact,
            n.bytes,
            n.destination.display()
        );
    })?;
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

    for name in &names {
        let text = fs::read_to_string(inputs.join(format!("{name}.txt")))?;
        let source = read_source(&inputs, name)?;
        let doc = engine
            .analyze(Ingest::text(text, Format::PlainText))
            .next()
            .ok_or("the pipeline returned no document")?
            .map_err(|e| format!("{name}: {}", e.error))?
            .analysis;

        for (figure, build) in FIGURES {
            let value = json!({
                "figure": figure,
                "input": name,
                "source": source,
                "generator": generator,
                "data": build(&doc).map_err(|e| format!("{name}: {figure}: {e}"))?,
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
fn parse_figure(doc: &Document) -> Result<Value, Box<dyn Error>> {
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
fn primitives_figure(doc: &Document) -> Result<Value, Box<dyn Error>> {
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
fn metrics_figure(doc: &Document) -> Result<Value, Box<dyn Error>> {
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
fn keyphrases_figure(doc: &Document) -> Result<Value, Box<dyn Error>> {
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

/// The input's sidecar: where the text came from and under what licence.
/// Every input has one; the docsite floor checks that too.
fn read_source(dir: &Path, name: &str) -> Result<BTreeMap<String, String>, Box<dyn Error>> {
    let path = dir.join(format!("{name}.source.toml"));
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("{name}.txt has no readable sidecar {}: {e}", path.display()))?;
    let table: BTreeMap<String, String> =
        toml::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
    for key in ["source", "licence"] {
        if table.get(key).is_none_or(|v| v.trim().is_empty()) {
            return Err(format!("{} has no {key}", path.display()).into());
        }
    }
    Ok(table)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
