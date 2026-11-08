# Search response reference

`seesea search --json`, `GET` and `POST /v1/search`, and the MCP `search` tool return the same JSON.

## Top-level fields

| Field | Type | Description |
|---|---|---|
| `query` | string | The query |
| `mode` | string | `fast`, `standard` or `deep` |
| `took_ms` | integer | Total time in milliseconds |
| `hits` | array | Results, best first |
| `failures` | array | Sources that failed, see below |
| `degraded` | array of strings | Parts of this search that did not work normally |
| `stages` | array | Time per stage |
| `cached` | boolean | Whether the response came from the search cache |

## Each item in hits

| Field | Type | Description |
|---|---|---|
| `url` | string | The result's URL |
| `title` | string | Title |
| `snippet` | string | Snippet from the source |
| `published` | time or null | Publication time, null if unknown |
| `score` | number | Final score; it decides the order |
| `score_parts` | object | How the score is made up, see below |
| `found_by` | array | Which sources returned it, each `{source, rank}` |
| `content` | object or null | The text that was read, see below; null when none was read |
| `content_error` | string or null | Why no text was read |
| `vet` | object or null | How far the address can be trusted, see below; absent for an unknown site with nothing wrong |
| `related` | array | Other reports of the same event, each `{url, title, host}` |

Items in `related` are merged from results with similar titles from different sites: a title word overlap (Jaccard) of 0.55, or the shorter title's containment of 0.85 with at least 4 shared words. Each additional independent site adds 0.03 to the main result, up to 0.09.

### vet

| Field | Meaning |
|---|---|
| `level` | `trusted`, `neutral`, `suspicious`, `risky` or `blocked` |
| `tier` | Popularity: `giant` (top 10 000), `well_known` (top 100 000), `listed` (the rest of the list), `unranked` |
| `rank` | Rank in the list, absent when not listed |
| `official` | The brand whose own domain this is, absent otherwise |
| `flags` | Codes of the signals that raised suspicion, omitted when there are none: `brand_in_subdomain`, `embedded_domain`, `typosquat`, `look_alike_brand`, `homograph`, `ip_host`, `shortener`, `random_name`, `risky_tld`, `path_keywords`, `threat_url`, `threat_host`, `local_block` and others |

Results hit by a threat list or by `block` are left out by default, and `degraded` names the hosts. A `risky` page is not read, so it has no `content`.

### score_parts

| Field | Layer | Description |
|---|---|---|
| `fusion` | first | Score from fusing the sources' ranks |
| `snippet_overlap` | first | Word overlap between query and snippet |
| `title` | first | Query-title match, discounted when the snippet or page does not back the title |
| `freshness` | first | Freshness |
| `vet` | first | Points added or taken away for the address: popular and official sites gain, suspicious and risky ones lose |
| `learned` | first | Points added for how this host and source delivered before |
| `stage1` | first | The first-layer score, normalised so the best is 1 |
| `relevance` | second | How well the text answers the query, 0 to 1 |
| `title_support` | second | How well the text backs up the title, 0 to 1 |
| `body_quality` | second | Text quality: extraction confidence, useful length, repetition |

For a result whose text was not read, the three second-layer fields are 0.

### content

| Field | Description |
|---|---|
| `sections` | The chosen sections, each `{path, markdown, score}`; `path` is the heading path from the top of the page |
| `markdown` | The whole text; present only when `full_content` was requested |
| `confidence` | Extraction confidence |
| `needs_render` | The static HTML held no content; scripts must run to get it |
| `truncated` | Sections were cut by the character budget |

## Each item in failures

| Field | Description |
|---|---|
| `source` | Source name |
| `kind` | `timeout`, `http`, `parse`, `blocked` or `other` |
| `message` | The reason |

## stages

Each item is `{stage, ms}`.

| Stage | Meaning |
|---|---|
| `search` | Asking the sources, in parallel |
| `fuse_rank` | Fusion, deduplication, first-layer ranking |
| `fetch_extract` | Downloading and extracting pages, the whole stage |
| `fetch_slowest_page` | The download time of the slowest page in that stage |
| `extract_slowest_page` | The extraction time of the slowest page |
| `extract_all_pages` | The sum of the extraction times of all pages |
| `render_slowest_page` | The slowest render among pages that needed one |
| `rerank` | Second-layer reranking on the text |
| `assemble` | Choosing sections and building the output |

Pages download in parallel, so `fetch_extract` is roughly the slowest page. `fast` mode has no stages after `fuse_rank`.

## Common degraded messages

| Message | Meaning |
|---|---|
| `N of M sources failed` | Some sources failed; details are in `failures` |
| `returned nothing: name` | A general engine returned no results while others did; it was probably blocked |
| `content: N of M pages could not be fetched or extracted (K timed out)` | Some pages could not be read; `K` of them timed out |
| `name skipped: it blocked us recently ...` | That source returned a captcha or rate limit lately and is not asked for now |
| `semantic rerank failed ...` | The embedding model failed; word-based scores were used |
| `rerank failed ...` | The rerank task failed; content scores were skipped |
