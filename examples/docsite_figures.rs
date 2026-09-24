//! Generate the docsite's figure data (EP-0012, M3).
//!
//! Runs the released pipeline, through the public surface only (`config`,
//! `Engine`, `Ingest`), over every input in `site/inputs/` and writes one JSON
//! file per figure: `site/src/lib/figures/<input>/<figure>.json`. The site
//! reads those files at build time; the browser never recomputes matra's
//! output, so there is no second implementation to drift from this one.
//!
//! The output is deterministic, so a regeneration diffs clean unless matra's
//! output changed: keys are sorted, the parse carries no floating-point
//! values, there are no timestamps, and each file records the matra version
//! and the UDPipe model (file name and SHA-256) that produced it.
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
use matra::domain::{Document, Format};
use matra::{Engine, Ingest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Turns one analysis into one figure's data.
type FigureData = fn(&Document) -> Value;

/// The figure kinds this generator writes, each a function of one analysis.
const FIGURES: &[(&str, FigureData)] = &[("parse", parse_figure)];

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
                "data": build(&doc),
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
fn parse_figure(doc: &Document) -> Value {
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
    json!({ "sentences": sentences })
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
