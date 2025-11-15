# 引擎绝对一致性修复指南

## 修复优先级和步骤

### P0-1: DuckDuckGo - quote_ddg_bangs函数

**Python实现 (lines 226-237)**:
```python
def quote_ddg_bangs(query):
    query_parts = []
    for val in re.split(r'(\s+)', query):
        if not val.strip():
            continue
        if val.startswith('!') and external_bang.get_node(external_bang.EXTERNAL_BANGS, val[1:]):
            val = f"'{val}'"
        query_parts.append(val)
    return ' '.join(query_parts)
```

**Rust修复**:
```rust
// 在DuckDuckGoEngine中添加
fn quote_ddg_bangs(query: &str) -> String {
    use regex::Regex;
    
    let re = Regex::new(r"(\s+)").unwrap();
    let parts: Vec<&str> = re.split(query).collect();
    let mut query_parts = Vec::new();
    
    for val in parts {
        if val.trim().is_empty() {
            continue;
        }
        
        let mut processed_val = val.to_string();
        
        // 检查是否是!bang命令
        // 注意: 完整实现需要external_bang支持，这里先做简化
        // 实际应该检查external_bang.EXTERNAL_BANGS
        if val.starts_with('!') {
            processed_val = format!("'{}'", val);
        }
        
        query_parts.push(processed_val);
    }
    
    query_parts.join(" ")
}

// 在request()函数开头调用
fn request(&self, query: &str, params: &mut RequestParams) -> Result<(), ...> {
    let query = Self::quote_ddg_bangs(query);
    
    if query.len() >= 500 {
        params.url = None;
        return Err("Query too long (max 499 chars)".into());
    }
    // ...
}
```

### P0-2: DuckDuckGo - form_data处理

**问题**: form_data在分页时需要从上一页响应中获取

**Python实现 (lines 274-278)**:
```python
params['data']['nextParams'] = form_data.get('nextParams', '')
params['data']['v'] = form_data.get('v', 'l')
params['data']['o'] = form_data.get('o', 'json')
params['data']['api'] = form_data.get('api', 'd.js')
```

**Rust修复思路**:
1. 在response()解析时提取并存储form_data
2. 在SearchEngine中添加状态存储
3. 或在RequestParams中添加form_data字段

**简化方案** (当前):
```rust
// 使用默认值，因为从响应中提取form_data需要状态管理
form_data.push(("nextParams".to_string(), String::new()));
form_data.push(("v".to_string(), "l".to_string()));
form_data.push(("o".to_string(), "json".to_string()));
form_data.push(("api".to_string(), "d.js".to_string()));
```

**完整方案**:
```rust
// 1. 在response()中提取form_data
fn response(&self, resp: Self::Response) -> Result<Vec<SearchResultItem>, ...> {
    // Python: form = eval_xpath(doc, '//input[@name="vqd"]/..')
    let form_selector = Selector::parse("form input[name='vqd']").unwrap();
    
    if let Some(form_elem) = document.select(&form_selector).next() {
        if let Some(parent) = form_elem.parent() {
            // 提取 nextParams, v, o, api 等字段
            // 需要状态管理机制存储这些值
        }
    }
    
    // ...
}
```

### P0-3: Wikipedia - display_type逻辑

**Python实现 (lines 77, 191-206)**:
```python
display_type = ["infobox"]

# 在response中
if "list" in display_type or api_result.get('type') != 'standard':
    results.append({'url': wikipedia_link, 'title': title, 'content': api_result.get('description', '')})

if "infobox" in display_type:
    if api_result.get('type') == 'standard':
        results.append({
            'infobox': title,
            'id': wikipedia_link,
            'content': api_result.get('extract', ''),
            'img_src': api_result.get('thumbnail', {}).get('source'),
            'urls': [{'title': 'Wikipedia', 'url': wikipedia_link}],
        })
```

**Rust修复**:
```rust
// 在WikipediaEngine结构体中添加
pub struct WikipediaEngine {
    info: EngineInfo,
    client: HttpClient,
    display_type: Vec<String>,  // 新增
}

impl WikipediaEngine {
    pub fn new() -> Self {
        Self {
            // ...
            display_type: vec!["infobox".to_string()],  // 默认值
        }
    }
    
    fn parse_json_result(&self, json_str: &str) -> Result<Vec<SearchResultItem>, ...> {
        let api_result: Value = serde_json::from_str(json_str)?;
        let mut items = Vec::new();
        
        let title = api_result.get("titles")
            .and_then(|t| t.get("display"))
            .and_then(|d| d.as_str())
            .or_else(|| api_result.get("title").and_then(|t| t.as_str()))
            .unwrap_or("");
            
        // Python: utils.html_to_text() 处理
        let title = Self::html_to_text(title);
        
        let wikipedia_link = api_result.get("content_urls")
            .and_then(|c| c.get("desktop"))
            .and_then(|d| d.get("page"))
            .and_then(|p| p.as_str())
            .unwrap_or("");
        
        let article_type = api_result.get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        
        // Python: if "list" in display_type or api_result.get('type') != 'standard':
        if self.display_type.contains(&"list".to_string()) || article_type != "standard" {
            items.push(SearchResultItem {
                title: title.clone(),
                url: wikipedia_link.to_string(),
                content: api_result.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string(),
                // ...
            });
        }
        
        // Python: if "infobox" in display_type:
        if self.display_type.contains(&"infobox".to_string()) {
            if article_type == "standard" {
                // 创建infobox结果
                let mut metadata = HashMap::new();
                metadata.insert("infobox".to_string(), title.clone());
                metadata.insert("id".to_string(), wikipedia_link.to_string());
                
                if let Some(extract) = api_result.get("extract").and_then(|e| e.as_str()) {
                    metadata.insert("extract".to_string(), extract.to_string());
                }
                
                if let Some(img_src) = api_result.get("thumbnail")
                    .and_then(|t| t.get("source"))
                    .and_then(|s| s.as_str()) {
                    metadata.insert("img_src".to_string(), img_src.to_string());
                }
                
                metadata.insert("urls".to_string(), 
                    format!("{{\"title\": \"Wikipedia\", \"url\": \"{}\"}}", wikipedia_link));
                
                items.push(SearchResultItem {
                    title: title.clone(),
                    url: wikipedia_link.to_string(),
                    content: api_result.get("extract")
                        .and_then(|e| e.as_str())
                        .unwrap_or("")
                        .to_string(),
                    metadata,
                    template: Some("infobox".to_string()),
                    // ...
                });
            }
        }
        
        Ok(items)
    }
}
```

### P0-4: Wikipedia - html_to_text工具函数

**Python使用**: `utils.html_to_text()`

**Rust实现**:
```rust
impl WikipediaEngine {
    /// HTML to text conversion (matches Python's utils.html_to_text)
    fn html_to_text(html: &str) -> String {
        use html_escape::decode_html_entities;
        
        // 1. 解码HTML实体
        let decoded = decode_html_entities(html);
        
        // 2. 移除HTML标签
        let re = regex::Regex::new(r"<[^>]*>").unwrap();
        let text = re.replace_all(&decoded, "");
        
        // 3. 规范化空白字符
        let re = regex::Regex::new(r"\s+").unwrap();
        let normalized = re.replace_all(&text, " ");
        
        normalized.trim().to_string()
    }
}
```

### P1-1: GitHub - 日期解析

**Python实现 (line 55)**:
```python
from dateutil import parser
'publishedDate': parser.parse(item.get("updated_at") or item.get("created_at")),
```

**Rust修复**:
```rust
use chrono::{DateTime, Utc};

// 在parse_json_result中
if let Some(date_str) = item.get("updated_at")
    .and_then(|u| u.as_str())
    .or_else(|| item.get("created_at").and_then(|c| c.as_str())) {
    
    if let Ok(parsed_date) = DateTime::parse_from_rfc3339(date_str) {
        // 存储为ISO 8601格式或时间戳
        metadata.insert("published_date".to_string(), parsed_date.to_rfc3339());
    }
}
```

### P1-2: Stack Overflow - 完整HTML unescape

**当前实现**: 仅处理基本实体

**完整实现**:
```rust
use html_escape::decode_html_entities;

fn html_unescape(text: &str) -> String {
    // 使用html_escape crate处理所有HTML实体
    decode_html_entities(text).to_string()
}
```

或使用更完整的实现:
```rust
fn html_unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&nbsp;", " ")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
        // 添加更多实体...
        // 或使用正则表达式处理数字实体
        // &#(\d+); -> 对应的字符
}
```

### P2-1: Unsplash - 完整URL解析

**Python实现**:
```python
from urllib.parse import urlparse, parse_qsl, urlencode, urlunparse

def clean_url(url):
    parsed = urlparse(url)
    query = [(k, v) for (k, v) in parse_qsl(parsed.query) if k != 'ixid']
    return urlunparse((parsed.scheme, parsed.netloc, parsed.path, 
                       parsed.params, urlencode(query), parsed.fragment))
```

**Rust完整实现**:
```rust
use url::Url;

fn clean_url(url_str: &str) -> String {
    // 使用url crate进行完整解析
    if let Ok(mut url) = Url::parse(url_str) {
        // 过滤掉ixid参数
        let filtered_params: Vec<(String, String)> = url.query_pairs()
            .filter(|(k, _)| k != "ixid")
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        
        // 重建查询字符串
        url.set_query(None);
        for (k, v) in filtered_params {
            url.query_pairs_mut().append_pair(&k, &v);
        }
        
        url.to_string()
    } else {
        url_str.to_string()
    }
}
```

## 修复验证清单

每个修复完成后需要验证:

1. ✅ 编译通过
2. ✅ 类型安全
3. ✅ 与Python行为完全一致
4. ✅ 边缘情况处理
5. ✅ 错误处理正确
6. ✅ 添加测试用例

## 依赖添加

可能需要添加的crates:
```toml
[dependencies]
regex = "1"
html-escape = "0.2"
chrono = "0.4"
url = "2"
```

## 下一步

按优先级逐个实现这些修复，确保每个细节都与SearXNG **绝对一致**。
