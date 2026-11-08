# Keep phishing out of results

Goal: have phishing and look-alike sites dropped, demoted or at least flagged in search results, and official sites kept near the top. For the reasoning see [How results are vetted](../explanation/vet.md).

## Turn it on

It is on by default. To give it the popularity list and the threat lists, update the data once:

```bash
seesea vet update
seesea vet status
```

`update` downloads the Tranco list (about 20 MB) and the Phishing.Database list of active phishing hosts (about 11 MB) into the `vet/` folder of the data directory. A part that fails keeps its old file. Restart `seesea serve` to load the new data.

## Update every day

The threat lists change daily. Refresh them from a scheduled job; a cron line:

```bash
17 4 * * * /usr/local/bin/seesea vet update >> /var/log/seesea-vet.log 2>&1
```

## See why an address was judged as it was

```bash
seesea vet check https://accounts.google.com.session-check.example/signin https://www.paypal.com/signin
```

It prints each address's level, popularity tier and every reason. The `vet.flags` of a search result hold the same reason codes.

## Trust or block a site

In the config file:

```toml
[vet]
allow = ["docs.example.org"]      # always trusted, wins over the threat lists
block = ["content-farm.example"]  # never shown
```

Both match the host and all its subdomains.

## Flag only, do not drop

To watch before deciding, only flag:

```toml
[vet]
drop_listed = false
skip_risky = false
boost = 0
```

Results still carry the `vet` field and mark suspicious addresses, but the order does not change and pages are read as before. Switch the keys back when you have seen enough.

## Add threat feeds

`feeds` takes the built-in names or the address of a plain list (one address or host per line):

```toml
[vet]
feeds = ["phishingdb", "openphish", "https://example.org/my-blocklist.txt"]
```

For a commercial service check each feed's terms first; see [Limits](../explanation/vet.md#limits).

## Check for false alarms

The search journal (the `journal/` folder of the data directory) records each result's `vet`: level, tier and reason codes. Look for results at level `suspicious` whose page was read and scored high on relevance. Those are false alarms: add the site to `allow` and report the example so the signal can be adjusted.
