# Engine Alignment Analysis: Rust vs Python SearxNG

This document tracks the alignment status of each Rust search engine implementation with its corresponding Python SearxNG engine.

## Alignment Criteria

For each engine, we need to ensure:

1. **Request Format Alignment**
   - URL parameters match exactly
   - Headers match exactly
   - Cookies match exactly
   - Form data (if POST) matches exactly
   - Time range format matches
   - Safe search format matches
   - Language/region format matches

2. **Parsing Mode Alignment**
   - HTML/JSON selectors match
   - Result extraction logic matches
   - Thumbnail extraction (if applicable)
   - Metadata extraction matches

3. **Engine Metadata Alignment**
   - `about` information matches
   - `categories` matches
   - `paging` behavior matches
   - `max_page` matches
   - `time_range_support` matches
   - `safesearch` matches

## Engine-by-Engine Status

### 1. Google Engine

**Python Reference**: `src/python/searx/engines/google.py`

#### Request Format
- [ ] Base URL: `https://<subdomain>/search`
- [ ] Parameters:
  - [x] `q`: query
  - [x] `start`: pagination offset
  - [x] `ie`: utf8
  - [x] `oe`: utf8
  - [x] `filter`: 0
  - [x] `hl`: interface language (format: `{lang}-{country}`)
  - [x] `lr`: language restriction (format: `lang_{code}`)
  - [x] `cr`: country restriction (format: `country{CODE}`)
  - [x] `asearch`: arc
  - [x] `async`: arc_id format
- [ ] Cookies:
  - [x] `CONSENT`: "YES+"
- [ ] Time range: `tbs=qdr:{d|w|m|y}`
- [ ] Safe search: `safe={off|medium|high}`

#### Parsing Mode
- [ ] Check for CAPTCHA (sorry.google.com)
- [ ] Detect AJAX response vs HTML response
- [ ] Parse AJAX JSON array format
- [ ] Fallback to HTML parsing
- [ ] Extract: title, url, content, thumbnail
- [ ] Suggestions extraction

#### Issues Found
1. ✅ `ui_async()` function implemented - arc_id generation
2. ⚠️ Need to verify exact AJAX response parsing
3. ⚠️ Need to verify subdomain selection logic
4. ⚠️ Need to verify language/region parameter format

### 2. Bing Engine

**Python Reference**: `src/python/searx/engines/bing.py`

#### Request Format
- [ ] Base URL: `https://www.bing.com/search`
- [ ] Parameters:
  - [x] `q`: query
  - [x] `pq`: query (prevents pagination issues)
  - [x] `first`: pagination offset (formula: `(page-1)*10+1`)
  - [ ] `FORM`: PERE/PERE1/PERE2/... (page 2+)
- [ ] Cookies:
  - [ ] `_EDGE_CD`: `m={region}&u={language}`
  - [ ] `_EDGE_S`: `mkt={region}&ui={language}`
- [ ] Time range: `filters=ex1:"ez{1|2|3|5_...}"`
- [ ] Allow redirects: true

#### Parsing Mode
- [ ] Selector: `//ol[@id="b_results"]/li[contains(@class, "b_algo")]`
- [ ] Extract: title (`h2/a`), url (href, may need base64 decode), content (`p`)
- [ ] Remove `span[@class="algoSlug_icon"]` from content
- [ ] Decode base64 URLs starting with `/ck/a?`
- [ ] Extract result count from `span[@class="sb_count"]`
- [ ] Validate pagination (check expected vs actual start)

#### Issues Found
1. ⚠️ FORM parameter not correctly implemented for page > 2
2. ⚠️ Base64 URL decoding not implemented
3. ⚠️ Result count extraction not implemented
4. ⚠️ Pagination validation not implemented

### 3. DuckDuckGo Engine

**Python Reference**: `src/python/searx/engines/duckduckgo.py`

#### Request Format
- [ ] Base URL: `https://html.duckduckgo.com/html/` (POST)
- [ ] Form data:
  - [ ] `q`: query
  - [ ] `v`: 'l'
  - [ ] `api`: 'd.js'
  - [ ] `o`: 'json'
  - [ ] `s`: pagination offset
  - [ ] `df`: time range
  - [ ] `vqd`: bot protection token (required for page 2+)
- [ ] Time range: `{d|w|m|y}`
- [ ] VQD token: Must be cached and retrieved from initial request

#### Parsing Mode
- [ ] Extract vqd from initial request
- [ ] Parse HTML results
- [ ] Handle CAPTCHA detection
- [ ] Extract: title, url, content

#### Issues Found
1. ❌ VQD token mechanism not implemented
2. ❌ Cache system for VQD not implemented
3. ❌ POST request not used
4. ❌ Form data structure incorrect

### 4. Baidu Engine

**Python Reference**: `src/python/searx/engines/baidu.py`

#### Status
- [ ] Not yet analyzed

### 5. Brave Engine

**Python Reference**: `src/python/searx/engines/brave.py`

#### Status
- [ ] Not yet analyzed

### 6. Mojeek Engine

**Python Reference**: `src/python/searx/engines/mojeek.py`

#### Status
- [ ] Not yet analyzed

### 7. Qwant Engine

**Python Reference**: `src/python/searx/engines/qwant.py`

#### Status
- [ ] Not yet analyzed

### 8. Startpage Engine

**Python Reference**: `src/python/searx/engines/startpage.py`

#### Status
- [ ] Not yet analyzed

### 9. Yahoo Engine

**Python Reference**: `src/python/searx/engines/yahoo.py`

#### Status
- [ ] Not yet analyzed

### 10. Yandex Engine

**Python Reference**: `src/python/searx/engines/yandex.py`

#### Status
- [ ] Not yet analyzed

## Priority Order

1. **High Priority**: Google, Bing, DuckDuckGo (most commonly used)
2. **Medium Priority**: Brave, Startpage, Qwant
3. **Low Priority**: Baidu, Yahoo, Yandex, Mojeek

## Implementation Strategy

1. For each engine, create a detailed comparison document
2. Update Rust implementation to match Python exactly
3. Add unit tests that verify alignment
4. Test with actual queries to ensure results match

## Notes

- All engines should use the `RequestResponseEngine` trait
- All request parameters should match Python SearxNG exactly
- All parsing logic should extract the same fields
- Error handling should be consistent
