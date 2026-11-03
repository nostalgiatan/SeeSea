# Fill the local index

The local index keeps the title, about 400 characters of opening, the date and the host of fresh articles, and no body text. Every search asks it as the `local` source, so with an index you can still find recent items when the network is slow. Why it is designed this way is in [Freshness and the local index](../explanation/freshness-and-index.md).

## Subscribe to RSS and Atom

Add the built-in English and Chinese starter feeds:

```bash
seesea feeds seed
```

These feeds start on probation, and bad ones are switched off automatically. Some addresses may not be reachable from your network; `seesea feeds list` shows each feed's last error.

Add your own feed:

```bash
seesea feeds add https://example.org/feed.xml
```

Have seesea find the feeds a site offers:

```bash
seesea feeds discover example.org --dry-run   # only list
seesea feeds discover example.org             # register
```

Fetch once, or keep fetching:

```bash
seesea poll
seesea poll --every 15m
```

`seesea serve` polls every 15 minutes in the background, so you do not need a separate `poll` while the server runs.

Handle a feed that has trouble:

| Goal | Command |
|---|---|
| See its state | `seesea feeds list` |
| Remove it | `seesea feeds remove <url>` |
| Turn a switched-off feed back on | `seesea feeds revive <url>` |

## Push articles yourself

Push articles to a running server, with the root key, up to 1000 a call:

```bash
curl -X POST http://127.0.0.1:8080/v1/ingest \
  -H "Authorization: Bearer $SEESEA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"items":[{"url":"https://example.org/a","title":"Title","lead":"Opening","published":1767225600,"source":"my-pusher"}]}'
```

## Make vectors for semantic search

Once items have vectors, an English question can find a Chinese report. Each poll makes vectors for 256 items. To fill in the rest now:

```bash
seesea index embed
```

To see which items are closest to a question in vector space, with the distance in bits:

```bash
seesea index near "refrigerators hacked in supermarkets" -k 5
```

This needs the embedding model; see [Set up local models](set-up-models.md).

## Find similar pages

While the server runs, find content close in meaning to a page or a piece of text:

```bash
curl -X POST http://127.0.0.1:8080/v1/similar \
  -H "Content-Type: application/json" \
  -d '{"url":"https://example.org/a","limit":5}'
```

## Check the size and apply retention

```bash
seesea index          # item count, days, size
seesea index prune    # apply retention now
```

The default is 30 days and a 512 MB cap; when over, the oldest day goes first. Adjust with `retention_days` and `index_quota_mb` in the config file.

## Receive WebSub pushes

When a feed names a WebSub hub, give the server an address the hub can reach:

```bash
seesea serve --public-url https://seesea.example.com
```

The server subscribes to the hub, receives pushes at `/v1/websub/{id}` and renews before expiry. Polling stays as the fallback. Only `https` hubs are subscribed to, and a hub at a private address, or a name that resolves to one, is skipped.
