//! A `Document` is data: it arrives from `Engine::analyze`, from JSON a
//! caller saved and read back, or from the public constructors, and matra
//! cannot tell those apart. So no public outcome may panic or loop on a
//! document matra did not make, whatever its tokens, spans and derived
//! fields claim.
//!
//! Each case below starts from a real parse (the outcome fixture's
//! document, `spec/tests/outcomes/outcomes.json`), breaks one thing the
//! parser would never produce, and reads the result back through
//! `Deserialize`, the route a saved document takes. Every outcome then runs
//! over it on its own thread with a deadline, so a panic and a loop both
//! fail the test by name rather than aborting or hanging it. What an
//! outcome returns is not pinned here: an error is an acceptable answer to
//! a document that contradicts itself, and so is a result that carries the
//! contradiction through. Only a panic or a hang is not.
//!
//! Runs with no parser in the build (`cargo test --no-default-features`),
//! since none of it parses.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use matra::domain::{Document, Embedding, Identity, Result, Sentence};
use matra::embed::Embedder;
use matra::{KeyphraseMethod, SummaryMethod};
use serde_json::{Value, json};

/// Long enough for every outcome over the fixture on a slow CI runner,
/// short enough that a loop fails the run rather than stalling it.
const DEADLINE: Duration = Duration::from_secs(60);

/// The fixture's parsed document, as JSON to break.
fn base() -> Value {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/tests/outcomes/outcomes.json");
    let raw = std::fs::read_to_string(&path).expect("read the outcome fixture");
    let fixture: Value = serde_json::from_str(&raw).expect("the fixture is JSON");
    fixture["document"].clone()
}

/// Every sentence in a document's JSON.
fn sentences(doc: &mut Value) -> Vec<&mut Value> {
    let mut out = Vec::new();
    for section in doc["sections"].as_array_mut().into_iter().flatten() {
        for paragraph in section["paragraphs"].as_array_mut().into_iter().flatten() {
            for sentence in paragraph["sentences"].as_array_mut().into_iter().flatten() {
                out.push(sentence);
            }
        }
    }
    out
}

/// Every token in a document's JSON.
fn tokens(doc: &mut Value) -> Vec<&mut Value> {
    let mut out = Vec::new();
    for sentence in sentences(doc) {
        for token in sentence["tokens"].as_array_mut().into_iter().flatten() {
            out.push(token);
        }
    }
    out
}

/// Every paragraph in a document's JSON.
fn paragraphs(doc: &mut Value) -> Vec<&mut Value> {
    let mut out = Vec::new();
    for section in doc["sections"].as_array_mut().into_iter().flatten() {
        for paragraph in section["paragraphs"].as_array_mut().into_iter().flatten() {
            out.push(paragraph);
        }
    }
    out
}

const BIG: u64 = u64::MAX;

/// One broken document: what is wrong with it, and the edit that makes it
/// so.
struct Case {
    name: &'static str,
    edit: fn(&mut Value),
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "token spans past the end of the text",
            edit: |doc| {
                for t in tokens(doc) {
                    t["span"] = json!({"byte_start": BIG - 1, "byte_end": BIG});
                }
            },
        },
        Case {
            name: "inverted token spans",
            edit: |doc| {
                for t in tokens(doc) {
                    t["span"] = json!({"byte_start": 40, "byte_end": 3});
                }
            },
        },
        Case {
            name: "inverted sentence and paragraph spans",
            edit: |doc| {
                for s in sentences(doc) {
                    s["span"] = json!({"byte_start": BIG, "byte_end": 0});
                }
                for p in paragraphs(doc) {
                    p["span"] = json!({"byte_start": BIG, "byte_end": 0});
                }
            },
        },
        Case {
            name: "heads pointing at no token",
            edit: |doc| {
                for t in tokens(doc) {
                    t["head"] = json!(BIG);
                }
            },
        },
        Case {
            name: "a two-token head cycle and no root",
            edit: |doc| {
                for s in sentences(doc) {
                    for t in s["tokens"].as_array_mut().into_iter().flatten() {
                        let id = t["id"].as_u64().unwrap_or(1);
                        t["head"] = json!(if id % 2 == 0 { id - 1 } else { id + 1 });
                    }
                }
            },
        },
        Case {
            name: "every token its own head",
            edit: |doc| {
                for t in tokens(doc) {
                    t["head"] = t["id"].clone();
                }
            },
        },
        Case {
            name: "duplicate token ids, all zero",
            edit: |doc| {
                for t in tokens(doc) {
                    t["id"] = json!(0);
                    t["head"] = json!(0);
                }
            },
        },
        Case {
            name: "tokens in reverse order",
            edit: |doc| {
                for s in sentences(doc) {
                    if let Some(list) = s["tokens"].as_array_mut() {
                        list.reverse();
                    }
                }
            },
        },
        Case {
            name: "sentences with no tokens",
            edit: |doc| {
                for s in sentences(doc) {
                    s["tokens"] = json!([]);
                }
            },
        },
        Case {
            name: "empty sentences, text and tokens",
            edit: |doc| {
                for s in sentences(doc) {
                    s["tokens"] = json!([]);
                    s["text"] = json!("");
                }
            },
        },
        Case {
            name: "paragraphs with no sentences",
            edit: |doc| {
                for p in paragraphs(doc) {
                    p["sentences"] = json!([]);
                }
            },
        },
        Case {
            name: "no sections",
            edit: |doc| doc["sections"] = json!([]),
        },
        Case {
            name: "sections with no paragraphs",
            edit: |doc| {
                for section in doc["sections"].as_array_mut().into_iter().flatten() {
                    section["paragraphs"] = json!([]);
                }
            },
        },
        Case {
            name: "sentence text that is not its tokens",
            edit: |doc| {
                for s in sentences(doc) {
                    s["text"] = json!("\u{1F980} nothing here matches \u{0301}");
                }
            },
        },
        Case {
            name: "paragraph text that is not its sentences",
            edit: |doc| {
                for p in paragraphs(doc) {
                    p["text"] = json!("");
                }
            },
        },
        Case {
            name: "token text and lemmas empty or multibyte",
            edit: |doc| {
                for (i, t) in tokens(doc).into_iter().enumerate() {
                    let (text, lemma) = if i % 2 == 0 {
                        ("", "")
                    } else {
                        ("\u{00E9}\u{0301}", "\u{1F980}\u{1F980}")
                    };
                    t["text"] = json!(text);
                    t["lemma"] = json!(lemma);
                }
            },
        },
        Case {
            name: "every token punctuation",
            edit: |doc| {
                for t in tokens(doc) {
                    t["is_punct"] = json!(true);
                }
            },
        },
        Case {
            name: "derived counts at their maximum",
            edit: |doc| {
                for s in sentences(doc) {
                    s["word_count"] = json!(BIG);
                    s["tree_depth"] = json!(BIG);
                }
                doc["metrics"]["total_words"] = json!(BIG);
                doc["metrics"]["total_sentences"] = json!(BIG);
                doc["metrics"]["paragraph_count"] = json!(BIG);
            },
        },
        Case {
            name: "derived records naming tokens that do not exist",
            edit: |doc| {
                for s in sentences(doc) {
                    s["negations"] = json!([{"cue_id": BIG, "cue_lemma": "not", "head_id": BIG}]);
                    s["modals"] = json!([{"aux_id": BIG, "aux_lemma": "can", "head_id": 0}]);
                    s["root_mood"] = json!({"root_id": BIG, "mood": "Ind"});
                    s["hearst_pairs"] = json!([{
                        "pattern": "such_as",
                        "hypernym": {"head_id": BIG, "head_lemma": "x", "first_id": BIG, "last_id": 0},
                        "hyponym": {"head_id": 0, "head_lemma": "y", "first_id": 3, "last_id": 1}
                    }]);
                }
            },
        },
        Case {
            name: "provenance naming an input of no bytes",
            edit: |doc| {
                doc["provenance"]["source"]["bytes"] = json!(0);
                doc["provenance"]["source"]["sha256"] = json!("");
            },
        },
        Case {
            name: "one sentence repeated past the summary cap",
            edit: |doc| {
                let one = sentences(doc)
                    .first()
                    .map(|s| (**s).clone())
                    .expect("the fixture has a sentence");
                let many: Vec<Value> = std::iter::repeat_n(one, 2_001).collect();
                doc["sections"] = json!([{
                    "heading": null, "level": 0, "span": null,
                    "paragraphs": [{
                        "span": {"byte_start": 0, "byte_end": 0},
                        "text": "", "kind": "paragraph",
                        "sentences": many,
                        "metrics": {"readability_grade": null, "lexical_density": null, "compression_ratio": null}
                    }]
                }]);
            },
        },
    ]
}

/// An embedder that turns each text into a small vector from its bytes:
/// deterministic, any dimension, and the zero vector for empty text,
/// which is the input a cosine has to survive.
struct ByteEmbedder;

impl Embedder for ByteEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Embedding>> {
        Ok(texts
            .iter()
            .map(|text| {
                let mut v = vec![0.0f32; 4];
                for (i, b) in text.bytes().enumerate() {
                    v[i % 4] += f32::from(b);
                }
                Embedding(v)
            })
            .collect())
    }

    fn identity(&self) -> Identity {
        Identity::new("bytes", None, None, None)
    }
}

/// Every public outcome and every public view over `doc`, as named calls.
/// A call's result is consumed, so nothing is optimised away, and dropped:
/// what it says is not under test.
fn exercise(doc: &Document) -> Vec<&'static str> {
    let mut ran = Vec::new();
    for n in [0, 1, 3, usize::MAX] {
        let _ = matra::summarize(doc, n);
        let _ = matra::summarize_with(doc, n, SummaryMethod::TfIdf);
        let _ = matra::summarize_with(doc, n, SummaryMethod::TextRank);
        let _ = matra::keyphrases(doc, n);
        let _ = matra::keyphrases_with(doc, n, KeyphraseMethod::Rake);
        let _ = matra::keyphrases_with(doc, n, KeyphraseMethod::Yake);
    }
    ran.push("summarize, keyphrases");
    for threshold in [-1.0, 0.0, 0.5, 1.0, f64::NAN] {
        let _ = matra::clusters(doc, &ByteEmbedder, threshold);
    }
    ran.push("clusters");

    let _ = doc.paragraphs().count();
    let _ = doc.tokens().count();
    for paragraph in doc.paragraphs() {
        let _ = paragraph.word_count();
        let _ = paragraph.sentence_count();
    }
    ran.push("paragraph counts");
    for sentence in doc.sentences() {
        let _ = sentence.root_token();
        let _ = sentence.content_tokens();
        let _ = sentence.clausal_complements_in(&["say"]);
        let _ = sentence.root_adverbials_in(&["reportedly"]);
        let ids = sentence
            .tokens
            .iter()
            .flat_map(|t| [t.id, t.head])
            .chain([0, usize::MAX]);
        for id in ids {
            let _ = sentence.children_of(id);
            let _ = sentence.head_of(id);
            let _ = sentence.subtree(id);
        }
        for token in &sentence.tokens {
            let _ = token.feat("Mood");
        }
        // The public constructor reruns every detector over the tokens.
        let rebuilt = Sentence::new(sentence.text.clone(), sentence.tokens.clone());
        let _ = rebuilt.tree_depth;
    }
    ran.push("sentence views and Sentence::new");

    let _ = serde_json::to_string(doc);
    ran.push("serialize");
    ran
}

#[test]
fn no_public_outcome_panics_or_loops_on_a_hostile_document() {
    let mut failures = Vec::new();
    for case in cases() {
        let mut value = base();
        (case.edit)(&mut value);
        let doc: Document = match serde_json::from_value(value) {
            Ok(doc) => doc,
            Err(e) => {
                failures.push(format!("{}: did not deserialize: {e}", case.name));
                continue;
            }
        };
        let (tx, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let ran = exercise(&doc);
            let _ = tx.send(ran);
        });
        match rx.recv_timeout(DEADLINE) {
            Ok(_) => {
                if let Err(panic) = worker.join() {
                    failures.push(format!(
                        "{}: panicked after finishing: {panic:?}",
                        case.name
                    ));
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let message = match worker.join() {
                    Err(panic) => panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "a panic with no message".to_string()),
                    Ok(()) => "the worker ended without reporting".to_string(),
                };
                failures.push(format!("{}: panicked: {message}", case.name));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // The thread is left running; the test fails, and the
                // process ends with it.
                failures.push(format!("{}: did not finish in {DEADLINE:?}", case.name));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of the hostile documents broke an outcome:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The edits are not idle: each one changes the document it starts from.
/// A case whose edit matched nothing would pass while testing nothing.
#[test]
fn every_case_changes_the_document() {
    let original = base();
    for case in cases() {
        let mut value = original.clone();
        (case.edit)(&mut value);
        assert_ne!(value, original, "{}: the edit changed nothing", case.name);
    }
}
