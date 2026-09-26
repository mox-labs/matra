# What the clustering threshold does

matra can group the sentences of a document that say the same thing in different words. It embeds each sentence as a vector, scores every pair by cosine similarity, and links each pair whose score clears a threshold you choose. The clusters are the connected groups those links make. [Cluster sentences by meaning](../guides/semantic-clusters.md) has the calls.

Everything else matra returns is structure read off the text, checkable against its bytes. Clusters are not: they depend on a model's representation of meaning. So they arrive as their own value from a separate call, never as a field on `Document`, and they carry the identity of the model that produced them and the threshold you chose.

## Three things the shape means

- **Co-membership is transitive, not pairwise.** Clusters are connected components, so sentence A and sentence C can share a cluster because both resemble B, without resembling each other. The edges travel in the result so you can see which pairs actually cleared the bar. A missing edge is no claim, not a low score.
- **Singletons are always excluded.** A sentence with no pair above the threshold appears in no cluster, so "not in any cluster" is a meaningful count.
- **The threshold is yours, and it does not travel.** Published cutoffs for paraphrase detection span 0.67 to 0.9 with no consensus; the working value depends on the model, the domain and the text length. A cutoff calibrated on sentences is not the cutoff for whole documents, because a document vector is the mean over far more tokens.

## Watch the threshold move

Ten short sentences, with the reference model. Before you move the threshold, predict: sentence 8 is about a different decision by the same committee. As the threshold falls from 0.85, at which value do you expect it to join sentences 1, 2 and 3, and through which of them? Then drag the threshold and find out.

<figure-clusters input="paraphrases" />

<details>
<summary>What sentence 8 did</summary>

It joins at 0.70, through sentence 1 alone: their pair scores 0.7239 and clears the bar, while its pairs with sentences 2 and 3 do not, yet it shares their cluster. A cluster can grow through a chain of pairs, on shared words ("committee approved") rather than shared meaning, and the clusters dissolve as the bar rises past 0.85.

</details>
