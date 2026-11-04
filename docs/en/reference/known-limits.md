# Known limits

This page lists what seesea cannot do yet or has not verified. Each entry describes the present state; none is a promise.

## Search sources

- The built-in sources asked by default are Bing, DuckDuckGo (HTML), Mojeek and Wikipedia. Stack Overflow, arXiv and GitHub answer only certain kinds of question and are used only when named (`-s stackoverflow,arxiv,github`). Baidu and Sogou were removed because of captchas and timeouts.
- Scraping sources break easily. DuckDuckGo and Mojeek often answer cloud hosts and some home networks with a captcha, reported as `Blocked`; on some networks Wikipedia and DuckDuckGo cannot be reached. Only arXiv, Stack Overflow and GitHub were checked against the real services; the parsers for the rest rely on samples.
- The SearXNG and Brave parsers were tested only against hand-written samples, not real services. Run `seesea sources --check` against your own instance.
- A source that answers with a captcha or rate limit is left alone for 5 minutes, doubling with each repeat up to an hour. It is not paused when that would leave nothing to ask.

## Fetching and extraction

- Pages are fetched with browser-like headers by default. A 403, 429 or 503 is retried as another browser profile with a search-engine Referer, at most 3 attempts in all, honouring `Retry-After`. Only headers change: there is no TLS fingerprint imitation, no script execution and no captcha solving, so sites that need these (Zhihu and Baidu Baike, for example) still refuse.
- Pages whose static HTML holds no content are rendered with Lightpanda. The rendering browser may not connect to private addresses, including requests made by page scripts; it cannot work with a SOCKS proxy and cannot get past a server that refuses seesea.
- Among the top candidates, pages that cannot be read are replaced by the next candidates, one round only. A page still unread after that ranks behind the read ones on the score of its title and snippet alone.
- One page times out after 10 seconds, and 8 download at a time. If one page times out, the `fetch_extract` stage takes at least 10 seconds.
- The extractor rebuilds tables and super- and subscripts and drops short promotional blocks. Evaluation data is in the [content extraction study](../../explanation/extraction-research.md) (Chinese).

## Ranking

- `deep` mode reads more pages but does not rewrite queries yet.
- The mapping range for semantic scores (`semantic.range`, default 0.3 to 0.75) was tuned for bge-m3. Other embedding models need their own; BGE-small English is about 0.45 to 0.85.
- The weights of the embedding model and the cross-encoder were tuned on 8 queries in `tools/search-eval`. That set is close to saturated, so differences should not be over-read.
- No evidence yet that first-layer learning raises nDCG on a judged set. A cached response to the same query is not updated as it learns. The host table holds at most 20,000 hosts.
- Seen in a comparison with Tavily: some pages that only touch the query (for example, a ranking for another region when the query names no region) still reach the top few places. Fixing this takes a judged dataset.

## Security

- SSRF guard: the client that fetches pages connects only to public addresses. Its resolver drops private addresses from every answer (loopback, RFC 1918, 100.64.0.0/10, link-local including 169.254.169.254, ULA, and IPv4-mapped, NAT64 and 6to4 forms that embed them), so a name that changes its answer at connection time is caught too. Redirects are checked against the target before the request is sent. `198.18.0.0/15` (the fake-IP range of TUN-style proxies) counts as public. With `--proxy` the proxy resolves, so only the addresses written in the URL and in redirects can be checked.
- Markup is checked before extraction: wrapper elements nested deeper than 128 levels lose their tags and keep their content, a page with more than 100,000 elements is cut there, and a tag that never ends cuts off everything after it. A page that is nested too deeply, or has a tag that never ends, is treated as hostile: it counts as unreadable (it ranks behind the pages that were read, `content_error` says why, and the host is recorded as unreadable). Too many elements is only noted; the page is still used.
- WebSub was checked only with unit tests and local HTTP tests, not against a real hub.

## Multiple nodes

- A cluster has at most two flywheels and a witness. Data nodes do not scale out.
- Replication is asynchronous except for account changes. After an unplanned crash the latest bit of token-bucket and usage counts (5 seconds), host standings (1 minute), search records (30 seconds) and index items (10 minutes) may be lost.
- Failover after an unplanned crash takes about 4 to 5 seconds. In that time requests slow down or fail after 5 retries.
- Each gateway counts its own concurrency limit and failed-key attempts, so one user on N gateways can have up to N times the concurrency. Every request costs one extra round trip to the flywheel.
- It relies on machine clocks running at the same speed; it does not need them to agree.
- It was never tested on real separate machines or across data centres. `scripts/netns-cluster.sh` simulates a cluster on one machine with network namespaces and 2 ms of delay.
- There is no sign-up page, billing or console.

## Storage

- Storage is memory and file caches, plus SQLite (the user database) and per-day SQLite files (the local index). Turso is not used.
