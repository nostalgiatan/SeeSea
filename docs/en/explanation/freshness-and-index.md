# Freshness and the local index

Search engines often answer "what happened today" badly: the crawler has not arrived yet, or the snippet is old. seesea fills the gap with two things: ranking that adapts to how time-sensitive the question is, and a local index it maintains itself. The steps are in [Fill the local index](../how-to/fill-the-local-index.md).

## Three classes of time sensitivity

Questions are sorted into three classes by their wording. Words such as "latest", "today", "breaking" and years decide it:

| Class | Freshness weight | Half-life |
|---|---|---|
| Evergreen | ×1 | 365 days |
| Recent | ×4 | 14 days |
| Breaking | ×8 | half a day |

The half-life is the time for an article's freshness score to fall to half. For a breaking question, an article from a few hours ago clearly beats one from a few days ago. `--topic news` treats the question as at least recent.

The news source `bingnews` joins automatically only for non-evergreen questions, and always when named with `-s bingnews`.

## Why an index of its own

The value of the real-time layer is that it does not depend on the coverage of outside engines. The index holds articles pushed from RSS, Atom, JSON Feed and `POST /v1/ingest`. At search time it takes part as the `local` source, and it can return results with the network down.

The index stores only the title, a lead of about 400 characters, the date and the host, never the body. The pipeline fetches the body live at search time, so the index stays small and never passes stale body text off as new. There is one SQLite file per day with a title-weighted FTS5 full-text index: a title hit counts three times a lead hit, Chinese is split per character, and queries use adjacent pairs of characters. Duplicates are found by normalised URL and by "same site, same title".

A page that was read and did answer the question (rerank relevance at least 0.5, body quality at least 0.4) is also written back to the index, again with only title and opening. Written-back pages have no publication date, so they are not taken for new content.

## How feeds come in

Polling supports RSS 2.0, RSS 1.0, Atom and JSON Feed. A failing feed backs off exponentially and is disabled after 3 failures in the trial period or 6 in the regular state. The polling interval adapts between 5 minutes and 6 hours depending on whether new items appear. Polling runs 6 feeds at once, each at most 2 MB with a 15 second timeout, and a failing feed affects only itself. Google News sitemaps count as feeds too.

Sites are discovered like this: sites seen in search results are queued, and each round probes at most 3. A probe first looks for `<link rel=alternate>` on the home page, then tries common places such as `/feed` and `/atom.xml`; when the home page has no link and Lightpanda is installed, it renders the page and looks again. A discovered feed starts in the trial period.

If a feed declares a WebSub hub, a server with a public address set subscribes to the hub and receives pushes at its callback instead of waiting for the next poll. The callback is checked by challenge echo and an HMAC signature. WebSub has been tested only with unit tests and a local HTTP test, not against a real hub.

## Semantic search with bit vectors, not a vector store

With an embedding model, every item gets a vector. To keep the index small, the vector keeps only the sign bits after subtracting the mean of the first batch: 1024 dimensions, 128 bytes per item. A search scans the recent days' bit vectors by Hamming distance, brute force. A distance within 42% of the bits counts as near, and the result is merged with the full-text result by RRF.

One measured example: with bge-m3, the query "refrigerators hacked in supermarkets" finds three articles in Chinese and English reporting one event, at distances of 316 to 364 bits, while unrelated content sits at 430 to 460. So an English question finds Chinese reports. `seesea index near "question"` shows the nearest neighbours and their distances.

Brute force was chosen because the candidate set is small (the last few days) and needs no persistent vector store. The cost is that scans slow down as the index grows, so the retention period and the cap are required.

## Event grouping

When several outlets report one event, the results contain several items with similar titles. When the word overlap of the titles (Jaccard) reaches 0.55, or the containment of the shorter title reaches 0.85 with at least 4 shared words, and the items come from different sites, they merge into one, and the rest are listed under `related`. Each extra independent site adds 0.03 to the main result, up to 0.09. The same event in different languages does not merge.

## Details in ranking

The local index first takes candidates from each day's file and then scores them with document frequencies accumulated across days, so ranking does not depend on which day an item landed in.

For errors, the local index is an auxiliary source: when all real sources fail and the index has no hit, the request still reports `all_sources_failed`. When the index has hits, the results are returned and `degraded` notes the degradation.
