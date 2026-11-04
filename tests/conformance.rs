//! Conformance tests: the Rust crust against the shared spec.
//!
//! Every fixture in `spec/tests/` is run through the Rust API and checked
//! against the same expectations the Python crust checks. A difference
//! between crusts is a binding defect, not a behaviour difference.
//!
//! Requires the UDPipe model, so these are `#[ignore]` by default:
//!
//!     cargo test --test conformance -- --ignored

#![cfg(feature = "udpipe")]

mod document_schema;

use std::fs;
use std::path::PathBuf;

use matra::domain::{Document, Format, Span};
use matra::{Engine, Input, KeyphraseMethod, SummaryMethod};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

const TOLERANCE: f64 = 1e-6;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    input: String,
    /// In the format vocabulary every surface shares (`plain_text`,
    /// `markdown`), so it deserializes straight into the domain type.
    format: Format,
    expect: Expect,
}

#[derive(Deserialize)]
struct Expect {
    metrics: ExpectedMetrics,
    /// The section tree's headings, when the fixture pins them.
    sections: Option<Vec<ExpectedSection>>,
    /// Every paragraph's kind and span, when the fixture pins them.
    paragraphs: Option<Vec<ExpectedParagraph>>,
    /// What the reader skipped, when the fixture pins it; an empty list
    /// pins "nothing".
    skipped: Option<Vec<ExpectedSkipped>>,
    sentences: Vec<ExpectedSentence>,
}

#[derive(Deserialize)]
struct ExpectedSection {
    heading: Option<String>,
    level: usize,
    span: Option<Span>,
}

#[derive(Deserialize)]
struct ExpectedParagraph {
    kind: matra::domain::ParagraphKind,
    span: Span,
}

#[derive(Deserialize)]
struct ExpectedSkipped {
    kind: matra::domain::SkippedKind,
    span: Span,
}

#[derive(Deserialize)]
struct ExpectedMetrics {
    total_sentences: usize,
    total_words: usize,
    paragraph_count: usize,
    vocabulary_ttr: Option<f64>,
    nominalization_ratio: Option<f64>,
    passive_ratio: Option<f64>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct ExpectedRootMood {
    root_id: usize,
    mood: String,
}

/// A key that may be absent (the fixture does not pin it) or present
/// and null (the fixture pins "none"): `Option<Option<T>>`, which serde
/// alone collapses.
fn pinned<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Deserialize)]
struct ExpectedSentence {
    /// Where the sentence lies in the input, when the fixture pins it.
    span: Option<Span>,
    text: String,
    token_count: usize,
    /// Expected negation cues, when the fixture pins them. `None` means
    /// the fixture predates the field and the check is skipped.
    negations: Option<Vec<ExpectedNegation>>,
    /// Expected modal auxiliaries, when the fixture pins them. Same
    /// convention as `negations`.
    modals: Option<Vec<ExpectedModal>>,
    /// Expected root mood, when the fixture pins it; a pinned `null`
    /// means the root clause carries no mood.
    #[serde(default, deserialize_with = "pinned")]
    root_mood: Option<Option<ExpectedRootMood>>,
    /// Expected clausal complements, when the fixture pins them. Same
    /// convention as `negations`.
    clausal_complements: Option<Vec<ExpectedClausalComplement>>,
    /// Expected root-attached adverbials, when the fixture pins them.
    /// Same convention as `negations`.
    root_adverbials: Option<Vec<ExpectedRootAdverbial>>,
    /// Expected Hearst pairs, when the fixture pins them. Same
    /// convention as `negations`; an empty list pins "nothing fires".
    hearst_pairs: Option<Vec<ExpectedHearstPair>>,
    tokens: Vec<ExpectedToken>,
}

#[derive(Deserialize)]
struct ExpectedHearstPair {
    pattern: matra::domain::HearstPattern,
    hypernym: ExpectedHearstSpan,
    hyponym: ExpectedHearstSpan,
}

#[derive(Deserialize)]
struct ExpectedHearstSpan {
    head_id: usize,
    head_lemma: String,
    first_id: usize,
    last_id: usize,
}

#[derive(Deserialize)]
struct ExpectedClausalComplement {
    verb_id: usize,
    verb_lemma: String,
    ccomp_id: usize,
    subject_id: Option<usize>,
    subject_lemma: Option<String>,
}

#[derive(Deserialize)]
struct ExpectedRootAdverbial {
    adv_id: usize,
    adv_lemma: String,
}

#[derive(Deserialize)]
struct ExpectedNegation {
    cue_id: usize,
    cue_lemma: String,
    head_id: usize,
}

#[derive(Deserialize)]
struct ExpectedModal {
    aux_id: usize,
    aux_lemma: String,
    head_id: usize,
}

#[derive(Deserialize)]
struct ExpectedToken {
    id: usize,
    /// Where the token lies in the input, when the fixture pins it.
    span: Option<Span>,
    text: String,
    lemma: String,
    pos: String,
    head: usize,
    dep: String,
}

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec")
        .join("tests")
}

fn load_fixtures() -> Vec<Fixture> {
    let dir = spec_dir();
    let mut fixtures: Vec<Fixture> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? != "json" {
                return None;
            }
            let raw = fs::read_to_string(&path).ok()?;
            Some(
                serde_json::from_str(&raw)
                    .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display())),
            )
        })
        .collect();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(
        !fixtures.is_empty(),
        "no fixtures found in {}",
        dir.display()
    );
    fixtures
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE
}

/// Every summary and keyphrase method over a real parse: each result
/// matches its schema, a summary sentence's span slices the input to a
/// sentence of the document, and every keyphrase occurrence lies in the
/// input.
fn check_outcomes(input: &str, doc: &Document, name: &str) {
    for method in [SummaryMethod::TfIdf, SummaryMethod::TextRank] {
        let summary = matra::summarize_with(doc, 3, method).expect("summarize");
        document_schema::assert_valid(
            "summary",
            &serde_json::to_value(&summary).expect("serialize"),
            name,
        );
        for picked in &summary.sentences {
            let sentence = doc.sentences().nth(picked.index).expect("an index");
            assert_eq!(picked.span, sentence.span, "{name}: summary span");
            assert!(picked.span.end <= input.len(), "{name}: summary span");
        }
    }
    for method in [KeyphraseMethod::Rake, KeyphraseMethod::Yake] {
        let found = matra::keyphrases_with(doc, 10, method).expect("keyphrases");
        document_schema::assert_valid(
            "keyphrases",
            &serde_json::to_value(&found).expect("serialize"),
            name,
        );
        for phrase in &found.phrases {
            for span in &phrase.occurrences {
                assert!(
                    span.start < span.end && input.is_char_boundary(span.start),
                    "{name}: {} occurs at {span:?}",
                    phrase.phrase
                );
                assert!(input.is_char_boundary(span.end), "{name}: {span:?}");
            }
        }
    }
}

/// Every span names the text it claims: a token's span slices the
/// input to the token's form, a sentence's runs from its first token to
/// its last, and both lie inside their paragraph's span.
fn check_spans(input: &str, doc: &Document, name: &str) {
    for (p, paragraph) in doc.paragraphs().enumerate() {
        let within =
            |span: Span| span.start >= paragraph.span.start && span.end <= paragraph.span.end;
        for (i, sentence) in paragraph.sentences.iter().enumerate() {
            let at = format!("{name}: paragraph {p} sentence {i}");
            assert!(within(sentence.span), "{at}: span outside its paragraph");
            if let (Some(first), Some(last)) = (sentence.tokens.first(), sentence.tokens.last()) {
                assert_eq!(
                    sentence.span,
                    Span::new(first.span.start, last.span.end),
                    "{at}: span"
                );
            }
            for token in &sentence.tokens {
                assert_eq!(
                    &input[token.span.start..token.span.end],
                    token.text,
                    "{at} token {}: span",
                    token.id
                );
            }
        }
    }
}

/// L8, the accounting law: every byte of the input that is not
/// whitespace lies in exactly one heading, paragraph or skipped span.
fn accounting(input: &str, doc: &Document) -> Result<(), String> {
    let mut owners = vec![0u32; input.len()];
    let spans = doc
        .sections
        .iter()
        .filter_map(|s| s.span)
        .chain(doc.paragraphs().map(|p| p.span))
        .chain(doc.skipped.iter().map(|s| s.span));
    for span in spans {
        for owner in &mut owners[span.start..span.end] {
            *owner += 1;
        }
    }
    match input
        .char_indices()
        .find(|&(at, c)| !c.is_whitespace() && owners[at] != 1)
    {
        Some((at, c)) => Err(format!("byte {at} ({c:?}) lies in {} spans", owners[at])),
        None => Ok(()),
    }
}

#[test]
#[ignore = "requires the UDPipe model"]
fn rust_crust_conforms_to_spec() {
    // The model directory comes from the environment, as `Engine::new`
    // resolves it for every caller: `MATRA_MODEL_DIR` names one.
    let engine = Engine::new().expect("load english model");

    for fixture in load_fixtures() {
        let doc: Document = engine
            .analyze(Input::text(fixture.input.clone(), fixture.format.clone()))
            .unwrap_or_else(|e| panic!("{}: analyze failed: {e}", fixture.name));

        let e = &fixture.expect;
        let name = &fixture.name;

        document_schema::assert_valid(
            "document",
            &serde_json::to_value(&doc).expect("serialize"),
            name,
        );
        // Every outcome of a real parse matches its schema too, and its
        // spans point into the input.
        check_outcomes(&fixture.input, &doc, name);

        // Provenance: this build's schema, the parser's own identity,
        // and the input as it was handed in.
        let p = &doc.provenance;
        assert_eq!(p.schema, matra::domain::SCHEMA_VERSION, "{name}: schema");
        assert_eq!(p.format, fixture.format, "{name}: format");
        assert_eq!(p.parser.name, "udpipe", "{name}: parser name");
        assert_eq!(
            p.parser.model.as_deref(),
            Some("english-ewt-ud-2.5-191206"),
            "{name}: parser model"
        );
        assert!(p.parser.license.is_some(), "{name}: parser licence");
        assert_eq!(p.source.path, None, "{name}: in-memory input");
        assert_eq!(p.source.bytes, fixture.input.len(), "{name}: input bytes");
        let digest: String = Sha256::digest(fixture.input.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(p.source.sha256, digest, "{name}: input digest");

        check_spans(&fixture.input, &doc, name);
        accounting(&fixture.input, &doc).unwrap_or_else(|why| panic!("{name}: L8: {why}"));
        if let Some(want) = &e.sections {
            let got: Vec<_> = doc
                .sections
                .iter()
                .map(|s| (s.heading.clone(), s.level, s.span))
                .collect();
            let want: Vec<_> = want
                .iter()
                .map(|s| (s.heading.clone(), s.level, s.span))
                .collect();
            assert_eq!(got, want, "{name}: sections");
        }
        if let Some(want) = &e.paragraphs {
            let got: Vec<_> = doc.paragraphs().map(|p| (p.kind, p.span)).collect();
            let want: Vec<_> = want.iter().map(|p| (p.kind, p.span)).collect();
            assert_eq!(got, want, "{name}: paragraphs");
        }
        if let Some(want) = &e.skipped {
            let got: Vec<_> = doc.skipped.iter().map(|s| (s.kind, s.span)).collect();
            let want: Vec<_> = want.iter().map(|s| (s.kind, s.span)).collect();
            assert_eq!(got, want, "{name}: skipped");
        }

        let m = &doc.metrics;
        assert_eq!(
            m.total_sentences, e.metrics.total_sentences,
            "{name}: sentence count"
        );
        assert_eq!(m.total_words, e.metrics.total_words, "{name}: word count");
        assert_eq!(
            m.paragraph_count, e.metrics.paragraph_count,
            "{name}: paragraph count"
        );

        let sentences: Vec<_> = doc.sentences().collect();
        assert_eq!(
            sentences.len(),
            e.sentences.len(),
            "{name}: sentences returned"
        );

        for (i, (got, want)) in sentences.iter().zip(&e.sentences).enumerate() {
            assert_eq!(got.text, want.text, "{name}: sentence {i} text");
            if let Some(span) = want.span {
                assert_eq!(got.span, span, "{name}: sentence {i} span");
            }
            if let Some(want_negs) = &want.negations {
                assert_eq!(
                    got.negations.len(),
                    want_negs.len(),
                    "{name}: sentence {i} negation count"
                );
                for (j, (n, w)) in got.negations.iter().zip(want_negs).enumerate() {
                    let at = format!("{name}: sentence {i} negation {j}");
                    assert_eq!(n.cue_id, w.cue_id, "{at} cue_id");
                    assert_eq!(n.cue_lemma, w.cue_lemma, "{at} cue_lemma");
                    assert_eq!(n.head_id, w.head_id, "{at} head_id");
                }
            }
            if let Some(want_modals) = &want.modals {
                assert_eq!(
                    got.modals.len(),
                    want_modals.len(),
                    "{name}: sentence {i} modal count"
                );
                for (j, (m, w)) in got.modals.iter().zip(want_modals).enumerate() {
                    let at = format!("{name}: sentence {i} modal {j}");
                    assert_eq!(m.aux_id, w.aux_id, "{at} aux_id");
                    assert_eq!(m.aux_lemma, w.aux_lemma, "{at} aux_lemma");
                    assert_eq!(m.head_id, w.head_id, "{at} head_id");
                }
            }
            if let Some(want_mood) = &want.root_mood {
                let got_mood = got.root_mood.as_ref().map(|m| ExpectedRootMood {
                    root_id: m.root_id,
                    mood: m.mood.clone(),
                });
                assert_eq!(&got_mood, want_mood, "{name}: sentence {i} root_mood");
            }
            if let Some(want_complements) = &want.clausal_complements {
                assert_eq!(
                    got.clausal_complements.len(),
                    want_complements.len(),
                    "{name}: sentence {i} clausal complement count"
                );
                for (j, (r, w)) in got
                    .clausal_complements
                    .iter()
                    .zip(want_complements)
                    .enumerate()
                {
                    let at = format!("{name}: sentence {i} clausal complement {j}");
                    assert_eq!(r.verb_id, w.verb_id, "{at} verb_id");
                    assert_eq!(r.verb_lemma, w.verb_lemma, "{at} verb_lemma");
                    assert_eq!(r.ccomp_id, w.ccomp_id, "{at} ccomp_id");
                    assert_eq!(r.subject_id, w.subject_id, "{at} subject_id");
                    assert_eq!(r.subject_lemma, w.subject_lemma, "{at} subject_lemma");
                }
            }
            if let Some(want_advs) = &want.root_adverbials {
                assert_eq!(
                    got.root_adverbials.len(),
                    want_advs.len(),
                    "{name}: sentence {i} root adverbial count"
                );
                for (j, (a, w)) in got.root_adverbials.iter().zip(want_advs).enumerate() {
                    let at = format!("{name}: sentence {i} root adverbial {j}");
                    assert_eq!(a.adv_id, w.adv_id, "{at} adv_id");
                    assert_eq!(a.adv_lemma, w.adv_lemma, "{at} adv_lemma");
                }
            }
            if let Some(want_pairs) = &want.hearst_pairs {
                assert_eq!(
                    got.hearst_pairs.len(),
                    want_pairs.len(),
                    "{name}: sentence {i} hearst pair count"
                );
                for (j, (p, w)) in got.hearst_pairs.iter().zip(want_pairs).enumerate() {
                    let at = format!("{name}: sentence {i} hearst pair {j}");
                    assert_eq!(p.pattern, w.pattern, "{at} pattern");
                    for (got_span, want_span, side) in [
                        (&p.hypernym, &w.hypernym, "hypernym"),
                        (&p.hyponym, &w.hyponym, "hyponym"),
                    ] {
                        assert_eq!(got_span.head_id, want_span.head_id, "{at} {side} head_id");
                        assert_eq!(
                            got_span.head_lemma, want_span.head_lemma,
                            "{at} {side} head_lemma"
                        );
                        assert_eq!(
                            got_span.first_id, want_span.first_id,
                            "{at} {side} first_id"
                        );
                        assert_eq!(got_span.last_id, want_span.last_id, "{at} {side} last_id");
                    }
                }
            }
            assert_eq!(
                got.tokens.len(),
                want.token_count,
                "{name}: sentence {i} token count"
            );
            for (j, (t, w)) in got.tokens.iter().zip(&want.tokens).enumerate() {
                let at = format!("{name}: sentence {i} token {j}");
                assert_eq!(t.id, w.id, "{at} id");
                if let Some(span) = w.span {
                    assert_eq!(t.span, span, "{at} span");
                }
                assert_eq!(t.text, w.text, "{at} text");
                assert_eq!(t.lemma, w.lemma, "{at} lemma");
                assert_eq!(t.pos, w.pos, "{at} pos");
                assert_eq!(t.head, w.head, "{at} head");
                assert_eq!(t.dep, w.dep, "{at} dep");
            }
        }

        if let Some(want) = e.metrics.passive_ratio {
            let got = m.passive_ratio.expect("passive_ratio present");
            assert!(close(got, want), "{name}: passive_ratio {got} != {want}");
        }
        if let Some(want) = e.metrics.vocabulary_ttr {
            let got = m.vocabulary_ttr.expect("vocabulary_ttr present");
            assert!(close(got, want), "{name}: vocabulary_ttr {got} != {want}");
        }
        if let Some(want) = e.metrics.nominalization_ratio {
            let got = m
                .nominalization_ratio
                .expect("nominalization_ratio present");
            assert!(
                close(got, want),
                "{name}: nominalization_ratio {got} != {want}"
            );
        }
    }
}
