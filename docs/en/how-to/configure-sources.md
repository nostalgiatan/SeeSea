# Configure search sources

This page shows how to add, limit and check search sources. Their limits and known problems are in [Known limits](../reference/known-limits.md#search-sources).

## Add your own SearXNG

The SearXNG instance needs its JSON output format turned on. Put the address in an environment variable or the config file:

```bash
export SEESEA_SEARXNG_URL=http://localhost:8888
```

```toml
# ~/.config/seesea/config.toml
searxng = "http://localhost:8888"
```

## Add Brave Search

```bash
export SEESEA_BRAVE_API_KEY=your-key
```

Or write `brave_key = "..."` in the config file.

## Use only some sources

Name them with `-s`, comma separated. `seesea sources` lists the names:

```bash
seesea search -s duckduckgo,wikipedia "query"
```

Stack Overflow, arXiv and GitHub are off by default and must be named:

```bash
seesea search -s stackoverflow,arxiv,github "query"
```

Set `SEESEA_GITHUB_TOKEN` to raise the GitHub rate limit.

## Turn off the built-in sources

To use only SearXNG or Brave, pass `--no-native` or write `no_native = true` in the config file.

## Go through a proxy

Every request, to sources and to pages, can go through a proxy, including a SOCKS5 proxy pointing at Tor:

```bash
seesea --proxy socks5://127.0.0.1:9050 search "query"
```

The config key is `proxy`.

## Check that sources work

```bash
seesea sources --check
```

Each source gets a test question (change it with `-q`). The output says whether it returned results, how long it took, and why it failed if it did. `Blocked` means it answered with a captcha or a rate limit.

## Include or exclude sites

Filter results by site; the options take a host or a parent domain:

```bash
seesea search --include docs.rs,tokio.rs "tokio select"
seesea search --exclude pinterest.com "query"
```

## Search recent events

Add `--topic news`. The news source (`bingnews`) joins automatically and freshness counts for more in ranking:

```bash
seesea search --topic news "query"
```
