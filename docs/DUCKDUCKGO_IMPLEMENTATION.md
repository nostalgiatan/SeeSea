# DuckDuckGo 引擎实现对比

## 问题
为什么 SearXNG 可以解析 DuckDuckGo，而我的项目不可以？

## 关键差异分析

### 1. HTML 选择器精度

**SearXNG (Python)**:
```python
# Line 356: 只选择 web-result，排除广告
for div_result in eval_xpath(doc, '//div[@id="links"]/div[contains(@class, "web-result")]'):
```

**原实现 (Rust)**:
```rust
// 不够精确，可能选中广告
let result_selector = Selector::parse("div#links div[class*=\"web-result\"]")
```

**修正后 (Rust)**:
```rust
// 精确匹配，直接子元素
let result_selector = Selector::parse("div#links > div.web-result")
```

### 2. CAPTCHA 检测

**SearXNG (Python)** - Line 329-342:
```python
def is_ddg_captcha(dom):
    return bool(eval_xpath(dom, "//form[@id='challenge-form']"))

if is_ddg_captcha(doc):
    raise SearxEngineCaptchaException(suspended_time=0, ...)
```

**修正后 (Rust)**:
```rust
// 添加 CAPTCHA 检测
let captcha_selector = Selector::parse("form#challenge-form").expect("valid selector");
if document.select(&captcha_selector).next().is_some() {
    return Err("DDG CAPTCHA detected".into());
}
```

### 3. 表单数据顺序（关键！）

DuckDuckGo 的机器人检测对表单字段的**顺序非常敏感**。

**SearXNG (Python)** - Lines 267-308:
```python
# 严格的顺序
params['data']['q'] = query           # 1. 查询

if params['pageno'] == 1:
    params['data']['b'] = ""          # 2. 第一页标记
elif params['pageno'] >= 2:
    params['data']['s'] = offset      # 2. 偏移量
    params['data']['nextParams'] = '' # 3. 续页参数
    params['data']['v'] = 'l'         # 4. 版本
    params['data']['o'] = 'json'      # 5. 输出格式
    params['data']['dc'] = offset+1   # 6. 显示计数
    params['data']['api'] = 'd.js'    # 7. API 标识
    params['data']['vqd'] = vqd       # 8. VQD token

params['data']['kl'] = eng_region     # 9. 区域
params['data']['df'] = ''             # 10. 时间过滤
```

**修正后 (Rust)**:
```rust
// 严格按照 Python 的顺序
let mut form_data: Vec<(String, String)> = vec![
    ("q".to_string(), query.to_string())
];

if params.pageno == 1 {
    form_data.push(("b".to_string(), String::new()));
} else if params.pageno >= 2 {
    let offset = 10 + (params.pageno - 2) * 15;
    form_data.push(("s".to_string(), offset.to_string()));
    form_data.push(("nextParams".to_string(), String::new()));
    form_data.push(("v".to_string(), "l".to_string()));
    form_data.push(("o".to_string(), "json".to_string()));
    form_data.push(("dc".to_string(), (offset + 1).to_string()));
    form_data.push(("api".to_string(), "d.js".to_string()));
    // vqd would go here for page 2+
}

form_data.push(("kl".to_string(), region.to_string()));
form_data.push(("df".to_string(), df_value.to_string()));
```

### 4. 关键 HTTP Headers

**SearXNG (Python)** - Lines 313-318:
```python
params['headers']['Content-Type'] = 'application/x-www-form-urlencoded'
params['headers']['Referer'] = url
params['headers']['Sec-Fetch-Dest'] = "document"
params['headers']['Sec-Fetch-Mode'] = "navigate"  # ← 这个最关键！DDG 用于检测机器人
params['headers']['Sec-Fetch-Site'] = "same-origin"
params['headers']['Sec-Fetch-User'] = "?1"
```

**修正后 (Rust)** - 完全匹配:
```rust
params.headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
params.headers.insert("Referer".to_string(), "https://html.duckduckgo.com/html/".to_string());
params.headers.insert("Sec-Fetch-Dest".to_string(), "document".to_string());
params.headers.insert("Sec-Fetch-Mode".to_string(), "navigate".to_string()); // 关键！
params.headers.insert("Sec-Fetch-Site".to_string(), "same-origin".to_string());
params.headers.insert("Sec-Fetch-User".to_string(), "?1".to_string());
```

### 5. 内容提取选择器

**SearXNG (Python)** - Line 365-367:
```python
item["title"] = extract_text(title)
item["url"] = eval_xpath(div_result, './/h2/a/@href')[0]
item["content"] = extract_text(
    eval_xpath_getindex(div_result, './/a[contains(@class, "result__snippet")]', 0, [])
)
```

**修正后 (Rust)**:
```rust
// 标题
let title_selector = Selector::parse("h2 a").expect("valid selector");
let title = title_elem.text().collect::<String>().trim().to_string();

// URL
let url = title_elem.value().attr("href").unwrap_or("").to_string();

// 内容 - 精确选择器
let content_selector = Selector::parse("a.result__snippet")
    .or_else(|_| Selector::parse("a[class*=\"result__snippet\"]"))
    .expect("valid selector");
```

### 6. 中国区域特殊处理

**SearXNG (Python)** - Lines 291-295:
```python
if params['searxng_locale'].startswith("zh"):
    # Some locales (at least China) do not have a "next page" button 
    # and DDG will return a HTTP/2 403 Forbidden for a request of such a page.
    params["url"] = None
    return
```

**修正后 (Rust)**:
```rust
// Python: Some locales (at least China) does not support pagination
if region.starts_with("zh") && params.pageno > 1 {
    params.url = None;
    return Err("Chinese locale does not support pagination".into());
}
```

### 7. VQD Token（分页用）

**SearXNG (Python)** - Lines 68-123:
```python
def get_vqd(query: str, region: str, force_request: bool = False) -> str:
    """Returns the vqd that fits to the *query*.
    
    A request with a wrong vqd value leads to DDG temporarily 
    putting SearXNG's IP on a block list.
    """
    # 从缓存获取或请求新的 vqd
    resp = get(f'https://duckduckgo.com/?q={quote_plus(query)}')
    value = extr(resp.text, 'vqd="', '"')
    cache.set(key=key, value=value)
    return value
```

**Rust 实现** - 已有但未在 request() 中使用:
```rust
async fn get_vqd(&self, query: &str, region: &str) -> Result<String, ...> {
    // 检查缓存
    // 发送请求到 duckduckgo.com
    // 提取 vqd="..." 值
    // 缓存 1 小时
}
```

## 为什么 SearXNG 能成功解析 DuckDuckGo

### 主要原因:

1. **精确的 CSS 选择器**
   - 使用 `>` 直接子选择器
   - 只选择 `.web-result`，排除 `.result--ad`

2. **严格的表单字段顺序**
   - DDG 的机器人检测检查字段顺序
   - 必须完全按照 Python 代码的顺序

3. **关键的 Sec-Fetch-Mode header**
   - `Sec-Fetch-Mode: navigate` 是最重要的
   - DDG 用它来区分真实浏览器和机器人

4. **CAPTCHA 检测与处理**
   - 识别 CAPTCHA 页面
   - 避免持续发送请求被封 IP

5. **VQD Token 管理**
   - 第一页不需要
   - 分页必须有正确的 VQD
   - 错误的 VQD 会导致 IP 被临时封禁

6. **区域特殊处理**
   - 中国区域不支持分页
   - 避免 403 错误

## 实现清单

- [x] 精确的 HTML 选择器 (`div#links > div.web-result`)
- [x] CAPTCHA 检测 (`form#challenge-form`)
- [x] 严格的表单数据顺序
- [x] 完整的 Sec-Fetch headers
- [x] 中国区域分页限制
- [x] VQD token 提取和缓存机制（已实现但待集成）
- [x] 详细的代码注释对应 Python 实现

## 测试建议

1. **第一页搜索**: 应该能正常工作
2. **分页**: 需要实现 VQD 集成
3. **中国区域**: 只请求第一页
4. **CAPTCHA**: 检测并停止请求

## 参考

- SearXNG DuckDuckGo 引擎: `src/python/searx/engines/duckduckgo.py`
- 关键行号:
  - Lines 267-308: 表单数据构建
  - Lines 313-318: 关键 headers
  - Lines 356-368: 结果解析
  - Lines 68-123: VQD token 管理
