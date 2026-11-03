# Ranking and learning

This page explains how scores are computed, what each of the two models does, how layer 1 learns from use, and which conclusions have been measured and which have not. The overall flow is in [How the search pipeline works](pipeline.md).

## Two layers of scores

Layer 1 looks only at snippets and picks the pages worth downloading. Layer 2 has seen the bodies and decides the final order. The final score mixes the two at fixed ratios:

```
final = 0.2 × layer 1 + 0.7 × relevance + 0.1 × body quality
```

The layer-1 score is first normalised so the highest is 1. Relevance and body quality come from layer 2.

Two adjustments:

- When a word of the query is the name of the site (`tokio` for tokio.rs, `postgresql` for postgresql.org, or the first path segment on docs.rs), and the body relevance is at least 0.3 and the quality at least 0.5, 0.08 is added. The reasoning is that when someone asks about a project, the project's own site is usually a good answer.
- A page whose body was not read gets its snippet-level layer-1 score multiplied by 0.45, so it ranks behind pages that were read and are relevant. Without this factor, an unread page could reach 0.62, enough to beat an official document that was read and had middling relevance.

## Where relevance comes from

Relevance stacks three signals, using as many as the loaded models allow.

**Lexical.** BM25, how much of the query the body covers, and agreement between title and body. When the title matches strongly and the body does not support it, the title term is negative. This layer is always present, and without models it is all there is.

**Embedding similarity (bge-m3).** The query and several passages of the page become vectors, and cosine similarity is compared. Each page contributes its opening passage and the best matching few; the best passage counts most and the second best adds a little. Cosine values are mapped into 0 to 1 over an interval, 0.3 to 0.75 by default, tuned for bge-m3. The embedding score carries 0.8 of content relevance, but it is multiplied by how well the body supports the title, so a clickbait page cannot pass on semantic similarity.

**Cross-encoder (bge-reranker-v2-m3).** It reads the query and a passage together. It can tell "mentions the topic" from "answers the question", which comparing vectors cannot. Its logit is mapped to relevance with `sigmoid((logit - center) / scale)`, by default `center = -4` and `scale = 3`: the model is strict, and real answers often land around -4. It carries 0.5 of content relevance, mixed with the lower layer's score (embedding or lexical).

All three scores are multiplied by the non-repetition of the page content, the share of distinct word trigrams. A page that repeats the query dozens of times looks like a perfect match to all three scorers; this factor takes it close to 0.

## Section selection

When a page does not fit the character budget, the lexical scorer first picks the 12 most promising fragments. The loaded model (the cross-encoder if there is one, otherwise the embedding) then scores them again, with 0.3 for lexical and 0.7 for semantic. This lets an answer section be chosen even when it shares no word with the query. `--lexical-sections` turns this step off.

## How layer 1 learns

After a body has been read, the relevance from layer 2 (weight 0.7) and body quality (weight 0.3) are recorded under the page's host and under the source that returned it. A page whose body could not be fetched is recorded as 0.15. Observations decay exponentially with a half-life of 90 days and shrink toward the global mean; about 3 observations are needed before they count. Once they do, layer 1 gets a bounded correction: at most about ±0.075 for a host and about ±0.025 for a source, visible in `score_parts.learned`. `seesea feedback good|bad` counts as 5 observations.

Learning has an obvious bias: only candidates that were read produce observations. To ease it, `explore` makes each search read a lower-ranked result instead with some probability.

What is learned is kept in `learn.json` in the data directory, and a serving process writes it every 30 seconds.

## Diversity of results

From the third result from one host on, each further result has its score multiplied by 0.88, so one site cannot fill the page. Results with similar titles from different sites merge into one event, and the main result gets 0.03 for each extra independent site, up to 0.09, because several independent reports are evidence in themselves.

## What was measured and what was not

On the 8 queries in `tools/search-eval`, nDCG@5 in `deep` mode:

| Setup | nDCG@5 |
|---|---|
| Lexical only | 0.828 |
| bge-m3 embedding | 0.952 |
| Cross-encoder only | 0.945 |
| Embedding plus cross-encoder | 0.970 |

The set has only 8 queries and is already close to saturated, so the differences should not be over-read.

Not measured:

- Whether layer-1 learning raises nDCG. A set of 8 queries cannot show it.
- The effect of the site-name bonus and the unread-page discount on real queries. They were added for what one comparison with Tavily showed, and that run had only 4 queries.
- Whether the semantic mapping interval suits any model other than bge-m3.

On speed: on an RTX 5070 laptop GPU, the cross-encoder takes about 70 milliseconds per query-passage pair of about 280 tokens in steady state. Batching is not faster, and the time grows linearly with the count. The first call has extra warm-up cost.
