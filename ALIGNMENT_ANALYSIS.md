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

#### Request Format ✅
- [x] Base URL: `https://<subdomain>/search`
- [x] Parameters:
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
- [x] Cookies:
  - [x] `CONSENT`: "YES+"
- [x] Time range: `tbs=qdr:{d|w|m|y}`
- [x] Safe search: `safe={off|medium|high}`

#### Parsing Mode ✅
- [x] Check for CAPTCHA (sorry.google.com)
- [x] Use selector: `div[jscontroller*="SC7lYd"]`
- [x] Extract title from `a > h3`
- [x] Extract URL from `a[h3]/@href`
- [x] Extract content from `div[data-sncf*="1"]`
- [x] Extract thumbnails from img tags
- [x] Skip results without content
- [x] Suggestions extraction

#### Status: ✅ ALIGNED (Request format already correct, parsing updated)

### 2. Bing Engine

**Python Reference**: `src/python/searx/engines/bing.py`

#### Request Format ✅
- [x] Base URL: `https://www.bing.com/search`
- [x] Parameters:
  - [x] `q`: query
  - [x] `pq`: query (prevents pagination issues)
  - [x] `first`: pagination offset (formula: `(page-1)*10+1`)
  - [x] `FORM`: PERE/PERE1/PERE2/... (page 2+)
- [x] Cookies:
  - [x] `_EDGE_CD`: `m={region}&u={language}`
  - [x] `_EDGE_S`: `mkt={region}&ui={language}`
- [x] Time range: `filters=ex1:"ez{1|2|3|5_...}"`
- [x] Allow redirects: true

#### Parsing Mode ✅
- [x] Selector: `ol#b_results > li.b_algo`
- [x] Extract title from `h2 > a`
- [x] Extract URL from href attribute
- [x] Decode base64 URLs starting with `/ck/a?`
- [x] Extract content from `p` elements
- [x] Filter out "Web" text from content
- [ ] Extract result count from `span[@class="sb_count"]` (not critical)
- [ ] Validate pagination (check expected vs actual start) (not critical)

#### Status: ✅ ALIGNED (Core functionality complete)

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
