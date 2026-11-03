# How the search pipeline works

seesea has to answer one question: for a query, which pages should it return, and which passages of each page, so that an AI can use them directly. This page explains why the pipeline has the stages it has and what each stage gives up. Fields and parameters are in the [search response reference](../reference/search-response.md) and the [command-line reference](../reference/cli.md).

## The problem

A single search engine has three faults. Its coverage has blind spots, its snippet often disagrees with the page, and its title can be written to attract clicks. Handing the snippets to an AI passes all three on.

seesea combines three measures. First, it asks several independent sources at once and merges the results; a hit from several sources is itself a quality signal. Second, it downloads the top pages, extracts the body text and re-ranks by the body instead of the snippet. Third, it feeds what use has taught it (which hosts often have useful pages) back into the first ranking layer.

## The stages

```
query → fan-out → fusion and dedup → layer-1 ranking (snippets) → fetch and extract the top few
      → layer-2 ranking (bodies) → section selection → assemble output
```

**Query understanding.** Words such as "latest", "today", "breaking" and years sort the question into evergreen, recent or breaking, and the weight of freshness in ranking changes with the class. `--topic news` treats the question as recent.

**Fan-out.** Every suitable source is asked at once, and a source gets at most 8 seconds before it counts as failed. A failed source does not fail the request. The request fails only when every source failed, or when only auxiliary sources remain and none returned anything.

**Fusion and dedup.** Results from the sources are fused by weighted rank. URLs are normalised first (tracking parameters and so on) and then deduplicated. Each result remembers which sources returned it at which rank; that is `found_by` in the response.

**Layer-1 ranking.** It uses only snippet-level information: the fused score, word overlap between query and snippet, a discounted title match, freshness, and the host and source adjustments learned from history. This layer is cheap. Its job is to decide which pages to download.

**Fetch and extract.** The top candidates from layer 1 are taken (`standard` takes at least 8 by default, `deep` at least 20, and never fewer than the number of results asked for). They are downloaded at most 8 at a time, with 10 seconds per page. `seesea-extract` pulls the body text and drops navigation, ads and footers. A page whose static HTML has no body is rendered by Lightpanda and then extracted. Pages that cannot be read are replaced by the next candidates, for one more round.

**Layer-2 ranking.** The body decides whether the page answers the question. Lexical scoring is always present. With an embedding model, semantic similarity joins in. With a cross-encoder, its judgement joins in as well. Details are in [Ranking and learning](ranking-and-learning.md).

**Section selection.** When a page does not fit the character budget, only the most relevant sections are returned, not the whole page.

**Output.** When one site has more than two results, the extras are down-weighted so one site cannot fill the page. Results with similar titles from different sites merge into one event and are listed under `related`. A page that was read and did answer the question has its title and opening written back to the local index.

## Three modes

All three modes share one pipeline and differ in how many stages run:

| Mode | Fetch | Layer-2 ranking | Section selection |
|---|---|---|---|
| `fast` | None | None, layer 1 only | None |
| `standard` | The top few | Yes | Yes |
| `deep` | More pages | Yes | Yes |

`deep` currently reads more pages and nothing else; it does not rewrite the query.

## Every stage can be absent

One rule of the pipeline is that a missing stage only lowers quality and never fails the request. Without a rerank model, lexical scores are used. When a page cannot be fetched, the snippet stays and ranks behind the pages that were read. When a source is blocked, the others are used. Every degradation is written to `degraded` in the response, so the caller can judge how far to trust the result.

## Why the title has no weight of its own

A title means something only after the body has verified it; otherwise clickbait goes straight into the ranking. So the effective score of a title is the query-to-title match multiplied by how well the body supports the title. Support looks at whether the entities, numbers and key claims of the title appear in the body, and at the semantic similarity between title and the opening of the body.

When the title matches strongly and the body does not support it, the title's contribution goes to zero and can turn negative. In layer 1 only the snippet is available, so support can only be estimated and the title is discounted. In layer 2 the measured support replaces the estimate.

## Trade-offs

Reading bodies makes results more accurate and searches slower. `fast` only merges snippets and returns within a few seconds. `standard` waits for the downloads and took 16 to 23 seconds in one comparison run, where the download stage depends on the slowest page. Because pages download concurrently, one page that times out holds the whole stage past 10 seconds. The `stages` field of the response shows where the time went.

The other trade-off is the dependence on sources. The built-in scraping sources get blocked by CAPTCHAs, so seesea records blocked sources and stops asking them for a while. It also recommends adding a steadier source such as SearXNG or Brave.
