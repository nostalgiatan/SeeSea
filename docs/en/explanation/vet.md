# How results are vetted

This page explains why SeeSea judges an address before it reads the page, what the judgement rests on, and why it would rather miss some bad sites than hurt honest ones. For the keys see [`[vet]`](../reference/config.md); for the steps see [Keep phishing out of results](../how-to/keep-phishing-out.md).

## The problem

We do not control how the upstream engines rank. A phishing page that one of them ranks high reaches us. For a search service used by AI agents that costs twice: the person sees a fake login page in the results, and the agent downloads the page, whose text may be addressed to the agent.

So the judgement has to come before the read, rest on the address alone, and be fast. A search has dozens of candidates and none of them may cost another network request.

## What it looks at

Everything is local.

**Popularity.** From the Tranco list of one million domains. The top 10 000 are *giants*, the top 100 000 *well known*, the rest of the list *listed*, and anything else *unranked*. On a well-known site `login`, `paypal` and `account` are ordinary content (paypal.com has a login page), so for the top 100 000 every suspicion based on names and paths is switched off.

**Official sites.** About five hundred famous brands with the domains they run themselves. An address on one of them gains and is marked `official`.

**Phishing signals**, all from the address:

- A famous domain written into a subdomain: `accounts.google.com.session-check.example`.
- A brand name in a domain that is not the brand's: `paypal-secure-login.example.com`, `binanceuz.co`.
- One or two letters away from a top-5000 name, or look-alike characters (`g00gle`).
- Mixed scripts in one name (Latin with Cyrillic), a raw IP as host, a domain before an `@`, link shorteners, random-looking names, abused top-level domains.

Two lists sit beside these: locally held threat lists (URLhaus by default, OpenPhish optional), where a match is dropped, and your own `allow` and `block`, which win over everything.

Every judgement carries reason codes, so any wrong call can be traced to the signal behind it.

## The score

It starts at 50, trust adds, suspicion subtracts, and the result lands between 0 and 100: 25 or less is `risky`, 42 or less `suspicious`, 65 or more `trusted`, the rest `neutral`. A threat-list or `block` match is `blocked`.

The effect on ranking is small and bounded:

| Case | Stage 1 | Final score |
|---|---|---|
| Official site (on top of its tier) | +0.06 | +0.03 |
| Giant | +0.06 | +0.03 |
| Well known | +0.04 | +0.02 |
| `suspicious` | -0.2 | -0.1 |
| `risky` | -0.6 | -0.3, and the page is not read |

The gains are small on purpose. Seven tenths of the final score is whether the page answers the question, so a popular site wins only when relevance is close and cannot beat a more relevant small one.

## Rules that protect honest sites

They come from testing on the real list. Every third domain of the Tranco million was judged as if it had come up in a search:

- Of 33 000 sites in the top 100 000, none was doubted.
- Of 300 000 listed sites, about 0.7% were flagged suspicious or risky. Spot checks showed mostly betting clones, adult sites and names like `google1102.com`.

What got it there:

- **A listed site needs a second reason.** The list also holds abused domains (`binanceuz.co` is on it) and honest small sites with unlucky names (`killedbygoogle.com`). Below the top 100 000, name-based signals count half unless another one points the same way: an abused ending, a random name, several login words.
- **A path alone never makes an address suspicious.** `login`, `verify` or a brand name in a path is common (a guide to resetting a Microsoft account). Path signals add at most 7 points, and a brand name is not itself a keyword.
- **A customer's site on a platform does not inherit the platform's rank.** `someone.github.io` and `someone.weebly.com` are their own sites and count as unranked; otherwise phishing pages hosted there would earn high marks. The platform's own address (`github.com`) keeps its rank.
- **Country endings are not a risk.** The upstream table lists `.cn` and `.ru`, which would flag honest Chinese and Russian sites. They are removed, leaving only free endings that attackers favour (`.tk`, `.ml`). The remaining new generic endings only corroborate: 7 points alone is not enough for `suspicious`.
- **Short names are not compared.** Names under six letters are one letter from many others; only names of ten or more may differ by two.

## How it feeds the flywheel

The verdict goes into the search journal: each result's level, tier and reason codes, and `score_parts.vet`. That gives later tuning three things:

- Which signal hurt a page that was read and found relevant. A `risky` page is not read, but a `suspicious` one is, and high relevance after reading is evidence of a false alarm.
- Whether `learned` (how the host delivered before) agrees with `vet`. A host that keeps delivering good content but is judged suspicious means a signal needs changing.
- Missed phishing sites, marked with `seesea feedback <url> bad`, whose traits can be added to a signal or to `block`.

`examples/calibrate` reruns the distribution over the whole list; use it after changing a signal or a weight to see whether the false-alarm rate moved.

## Limits

- It judges the address, not the page. A page that looks like phishing (forms, a copied title) can only be recognised after reading, which is not built yet.
- A freshly registered phishing domain is neither on the list nor on the feeds, so only the name signals apply. Feeds lag the first sighting by hours to days.
- The brand table leans toward the English-speaking world; Chinese brands are thinly covered. Use `block` to fill gaps.
- The default Tranco list includes Cloudflare Radar data, which is licensed for non-commercial use. A commercial service should use a custom list without it (put its ID in `rank_list`) or ask Tranco first. The threat feeds have their own terms: URLhaus is CC0, OpenPhish's free feed is non-commercial.
- The signals follow [url.vet](https://github.com/urlvet/urlvet) (AGPL-3.0) and the tables were converted from its own; see `seesea-core/crates/seesea-vet/NOTICE.md`.
