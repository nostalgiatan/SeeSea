# Notice

The signals in this crate follow the detection approach of [url.vet](https://github.com/urlvet/urlvet)
(AGPL-3.0, Copyright (C) 2021-2026 Abhishek K P), and the files in `data/` were derived from its
constant tables: URL shorteners, trusted and risky top-level domains, hosts that give each customer a
subdomain, path keywords and the brand list. The code is a new implementation in Rust. SeeSea is
licensed under the same AGPL-3.0, so the notices are kept here.

The URL shortener table itself comes from
[PeterDaveHello/url-shorteners](https://github.com/PeterDaveHello/url-shorteners) and
[korlabsio/urlshortener](https://github.com/korlabsio/urlshortener) through url.vet.

Popularity ranks come from the [Tranco list](https://tranco-list.eu/) (cite the list ID that
`seesea vet update` prints). Threat lists come from the feeds named in the configuration; see
`docs/reference/config.md` for the terms of each.

`data/risky_tlds.txt` differs from url.vet's table: two-letter country endings are removed except
the free ones that attackers favour (tk, ml, ga, cf, gq, pw, cc, ws), because sites in China,
Russia and many other countries live on their own endings and a search service has to serve them.
