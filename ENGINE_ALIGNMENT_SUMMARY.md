# Engine Alignment Summary

## Objective
Align all Rust search engine implementations with their corresponding Python SearxNG engines in terms of:
1. **Request Format**: URL parameters, headers, cookies, form data
2. **Parsing Mode**: Response parsing selectors, result extraction logic

## Completed Alignments

### 1. Bing Engine ✅
**Status**: Aligned with Python SearxNG

**Changes Made**:
- ✅ Added base64 dependency to Cargo.toml
- ✅ Implemented `decode_bing_url()` function to handle base64-encoded URLs from /ck/a? endpoints
- ✅ Updated HTML parsing to use exact Python selector: `ol#b_results > li.b_algo`
- ✅ Extract title from `h2 > a` (matching Python XPath: `.//h2/a`)
- ✅ Extract content from `p` elements, filtering out "Web" text
- ✅ URL decoding logic matches Python's base64 decoding

**Python Reference**: `src/python/searx/engines/bing.py`

### 2. Google Engine ✅ (Partially)
**Status**: Parsing aligned with Python SearxNG

**Changes Made**:
- ✅ Updated HTML parsing to use Python selector: `div[jscontroller*="SC7lYd"]`
- ✅ Extract title from `a > h3` (matching Python XPath: `.//a/h3[1]`)
- ✅ Extract URL from `a[h3]/@href` (links containing h3)
- ✅ Extract content from `div[data-sncf*="1"]` (matching Python XPath)
- ✅ Extract thumbnails from img tags
- ✅ Skip results without content (matching Python behavior)

**Request Format** (already implemented):
- ✅ Base URL format
- ✅ ARC async parameter generation (`ui_async()`)
- ✅ Query parameters: q, start, ie, oe, filter, hl, lr, cr, asearch, async
- ✅ Cookies: CONSENT=YES+
- ✅ Time range: tbs=qdr:{h|d|w|m|y}
- ✅ Safe search: safe={off|medium|high}

**Python Reference**: `src/python/searx/engines/google.py`

## Remaining Work

### 3. DuckDuckGo Engine ⚠️
**Status**: Complex - Requires significant refactoring

**Required Changes**:
- ❌ Implement VQD token mechanism (bot protection)
- ❌ Add caching system for VQD tokens (using sled or similar)
- ❌ Change from GET to POST request
- ❌ Implement form data structure:
  - q: query
  - b: "" (for page 1)
  - s: offset (for page 2+)
  - v: 'l'
  - o: 'json'
  - api: 'd.js'
  - vqd: token
  - kl: region
  - df: time filter
- ❌ Add special headers:
  - Content-Type: application/x-www-form-urlencoded
  - Referer, Sec-Fetch-* headers
- ❌ Handle pagination offset calculation: Page 2 = 10, Page 3+ = 10 + n*15
- ❌ Handle regions without "next page" (e.g., zh-* locales)

**Python Reference**: `src/python/searx/engines/duckduckgo.py`

### 4. Baidu Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/baidu.py`
- Identify request format and parameters
- Identify parsing selectors and logic
- Document differences with current Rust implementation

### 5. Brave Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/brave.py`
- Note: Supports categories (general, news, videos, images)
- Time range and paging support with limitations
- Document API structure and authentication if needed

### 6. Mojeek Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/mojeek.py`
- Note: Paging only supported for general search
- Document selectors and extraction logic

### 7. Qwant Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/qwant.py`
- Identify API structure
- Document request/response format

### 8. Startpage Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/startpage.py`
- Note: Complex region/language handling
- May require cache system for session tokens
- Supports categories: web, news, images

### 9. Yahoo Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/yahoo.py`
- Document request format
- Identify parsing selectors

### 10. Yandex Engine 📋
**Status**: Not yet analyzed

**Required Investigation**:
- Check Python implementation in `src/python/searx/engines/yandex.py`
- Document API structure
- Identify special requirements

## Implementation Priorities

### High Priority (Most Used)
1. ✅ Google - DONE (parsing aligned)
2. ✅ Bing - DONE
3. ⚠️ DuckDuckGo - Complex, requires VQD system

### Medium Priority
4. Brave - Growing popularity
5. Startpage - Privacy-focused
6. Qwant - European alternative

### Lower Priority
7. Baidu - Regional (China)
8. Yahoo - Less commonly used
9. Yandex - Regional (Russia)
10. Mojeek - Smaller engine

## Key Insights

### Common Patterns Across Engines

1. **Request Patterns**:
   - Most use GET requests with URL parameters
   - DuckDuckGo uses POST with form data
   - Headers often include: Accept, User-Agent, Referer
   - Cookies used for region/language preferences

2. **Parsing Patterns**:
   - Most use HTML parsing with CSS/XPath selectors
   - Common extraction: title, url, content (snippet), thumbnail
   - Need to handle CAPTCHA detection
   - Need to skip invalid/empty results

3. **Bot Protection**:
   - DuckDuckGo: VQD token system
   - Google: ARC async parameter
   - Bing: Cookie-based region/language
   - May need rate limiting

### Technical Debt

1. **Cache System**: Need a unified caching system for:
   - DuckDuckGo VQD tokens
   - Startpage session tokens
   - Rate limiting state

2. **Testing**: Need integration tests for each engine:
   - Mock responses for unit tests
   - Live API tests (optional, rate-limited)
   - Validate alignment with Python behavior

3. **Documentation**: Each engine needs:
   - Detailed comments matching Python implementation
   - API parameter documentation
   - Example usage

## Next Steps

1. **Immediate**:
   - Review and test Bing and Google alignments with actual queries
   - Document any edge cases or issues found

2. **Short-term**:
   - Analyze and align Brave, Startpage, Qwant engines
   - These are simpler than DuckDuckGo and widely used

3. **Medium-term**:
   - Implement DuckDuckGo VQD system and caching
   - This is the most complex but important change

4. **Long-term**:
   - Align remaining engines (Baidu, Yahoo, Yandex, Mojeek)
   - Add comprehensive test coverage
   - Performance optimization

## Success Criteria

An engine is considered "aligned" when:
- ✅ Request parameters match Python SearxNG exactly
- ✅ Headers and cookies match Python SearxNG
- ✅ Parsing selectors match Python SearxNG
- ✅ Result extraction logic produces equivalent data
- ✅ Error handling (CAPTCHA, rate limits) matches
- ✅ Edge cases (pagination, empty results) handled identically
