# Own evaluation set (Chinese and multilingual first)

Purpose: measure extraction on the pages SeeSea will actually see, with block-level labels made
for this project, independent of WebMainBench and WCXB conventions.

## Target composition (about 300 pages)

- Chinese 50%, English 25%, other 25% (Japanese, Korean, Russian, Arabic, German/French/Spanish).
- Page types: article/news, blog, forum/Q&A, product, listing/category, documentation, service/landing.
- At least 3 pages per site for 20+ sites, so the same-site template signal can be tested later.
- Include 20+ pages that need JavaScript rendering, saved both as plain HTTP HTML and as rendered HTML.

## Workflow

1. Save pages as `<id>.html` in one directory; add `manifest.tsv` (`id<TAB>url<TAB>page_type`).
2. `extract-eval prelabel <dir>` writes `<id>.labels.json` with the model's guess for every block.
3. A person reviews each file, flips wrong `label` values, then sets `"reviewed": true`.
   Unreviewed files are ignored.
4. `extract-eval crate-eval ... --own <dir>` reports F1 on the reviewed pages.

Rules for the reviewer: label a block `true` if a reader would call it part of the page's main
content (article body, thread posts, product description, documentation text); `false` for
navigation, ads, related links, comment boxes, footers, cookie banners. When unsure, follow the
`Core` convention (body only).

## Caveats

- Pre-labels come from the model under test, so an unreviewed or lightly reviewed set scores the
  model against itself (a first dry run scored 1.000 for exactly that reason). Review every block
  of every page, and have a second person re-check about 10%.
- Never commit page HTML whose licence does not allow redistribution; keep the set outside the
  repository and commit only the manifest and label files if the URLs are stable.
- If segmentation changes, `load` refuses stale label files rather than misaligning them.
