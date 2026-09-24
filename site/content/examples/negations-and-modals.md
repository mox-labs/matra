# Find negations and modals

Which sentences of a legal text are negated, and which carry a modal verb, read off each sentence's dependency tree.

## Input

<example-input name="negations-and-modals" />

## The call

<example-call name="negations-and-modals" />

## Output

<example-output name="negations-and-modals" />

Every primitive of every sentence, marked on the word it is read from and the word it attaches to:

<figure-primitives input="constitution-article-one" />

## What to notice

- Every sentence has a `negations` list and a `modals` list, empty when there is none. A negation names its cue (`cue_lemma`, at token `cue_id`) and the word it attaches to (`head_id`). A modal names its auxiliary (`aux_lemma`, at `aux_id`) and the word it governs (`head_id`). The ids are token ids within the sentence.
- The second sentence of Section 2 has three negations, `no` on `Person` and `not` twice, and four modals, each `shall`. To the parser the Constitution's `shall` is a modal like any other.
- One `not` attaches to `shall` rather than to a verb: in "who shall not, when elected, be an Inhabitant", the parser hung the negation on the auxiliary. `head_id` is what the tree says, and that is all it says.
- To collect the negated sentences, keep those whose `negations` is not empty. What a negation means for your purpose is your code's decision; matra reports where it is.
