# Check search quality

This page shows how to use the search record, judged data and feedback to find ranking problems and measure them. To compare with another search service, see [Compare with Tavily](compare-with-tavily.md).

## See what went wrong lately

Every search (command line, HTTP, MCP) and every piece of feedback is recorded under `journal/` in the data directory, one JSONL file a day, kept 60 days. A record holds the query, each result's rank and score parts, why sources failed, the time per stage and the pages that could not be read. `--no-journal` or `no_journal = true` in the config turns it off.

Summarise the last 7 days:

```bash
seesea report --since 7d
```

It writes four files, by default into a new folder under the data directory; `--out` picks the place:

| File | Content |
|---|---|
| `report.md`, `report.json` | The summary: failing sources, empty results, unreadable hosts |
| `suspects.jsonl` | Searches that look wrong: no results, every source failed, low top score, most pages unreadable, slow |
| `candidates.jsonl` | The top ten results of each query, with `grade` empty for you to fill |

## Judge and score

1. Open `candidates.jsonl` and fill each result's `grade` with 0 to 3.
2. Convert it to an evaluation file:

   ```bash
   tools/auto/judge.py candidates.jsonl >> tools/search-eval/live-judgments.jsonl
   ```

3. Score the current setup against it:

   ```bash
   seesea eval tools/search-eval/live-judgments.jsonl -k 10 -m standard
   ```

   This prints nDCG, recall and MRR.

Each line of an evaluation file is a query with its relevant URLs and their grades.

## Build, test and evaluate in one run

```bash
tools/auto/run.sh
```

It builds, tests, evaluates offline, probes the sources and writes a report, all under `runs/<time>/`, and compares with the previous run.

## Tell the ranking a result was good or bad

```bash
seesea feedback https://example.org/a bad
```

Over HTTP it is `POST /v1/feedback`. One piece of feedback counts as 5 observations in the first ranking layer.

## See what the first layer learned

```bash
seesea learn show --top 15    # best and worst hosts, and every source
seesea learn reset            # clear it
```

To stop the first layer learning, pass `--no-learn`; `eval` does not learn on its own.

## Judge cache policies from a log

```bash
seesea search --log s.jsonl "query"          # run a number of different queries
seesea replay s.jsonl --ttl 600              # hit ratio, and how far cached results differ from real ones
```

`replay` compares fuzzy thresholds 0.9, 0.8 and 0.7 by default. Look at its cost before turning on `--fuzzy-cache`.
