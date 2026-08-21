//! Hearst-pattern detection: candidate hypernymy pairs as structure.
//!
//! The six lexico-syntactic patterns of Hearst (1992), implemented as
//! dependency-arc patterns over the parse rather than regex over
//! surface text: the 1992 work used regex because parsers were not
//! available, and matra has a parser. Each detection returns a
//! [`HearstPair`]: a (hypernym span, hyponym span) pair referencing
//! token ids, tagged with the [`HearstPattern`] that matched, so
//! provenance holds against [`crate::domain::Sentence::tokens`].
//!
//! Precision is favoured over recall throughout: a missed pattern is
//! acceptable, a false pair is not. Every arc shape below was verified
//! against live UDPipe parses (english-ewt, 2026-08-21) before being
//! encoded, and each detector requires the full shape (markers on
//! their observed relations, left-to-right id order) rather than any
//! single cue word.
//!
//! Substrate discipline: the detector reports that a sentence used a
//! construction which conventionally signals hypernymy. It does not
//! build a taxonomy and does not assert the relation is true; that
//! reading is the consumer's.
//!
//! This module imports only `domain` (I7 M5 boundary). The pipeline
//! calls [`hypernymy_pairs`] at the annotate stage and stores the
//! result on [`crate::domain::Sentence::hearst_pairs`], so the
//! detection crosses FFI as data (ADR-0008).

use crate::domain::{HearstPair, HearstPattern, HearstSpan, Token};

/// Parts of speech a Hearst noun phrase head may carry. Pronouns and
/// adjectives are excluded by decision: "anyone such as her" and the
/// idiom "as such" (where `such` itself lands on the `nmod` arc, ADJ)
/// must not produce pairs.
const NOMINAL_POS: [&str; 2] = ["NOUN", "PROPN"];

/// Relations that extend a noun's span leftward over its adjacent
/// modifiers: "Common-law countries" spans all three tokens, "New
/// York" spans both. Pattern marker words (`such` as `amod`, `other`
/// as `amod`) are excluded by the caller via the marker list.
const SPAN_MODIFIER_DEPS: [&str; 5] = ["det", "amod", "compound", "nummod", "flat"];

fn is_nominal(t: &Token) -> bool {
    NOMINAL_POS.contains(&t.pos.as_str())
}

fn by_id(tokens: &[Token], id: usize) -> Option<&Token> {
    tokens.iter().find(|t| t.id == id)
}

fn children(tokens: &[Token], head_id: usize) -> impl Iterator<Item = &Token> {
    tokens.iter().filter(move |t| t.head == head_id)
}

/// The contiguous span of `head` plus its adjacent nominal modifiers.
///
/// Walks left from the head over consecutive tokens that attach to it
/// with a relation in [`SPAN_MODIFIER_DEPS`], stopping at the first
/// token that does not (or that is a pattern marker), then walks right
/// over `flat` continuations of a multiword name. Contiguity is
/// required, not assumed: a modifier separated from the head by an
/// intervening non-modifier does not extend the span, which keeps a
/// span from silently swallowing the connective or the other noun
/// phrase of the pattern.
fn span(tokens: &[Token], head: &Token, marker_ids: &[usize]) -> HearstSpan {
    let extends_left = |id: usize| {
        !marker_ids.contains(&id)
            && by_id(tokens, id)
                .is_some_and(|t| t.head == head.id && SPAN_MODIFIER_DEPS.contains(&t.dep.as_str()))
    };
    let mut first_id = head.id;
    while first_id > 1 && extends_left(first_id - 1) {
        first_id -= 1;
    }
    let extends_right = |id: usize| {
        by_id(tokens, id)
            .is_some_and(|t| t.head == head.id && (t.dep == "flat" || t.dep.starts_with("flat:")))
    };
    let mut last_id = head.id;
    while extends_right(last_id + 1) {
        last_id += 1;
    }
    HearstSpan {
        head_id: head.id,
        head_lemma: head.lemma.clone(),
        first_id,
        last_id,
    }
}

/// Detect the six Hearst (1992) patterns over one sentence's tokens.
///
/// Returns candidate pairs in token order, each tagged with the
/// pattern that matched. Ids in the result are sentence-scoped token
/// ids into `tokens`. An empty result means no construction matched,
/// which (by the precision bias) includes sentences the model
/// misparsed: the detector reports only what the arcs support.
pub fn hypernymy_pairs(tokens: &[Token]) -> Vec<HearstPair> {
    let mut pairs = Vec::new();
    for t in tokens {
        if !is_nominal(t) {
            continue;
        }
        match t.dep.as_str() {
            "nmod" => detect_nmod_family(tokens, t, &mut pairs),
            "conj" => detect_conj_family(tokens, t, &mut pairs),
            _ => {}
        }
    }
    pairs
}

/// The three patterns whose hyponym attaches as `nmod` to the
/// hypernym, verified live:
///
/// - `NP such as NP` ("Animals such as dogs and cats"): `such` is
///   `case` on the hyponym, with `as` attached to it as `fixed`.
/// - `such NP as NP` ("such authors as Herrick"): `such` is `amod` on
///   the hypernym, `as` is `case` on the hyponym.
/// - `NP, including NP` ("countries, including Canada"): `including`
///   (lemma `include`) is `case` on the hyponym.
///
/// Further hyponyms ride `conj` arcs off the first ("dogs and cats":
/// `cats` is `conj` on `dogs`). The verbal use of `including` ("The
/// committee is including new members") produces no `nmod` arc and
/// cannot fire.
fn detect_nmod_family(tokens: &[Token], t: &Token, out: &mut Vec<HearstPair>) {
    let Some(h) = by_id(tokens, t.head) else {
        return;
    };
    if !is_nominal(h) || h.id >= t.id {
        return;
    }
    let case_children: Vec<&Token> = children(tokens, t.id)
        .filter(|c| c.dep == "case" && c.id > h.id && c.id < t.id)
        .collect();

    let such_case = case_children.iter().any(|c| {
        c.lemma == "such" && children(tokens, c.id).any(|f| f.dep == "fixed" && f.lemma == "as")
    });
    let such_amod =
        children(tokens, h.id).find(|c| c.dep == "amod" && c.lemma == "such" && c.id < h.id);
    let as_case = case_children.iter().any(|c| c.lemma == "as");
    let include_case = case_children.iter().any(|c| c.lemma == "include");

    let (pattern, hyper_markers) = if such_case {
        (HearstPattern::SuchAs, Vec::new())
    } else if let (Some(s), true) = (such_amod, as_case) {
        (HearstPattern::SuchNpAs, vec![s.id])
    } else if include_case {
        (HearstPattern::Including, Vec::new())
    } else {
        return;
    };

    let hypernym = span(tokens, h, &hyper_markers);
    let mut hyponyms = vec![t];
    hyponyms
        .extend(children(tokens, t.id).filter(|c| c.dep == "conj" && c.id > t.id && is_nominal(c)));
    for n in hyponyms {
        out.push(HearstPair {
            pattern,
            hypernym: hypernym.clone(),
            hyponym: span(tokens, n, &[]),
        });
    }
}

/// The three patterns whose marked noun attaches as `conj`, verified
/// live:
///
/// - `NP {, NP}* {and|or} other NP` ("temples, treasuries, and other
///   civic buildings"): the *hypernym* is the conjunct carrying both a
///   `cc` child (`and`/`or`) and an `amod` child with lemma `other`;
///   the hyponyms are the head of its `conj` arc plus that head's
///   earlier `conj` children.
/// - `NP, especially NP` ("countries, especially France"): the
///   *hyponym* is an asyndetic conjunct (no `cc` child) carrying
///   `especially` as a preceding `advmod`. The no-`cc` requirement is
///   the discriminator against plain coordination ("France and
///   especially Spain"), where the conjunct carries `cc` or the
///   adverb lands on the verb, both verified live.
///
/// Clausal coordination ("She invited John, and other guests brought
/// gifts.") cannot fire: `guests` rides `nsubj`, not `conj`. A `conj`
/// arc pointing rightward at its head (seen only on misparses) is
/// rejected by the id-order guard.
fn detect_conj_family(tokens: &[Token], t: &Token, out: &mut Vec<HearstPair>) {
    let Some(f) = by_id(tokens, t.head) else {
        return;
    };
    if !is_nominal(f) || f.id >= t.id {
        return;
    }
    let cc = children(tokens, t.id).find(|c| c.dep == "cc");
    let other =
        children(tokens, t.id).find(|c| c.dep == "amod" && c.lemma == "other" && c.id < t.id);
    let especially = children(tokens, t.id)
        .find(|c| c.dep == "advmod" && c.lemma == "especially" && c.id < t.id);

    if let (Some(c), Some(o)) = (cc, other) {
        if c.id < o.id && (c.lemma == "and" || c.lemma == "or") {
            let pattern = if c.lemma == "and" {
                HearstPattern::AndOther
            } else {
                HearstPattern::OrOther
            };
            let hypernym = span(tokens, t, &[o.id]);
            let mut hyponyms = vec![f];
            hyponyms.extend(
                children(tokens, f.id)
                    .filter(|g| g.dep == "conj" && g.id > f.id && g.id < c.id && is_nominal(g)),
            );
            for n in hyponyms {
                out.push(HearstPair {
                    pattern,
                    hypernym: hypernym.clone(),
                    hyponym: span(tokens, n, &[]),
                });
            }
        }
    } else if cc.is_none() {
        if let Some(e) = especially {
            out.push(HearstPair {
                pattern: HearstPattern::Especially,
                hypernym: span(tokens, f, &[]),
                hyponym: span(tokens, t, &[e.id]),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Token fixtures below transcribe live UDPipe parses (english-ewt,
    /// verified 2026-08-21) verbatim: ids, lemmas, POS, heads, and
    /// relations are the model's output, not hand-idealized shapes.
    fn tok(id: usize, text: &str, lemma: &str, pos: &str, dep: &str, head: usize) -> Token {
        Token::builder(
            id,
            text.to_string(),
            lemma.to_string(),
            pos.to_string(),
            head,
            dep.to_string(),
        )
        .is_punct(pos == "PUNCT")
        .build()
    }

    fn span_of(head_id: usize, head_lemma: &str, first_id: usize, last_id: usize) -> HearstSpan {
        HearstSpan {
            head_id,
            head_lemma: head_lemma.to_string(),
            first_id,
            last_id,
        }
    }

    fn pair(pattern: HearstPattern, hypernym: HearstSpan, hyponym: HearstSpan) -> HearstPair {
        HearstPair {
            pattern,
            hypernym,
            hyponym,
        }
    }

    // "Animals such as dogs and cats need daily care."
    #[test]
    fn such_as_detects_pair_and_conjuncts() {
        let tokens = vec![
            tok(1, "Animals", "animal", "NOUN", "nsubj", 7),
            tok(2, "such", "such", "ADJ", "case", 4),
            tok(3, "as", "as", "ADP", "fixed", 2),
            tok(4, "dogs", "dog", "NOUN", "nmod", 1),
            tok(5, "and", "and", "CCONJ", "cc", 6),
            tok(6, "cats", "cat", "NOUN", "conj", 4),
            tok(7, "need", "need", "VERB", "root", 0),
            tok(8, "daily", "daily", "ADJ", "amod", 9),
            tok(9, "care", "care", "NOUN", "obj", 7),
            tok(10, ".", ".", "PUNCT", "punct", 7),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![
                pair(
                    HearstPattern::SuchAs,
                    span_of(1, "animal", 1, 1),
                    span_of(4, "dog", 4, 4),
                ),
                pair(
                    HearstPattern::SuchAs,
                    span_of(1, "animal", 1, 1),
                    span_of(6, "cat", 6, 6),
                ),
            ]
        );
    }

    // "The works of such authors as Herrick and Shakespeare are studied."
    // The `of` nmod arc (works -> authors) must not fire; only the
    // such-marked one (authors -> Herrick) may.
    #[test]
    fn such_np_as_detects_pair_and_excludes_such_from_span() {
        let tokens = vec![
            tok(1, "The", "the", "DET", "det", 2),
            tok(2, "works", "work", "NOUN", "nsubj:pass", 11),
            tok(3, "of", "of", "ADP", "case", 5),
            tok(4, "such", "such", "ADJ", "amod", 5),
            tok(5, "authors", "author", "NOUN", "nmod", 2),
            tok(6, "as", "as", "ADP", "case", 7),
            tok(7, "Herrick", "Herrick", "PROPN", "nmod", 5),
            tok(8, "and", "and", "CCONJ", "cc", 9),
            tok(9, "Shakespeare", "Shakespeare", "PROPN", "conj", 7),
            tok(10, "are", "be", "AUX", "aux:pass", 11),
            tok(11, "studied", "study", "VERB", "root", 0),
            tok(12, ".", ".", "PUNCT", "punct", 11),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![
                pair(
                    HearstPattern::SuchNpAs,
                    span_of(5, "author", 5, 5),
                    span_of(7, "Herrick", 7, 7),
                ),
                pair(
                    HearstPattern::SuchNpAs,
                    span_of(5, "author", 5, 5),
                    span_of(9, "Shakespeare", 9, 9),
                ),
            ]
        );
    }

    // "Common-law countries, including Canada and England, use juries."
    #[test]
    fn including_detects_pair_with_modifier_span() {
        let tokens = vec![
            tok(1, "Common", "common", "ADJ", "amod", 3),
            tok(2, "-law", "-law", "ADJ", "amod", 3),
            tok(3, "countries", "country", "NOUN", "nsubj", 10),
            tok(4, ",", ",", "PUNCT", "punct", 3),
            tok(5, "including", "include", "VERB", "case", 6),
            tok(6, "Canada", "Canada", "PROPN", "nmod", 3),
            tok(7, "and", "and", "CCONJ", "cc", 8),
            tok(8, "England", "England", "PROPN", "conj", 6),
            tok(9, ",", ",", "PUNCT", "punct", 10),
            tok(10, "use", "use", "VERB", "root", 0),
            tok(11, "juries", "jury", "NOUN", "obj", 10),
            tok(12, ".", ".", "PUNCT", "punct", 10),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![
                pair(
                    HearstPattern::Including,
                    span_of(3, "country", 1, 3),
                    span_of(6, "Canada", 6, 6),
                ),
                pair(
                    HearstPattern::Including,
                    span_of(3, "country", 1, 3),
                    span_of(8, "England", 8, 8),
                ),
            ]
        );
    }

    // "Most European countries, especially France and Spain, joined early."
    // France carries the especially advmod and no cc: it fires. Spain
    // rides a plain cc-marked conj: it does not (recall given up for
    // precision, per the milestone rubric).
    #[test]
    fn especially_detects_marked_conjunct_only() {
        let tokens = vec![
            tok(1, "Most", "most", "ADJ", "amod", 3),
            tok(2, "European", "european", "ADJ", "amod", 3),
            tok(3, "countries", "country", "NOUN", "nsubj", 10),
            tok(4, ",", ",", "PUNCT", "punct", 6),
            tok(5, "especially", "especially", "ADV", "advmod", 6),
            tok(6, "France", "France", "PROPN", "conj", 3),
            tok(7, "and", "and", "CCONJ", "cc", 8),
            tok(8, "Spain", "Spain", "PROPN", "conj", 3),
            tok(9, ",", ",", "PUNCT", "punct", 10),
            tok(10, "joined", "join", "VERB", "root", 0),
            tok(11, "early", "early", "ADV", "advmod", 10),
            tok(12, ".", ".", "PUNCT", "punct", 10),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![pair(
                HearstPattern::Especially,
                span_of(3, "country", 1, 3),
                span_of(6, "France", 6, 6),
            )]
        );
    }

    // "The bruise, the wound, or other injuries healed slowly."
    #[test]
    fn or_other_detects_first_conjunct_and_earlier_siblings() {
        let tokens = vec![
            tok(1, "The", "the", "DET", "det", 2),
            tok(2, "bruise", "bruise", "NOUN", "root", 0),
            tok(3, ",", ",", "PUNCT", "punct", 5),
            tok(4, "the", "the", "DET", "det", 5),
            tok(5, "wound", "wound", "NOUN", "conj", 2),
            tok(6, ",", ",", "PUNCT", "punct", 9),
            tok(7, "or", "or", "CCONJ", "cc", 9),
            tok(8, "other", "other", "ADJ", "amod", 9),
            tok(9, "injuries", "injury", "NOUN", "conj", 2),
            tok(10, "healed", "heal", "VERB", "acl", 9),
            tok(11, "slowly", "slowly", "ADV", "advmod", 10),
            tok(12, ".", ".", "PUNCT", "punct", 2),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![
                pair(
                    HearstPattern::OrOther,
                    span_of(9, "injury", 9, 9),
                    span_of(2, "bruise", 1, 2),
                ),
                pair(
                    HearstPattern::OrOther,
                    span_of(9, "injury", 9, 9),
                    span_of(5, "wound", 4, 5),
                ),
            ]
        );
    }

    // "They built temples, treasuries, and other important civic buildings."
    // The hypernym span keeps `important civic` and drops the `other`
    // marker.
    #[test]
    fn and_other_detects_pairs_and_excludes_other_from_span() {
        let tokens = vec![
            tok(1, "They", "they", "PRON", "nsubj", 2),
            tok(2, "built", "build", "VERB", "root", 0),
            tok(3, "temples", "temple", "NOUN", "obj", 2),
            tok(4, ",", ",", "PUNCT", "punct", 5),
            tok(5, "treasuries", "treasury", "NOUN", "conj", 3),
            tok(6, ",", ",", "PUNCT", "punct", 11),
            tok(7, "and", "and", "CCONJ", "cc", 11),
            tok(8, "other", "other", "ADJ", "amod", 11),
            tok(9, "important", "important", "ADJ", "amod", 11),
            tok(10, "civic", "civic", "ADJ", "amod", 11),
            tok(11, "buildings", "building", "NOUN", "conj", 3),
            tok(12, ".", ".", "PUNCT", "punct", 2),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![
                pair(
                    HearstPattern::AndOther,
                    span_of(11, "building", 9, 11),
                    span_of(3, "temple", 3, 3),
                ),
                pair(
                    HearstPattern::AndOther,
                    span_of(11, "building", 9, 11),
                    span_of(5, "treasury", 5, 5),
                ),
            ]
        );
    }

    // "Cities such as New York and Los Angeles grew quickly."
    // Multiword names span their compound modifiers.
    #[test]
    fn spans_cover_compound_names() {
        let tokens = vec![
            tok(1, "Cities", "city", "NOUN", "nsubj", 9),
            tok(2, "such", "such", "ADJ", "case", 5),
            tok(3, "as", "as", "ADP", "fixed", 2),
            tok(4, "New", "New", "PROPN", "compound", 5),
            tok(5, "York", "York", "PROPN", "nmod", 1),
            tok(6, "and", "and", "CCONJ", "cc", 8),
            tok(7, "Los", "Los", "PROPN", "compound", 8),
            tok(8, "Angeles", "Angeles", "PROPN", "conj", 5),
            tok(9, "grew", "grow", "VERB", "root", 0),
            tok(10, "quickly", "quickly", "ADV", "advmod", 9),
            tok(11, ".", ".", "PUNCT", "punct", 9),
        ];
        assert_eq!(
            hypernymy_pairs(&tokens),
            vec![
                pair(
                    HearstPattern::SuchAs,
                    span_of(1, "city", 1, 1),
                    span_of(5, "York", 4, 5),
                ),
                pair(
                    HearstPattern::SuchAs,
                    span_of(1, "city", 1, 1),
                    span_of(8, "Angeles", 7, 8),
                ),
            ]
        );
    }

    // Hard negative: "She invited John, and other guests brought gifts."
    // Surface text contains ", and other" but the coordination is
    // clausal: `guests` rides nsubj under the second verb, not conj
    // under a noun. A regex fires here; the arc pattern must not.
    #[test]
    fn clausal_and_other_does_not_fire() {
        let tokens = vec![
            tok(1, "She", "she", "PRON", "nsubj", 2),
            tok(2, "invited", "invite", "VERB", "root", 0),
            tok(3, "John", "John", "PROPN", "obj", 2),
            tok(4, ",", ",", "PUNCT", "punct", 8),
            tok(5, "and", "and", "CCONJ", "cc", 8),
            tok(6, "other", "other", "ADJ", "amod", 7),
            tok(7, "guests", "guest", "NOUN", "nsubj", 8),
            tok(8, "brought", "bring", "VERB", "conj", 2),
            tok(9, "gifts", "gift", "NOUN", "obj", 8),
            tok(10, ".", ".", "PUNCT", "punct", 2),
        ];
        assert_eq!(hypernymy_pairs(&tokens), Vec::new());
    }

    // Hard negative: "France and especially Spain joined early."
    // Plain coordination with emphasis, not a class-member listing.
    // On the live parse both nouns ride nsubj and `especially` lands
    // on the verb; nothing may fire.
    #[test]
    fn plain_coordination_especially_does_not_fire() {
        let tokens = vec![
            tok(1, "France", "France", "PROPN", "nsubj", 5),
            tok(2, "and", "and", "CCONJ", "cc", 5),
            tok(3, "especially", "especially", "ADV", "advmod", 5),
            tok(4, "Spain", "Spain", "PROPN", "nsubj", 5),
            tok(5, "joined", "join", "VERB", "root", 0),
            tok(6, "early", "early", "ADV", "advmod", 5),
            tok(7, ".", ".", "PUNCT", "punct", 5),
        ];
        assert_eq!(hypernymy_pairs(&tokens), Vec::new());
    }

    // Hard negative, synthetic arc shape for the same sentence: were
    // the model to attach Spain as conj under France with cc `and`
    // plus advmod `especially`, the cc child marks it as syndetic
    // coordination and the especially arm must stay silent.
    #[test]
    fn syndetic_especially_conjunct_does_not_fire() {
        let tokens = vec![
            tok(1, "France", "France", "PROPN", "nsubj", 5),
            tok(2, "and", "and", "CCONJ", "cc", 4),
            tok(3, "especially", "especially", "ADV", "advmod", 4),
            tok(4, "Spain", "Spain", "PROPN", "conj", 1),
            tok(5, "joined", "join", "VERB", "root", 0),
            tok(6, ".", ".", "PUNCT", "punct", 5),
        ];
        assert_eq!(hypernymy_pairs(&tokens), Vec::new());
    }

    // Hard negative: "He dismissed the report as such." The idiom puts
    // `such` itself on the nmod arc with `as` as its case marker; the
    // nominal requirement keeps it out.
    #[test]
    fn as_such_idiom_does_not_fire() {
        let tokens = vec![
            tok(1, "He", "he", "PRON", "nsubj", 2),
            tok(2, "dismissed", "dismiss", "VERB", "root", 0),
            tok(3, "the", "the", "DET", "det", 4),
            tok(4, "report", "report", "NOUN", "obj", 2),
            tok(5, "as", "as", "ADP", "case", 6),
            tok(6, "such", "such", "ADJ", "nmod", 4),
            tok(7, ".", ".", "PUNCT", "punct", 2),
        ];
        assert_eq!(hypernymy_pairs(&tokens), Vec::new());
    }

    // Hard negative: "The committee is including new members this year."
    // Verbal `including` takes an obj, produces no nmod-with-case arc,
    // and must not fire. Live arcs transcribed from a misparse (the
    // model parked `include` as case under `year`), which is exactly
    // why the detector demands the full nmod shape.
    #[test]
    fn verbal_including_does_not_fire() {
        let tokens = vec![
            tok(1, "The", "the", "DET", "det", 2),
            tok(2, "committee", "committee", "NOUN", "nsubj", 8),
            tok(3, "is", "be", "AUX", "cop", 8),
            tok(4, "including", "include", "VERB", "case", 8),
            tok(5, "new", "new", "ADJ", "amod", 6),
            tok(6, "members", "member", "NOUN", "obj", 4),
            tok(7, "this", "this", "DET", "det", 8),
            tok(8, "year", "year", "NOUN", "root", 0),
            tok(9, ".", ".", "PUNCT", "punct", 8),
        ];
        assert_eq!(hypernymy_pairs(&tokens), Vec::new());
    }

    // Precision guard: "Boston, Philadelphia, and other cities held
    // elections." The live model misparses this (cities rides conj
    // pointing FORWARD at an appos head), so the true pairs are
    // missed. The id-order guard must keep the misparse from
    // producing a false pair; missing is the accepted cost.
    #[test]
    fn forward_pointing_conj_head_does_not_fire() {
        let tokens = vec![
            tok(1, "Boston", "Boston", "PROPN", "root", 0),
            tok(2, ",", ",", "PUNCT", "punct", 3),
            tok(3, "Philadelphia", "Philadelphia", "PROPN", "conj", 1),
            tok(4, ",", ",", "PUNCT", "punct", 7),
            tok(5, "and", "and", "CCONJ", "cc", 7),
            tok(6, "other", "other", "ADJ", "amod", 7),
            tok(7, "cities", "city", "NOUN", "conj", 9),
            tok(8, "held", "hold", "VERB", "amod", 9),
            tok(9, "elections", "election", "NOUN", "appos", 1),
            tok(10, ".", ".", "PUNCT", "punct", 1),
        ];
        assert_eq!(hypernymy_pairs(&tokens), Vec::new());
    }

    #[test]
    fn empty_tokens_yield_no_pairs() {
        assert_eq!(hypernymy_pairs(&[]), Vec::new());
    }

    // The pattern tag is wire schema: pin the serde names each crust
    // will read.
    #[test]
    fn pattern_tags_serialize_snake_case() {
        let tags: Vec<(HearstPattern, &str)> = vec![
            (HearstPattern::SuchAs, "such_as"),
            (HearstPattern::SuchNpAs, "such_np_as"),
            (HearstPattern::Including, "including"),
            (HearstPattern::Especially, "especially"),
            (HearstPattern::AndOther, "and_other"),
            (HearstPattern::OrOther, "or_other"),
        ];
        for (pattern, expected) in tags {
            assert_eq!(
                serde_json::to_value(pattern).unwrap(),
                serde_json::Value::String(expected.to_string())
            );
        }
    }
}
