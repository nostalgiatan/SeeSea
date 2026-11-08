# Model load under high concurrency

## The problem

In a search only model inference is heavy. A standard search (12 pages, 3 passages per page sent to the cross-encoder) takes about 450 ms on an RTX 5070 laptop GPU, so one model serves about 2.2 of them per second. Everything else (fusion, scoring, vetting, clustering, section selection) adds up to a few milliseconds for 500 candidates.

The model's compute is fixed. More simultaneous searches only make each one finish later; total throughput does not grow. With 16 clients searching at once every search joins one queue, latency reaches 7 seconds, and throughput stays at 2.3 per second. Merging requests into one batch did not help either: on the GPU it was about 12% faster, on the CPU slower, because both are already saturated.

## What it does

Three measures, aimed at making the model something a search can do without, rather than something every search queues for.

**A limit and a short wait.** At most `model_slots` calls run in one model at a time (default 1). The others wait up to `model_wait_ms` (default 1000 ms). If no turn comes, the search goes on without that model at that step: lexical relevance for page reranking, lexical order for section selection, and `degraded` says "the model is busy". The rest of the pipeline runs as usual, so the number of searches a machine can serve no longer depends on the model.

**Send less while busy.** If other searches are using the cross-encoder, the search sends one passage per page, the most relevant, instead of three. The same compute serves about 2.5 times as many searches, each reranked a little more coarsely, none dropped.

**Remember scores.** Cross-encoder scores for (query, passage) pairs are cached (up to 50 000), so the same query over the same pages does not use the model a second time. The embedding model already had a vector cache.

## Measured

Same model, 16 clients searching for 10 seconds, 12 pages per search:

| | Searches per second | Of those, with the model | Latency p50 | Latency p95 |
|---|---|---|---|---|
| No limit | 2.3 | 100% | about 7.0 s | about 7.0 s |
| Limit (400 ms wait) | 36.4 | 6% | 0.40 s | 0.62 s |
| Limit and send less | 36.5 | 15% | 0.40 s | 0.55 s |

Searches served by the model rise from 2.2 to 5.5 per second, total throughput is more than ten times higher, and latency is bounded by the wait. The price is that under heavy load most searches are ranked on lexical scores without the model. In this test a "search" is only the model call plus a fixed 3 ms, so the rest of the pipeline is not counted; 36 per second is the order of magnitude of the limit, not the throughput of a full service.

## Choosing

- Ranking quality first: raise `model_wait_ms` (say 3000); latency grows under load.
- Latency first: lower it, or set 0 so a busy model is always skipped.
- Several GPUs or a small model: raise `model_slots`. On a CPU keep it at 1; more calls only fight for the same cores.
- For still more throughput, add nodes in cluster mode; each node has its own model.
