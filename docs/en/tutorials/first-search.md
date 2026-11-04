# Tutorial: your first search

This tutorial takes you from nothing to a working search and a page read. By the end you will have seen what a result looks like and where each score comes from. It takes about 20 minutes. You need a machine with internet access and a Rust toolchain.

## 1. Build

From the repository root:

```bash
cd seesea-core
cargo build --release -p seesea-cli
```

When the build finishes, the executable is `target/release/seesea`. The commands below assume you are in the `seesea-core` directory and run it as `./target/release/seesea`.

## 2. A search on snippets only

Search once in the fastest mode:

```bash
./target/release/seesea search -m fast -n 5 "rust async runtime"
```

A title line shows the query, the number of results and the time taken. Below it are five numbered results. Each has a title, a URL, and a line like `score 0.62 · found by duckduckgo#1`: the score, and which source ranked it at which position.

`fast` mode merges the snippets the sources return and downloads no pages, so it is quick.

## 3. A search that reads page text

Drop `-m fast` to use the default `standard` mode:

```bash
./target/release/seesea search -n 5 "rust async runtime"
```

This time seesea downloads the top pages, extracts their text and reranks by that text. Each result now has a few paragraphs under it: the sections of the page that best answer the query. It takes longer than the previous step, usually over ten seconds, depending on the slowest page.

## 4. See how a score is made up

Add `--json` and the same search prints JSON. Save it:

```bash
./target/release/seesea search --json -n 5 "rust async runtime" > result.json
```

Open `result.json` and find the first item in `hits`. Its `score_parts` object lists the pieces of the score. `fusion`, `snippet_overlap`, `title` and `freshness` belong to the first layer, which sees only snippets. `relevance`, `title_support` and `body_quality` belong to the second layer, which has read the page. `found_by` records which sources returned the URL.

At the end of the file, `stages` gives the time of each stage, and `degraded` says which parts of this search did not work normally, for example a blocked source.

## 5. Read one page

Pick any URL from `result.json` and have seesea read only that page, choosing the sections that match a question:

```bash
./target/release/seesea extract https://tokio.rs/tokio/tutorial/select -q "cancellation" --max-chars 1500
```

The output is the part of the page about `cancellation`, already converted to Markdown, with the navigation and footer removed.

## 6. Check the sources

Last, see which sources are working:

```bash
./target/release/seesea sources --check
```

Each source gets a test question. The command prints whether it returned results, how long it took, and the reason for any failure. A captcha block shows up as `Blocked`.

## Next

- To add your own SearXNG or Brave: [Configure search sources](../how-to/configure-sources.md)
- To improve ranking with local models: [Set up local models](../how-to/set-up-models.md)
- To learn what happens inside a search: [How the search pipeline works](../explanation/pipeline.md)
