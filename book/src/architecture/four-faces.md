# Four faces of voice

"Voice" is not one thing. Collapsing it into "how writing sounds" buries the structural distinctions that make vaani useful. Four faces, all readable from the same `Document`, each answering a different question about a text.

The four faces are not separate analyses. They are four views into the same structured output. A consumer that needs all four makes one call to `vaani::analyze()` and reads whichever fields apply. The pipeline runs once; the `Document` is queryable from every angle.

Understanding the four faces is what separates a consumer that queries vaani well from one that uses only a fraction of the structure it exposes.

## Agentive: who acts, and with what stance?

Agentive voice asks: who is performing the action? Whose agency is foregrounded? Whose is suppressed?

The answer lives in the dependency arcs. When a sentence is parsed, each token carries a dependency relation (`dep` field) connecting it to its head token. The arc `nsubj` connects a nominal subject to the verb it governs. The arc `obj` connects the direct object. These arcs are what vaani records; their interpretation is what a consumer constructs.

Consider the committee sentence, the pair that appears throughout vaani's documentation precisely because it demonstrates the agentive face clearly:

"The committee approved the proposal without debate."
Here `committee` carries `dep = "nsubj"` pointing to `approved`. Agency is foregrounded. The actor is the grammatical subject, the sentence's structural focus.

"Three amendments were submitted by the working group."
Here `amendments` carries `dep = "nsubj:pass"` pointing to `submitted`, and `was` carries `dep = "aux:pass"`. The patient is now the grammatical subject. The working group is demoted to an oblique argument. In many procedural documents, the agent is omitted entirely: "The proposal was approved." No actor named.

`Sentence::is_passive()` detects this: it checks for any token with `dep == "nsubj:pass"`, `dep == "nsubjpass"`, or `dep == "aux:pass"`. `Document::passive_ratio()` computes the fraction of passive sentences across the whole document.

An application building intent analysis reads these fields to distinguish documents where institutions act versus documents where actions happen to things. The dependency tree also supports richer extraction: `Sentence::children_of(verb_id)` returns all dependents of a verb, letting a consumer pull the full argument frame (subject, object, prepositional arguments) without re-parsing.

## Modal: how certain, how obligated, how evidenced?

Modal voice asks: how does the text qualify its claims? With certainty? With obligation? With epistemic hedging?

English modality is primarily carried by auxiliary verbs. Tokens with `pos = "AUX"` and `dep = "aux"` carry the modal load. The lemma distinguishes the type:

- `must`, `shall`, `have to`: deontic obligation ("you must disclose")
- `should`, `ought to`: deontic recommendation ("you should consider")
- `may`, `might`, `could`: epistemic possibility ("this may be significant")
- `will`, `would`: future or conditional ("the committee will review")

The distinction between deontic and epistemic is not automatic from POS tags alone. The lemma is required. A consumer building a modality classifier reads the `pos`, `dep`, and `lemma` fields together.

vaani's UDPipe adapter preserves the full CoNLL-U annotation set in the `feats` field on each token. This field carries morphological features as a pipe-separated string: `Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin`. The `Mood` feature encodes indicative vs subjunctive. `VerbForm` encodes finite vs infinitive vs participle. `Voice` encodes active vs passive. A consumer that needs fine-grained morphological analysis reads and parses this field.

Evidentiality (markers of how an author knows what they claim) is not directly annotated by UDPipe's English model, but it is recoverable. Lemmas like "reportedly," "allegedly," "according," and collocations like "studies show" appear as `ADV` or `NOUN` tokens at predictable sentence positions. A consumer building evidential detection reads token lemmas and syntactic positions.

## Structural: what forms does the text use?

Structural voice asks: what document architecture is the author choosing? How are sentences constructed? Where does the text concentrate its complexity?

**Section hierarchy.** The `sections` field on `Document` is a `Vec<Section>`, where each `Section` has a `level` (0 for heading-less plain text, 1+ for markdown `#`/`##`/etc.) and a `heading` (`Option<String>`). A flat document with one section level differs structurally from a nested document with three levels, even at identical word counts. The section tree is the skeleton that downstream consumers use to bound their reasoning: "analyze only the second-level sections" is a query that the structure supports directly.

**Sentence rhythm.** `Document::mean_sentence_length()` returns the mean non-punctuation token count per sentence. `Document::sentence_length_std()` returns the sample standard deviation. Together these characterize sentence rhythm: a high mean with low standard deviation signals monotone pacing; a high standard deviation signals varied rhythm with short and long sentences mixed. Neither is inherently good or bad; the interpretation depends on register.

**Nominalization ratio.** `Document::nominalization_ratio` measures the share of noun tokens whose surface form ends in a nominalizing suffix: `tion`, `ment`, `ness`, `ity`, `ence`, `ance`. High nominalization tends to signal bureaucratic or academic register. "The implementation of a solution required coordination of multiple stakeholders" nominalizes four process verbs; "they implemented a solution and coordinated stakeholders" does not. vaani counts the nominalizations; the consumer decides whether the ratio is appropriate.

**Per-paragraph readability.** `Paragraph::readability_grade` is the Flesch-Kincaid grade level for that paragraph (populated only for paragraphs with more than 10 whitespace-counted words). The formula uses raw whitespace splitting, not NLP token counts, per the original 1975 specification.

**Compression ratio.** `Paragraph::compression_ratio` is the brotli-compressed size divided by original size, for paragraphs with more than 50 words. Lower ratio means more compressible, which proxies surface redundancy: text that reuses the same syntactic frames compresses more than text with varied structure. Generated prose tends to compress differently than varied human prose for this reason. This is a rough proxy, not a precise linguistic measure.

## Stylistic: how does authorship signal?

Stylistic voice asks: what does the lexical selection tell us about the author? What patterns in word choice constitute a fingerprint?

**Vocabulary TTR.** `Document::vocabulary_ttr` is the type-token ratio computed over non-punctuation lemmas: unique lemma count divided by total token count. A TTR of 0.65 means 65% of tokens are distinct lemmas. High TTR signals lexical variety; low TTR signals reliance on a smaller core vocabulary. Technical documentation often has low TTR because consistent terminology is a goal; literary prose often has high TTR as a quality signal. The interpretation is always domain-dependent.

**Lexical density.** `Paragraph::lexical_density` is the ratio of content words (tokens whose lowercased lemma is not in the stopword list) to total whitespace-split words in the paragraph. Nouns, main verbs, adjectives, and adverbs count as content words. Articles, prepositions, conjunctions, and auxiliary verbs are function words and do not count. High lexical density means the paragraph packs more information per word; low density means more grammatical scaffolding relative to informational content.

**Keyphrase distribution.** RAKE and YAKE both return `Vec<Keyphrase>` where each entry has `phrase: String` and `score: f64`. The top phrases name the text's dominant themes. The score distribution (whether a few phrases score far above the rest, or scores are evenly distributed) is a signal about thematic focus. A consumer building a voice signature model might use the top-N keyphrases as features, the score distribution's variance as a separate feature, or both.

Together these four stylistic signals (TTR, lexical density, compression ratio, keyphrase distribution) constitute the raw material for a per-author stylometric profile. No single signal is sufficient. The profile emerges from the combination.

## Four consumer use cases

These four faces map to four distinct application patterns. None of these applications is vaani; each is a consumer that queries vaani's output.

**Intent analysis.** Reads the agentive and modal faces. Which agents are foregrounded? Which actions are obligated versus possible? The passive ratio, the subject-verb-object triples from the dependency tree, and the modal auxiliary lemmas are the inputs. The intent taxonomy and classification logic belong to the application.

**Voice signature analysis.** Reads the stylistic and structural faces. Vocabulary TTR, lexical density, mean sentence length, nominalization ratio, compression ratio: together these form a fingerprint. Whether a document matches a target signature is an application judgment made from vaani's measurements.

**Relation and schema extraction.** Reads the agentive face to pull structured triples: who did what to whom. The subject-verb-object pattern is a dependency arc pattern: `nsubj` to verb, `obj` to verb. `Sentence::children_of()` and `Sentence::head_of()` navigate the parse tree to surface these patterns. Rule evaluation over the parse tree (planned v0.2+ 🛠️) will formalize this into a declarative surface.

**Modality and frame transformation.** Reads the modal face to classify epistemic commitment: distinguishing "the board decided" (certain, active) from "the board might decide" (uncertain) from "it is believed that the board decided" (evidential, passive). The `AUX` token lemmas and the `feats` morphological annotations are the inputs; the classification scheme belongs to the application.

## Why four faces instead of one

A writing-coach application that only reads lexical density misses the agentive signal. An intent analysis application that only reads dependency arcs misses the TTR signal. An application detecting epistemic hedging cannot stop at POS tags; it needs the lemma and `feats` to distinguish "may" as possibility from "may" as permission, which requires reading both `dep` and lexical context.

The four faces are not a marketing taxonomy. They are the natural result of asking: what distinct structural questions can a consumer ask of a text? Each face answers a different question from the same `Document`. The consumer decides which questions to ask.

See [The pipeline](./pipeline.md) for how the `Document` is constructed. See [Domain types](../reference/domain-types.md) for the field-level reference on `Token`, `Sentence`, `Paragraph`, `Section`, and `Document`.
