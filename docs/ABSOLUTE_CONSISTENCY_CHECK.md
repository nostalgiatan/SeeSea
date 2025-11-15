# 引擎绝对一致性检查报告

## 检查方法
逐行对照SearXNG Python代码，确保每个细节都完全一致。

## DuckDuckGo 缺失细节

### 1. quote_ddg_bangs 函数
**Python (lines 226-237)**:
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
**状态**: ❌ 缺失
**影响**: DDG的!bang命令可能无法正确处理

### 2. request 第一行调用
**Python (line 241)**:
```python
def request(query, params):
    query = quote_ddg_bangs(query)
```
**状态**: ❌ 缺失

### 3. eng_region获取
**Python (line 248)**:
```python
eng_region: str = traits.get_region(params['searxng_locale'], traits.all_locale)
```
**状态**: ❌ 简化为默认值

### 4. form_data 获取 (分页时)
**Python (lines 274-278)**:
```python
params['data']['nextParams'] = form_data.get('nextParams', '')
params['data']['v'] = form_data.get('v', 'l')
params['data']['o'] = form_data.get('o', 'json')
params['data']['api'] = form_data.get('api', 'd.js')
```
**状态**: ❌ form_data 未定义，使用硬编码值

## Wikipedia 缺失细节

### 1. display_type 变量
**Python (line 77)**:
```python
display_type = ["infobox"]
```
**状态**: ❌ 缺失
**影响**: 无法正确控制返回结果类型

### 2. 条件返回逻辑
**Python (lines 191-206)**:
```python
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
**状态**: ❌ 简化为单一结果
**影响**: 可能无法正确显示infobox

### 3. params设置
**Python (lines 161-162)**:
```python
params['raise_for_httperror'] = False
params['soft_max_redirects'] = 2
```
**状态**: ❌ 缺失
**影响**: 错误处理可能不一致

### 4. utils.html_to_text
**Python (line 188)**:
```python
title = utils.html_to_text(api_result.get('titles', {}).get('display') or api_result.get('title'))
```
**状态**: ❌ 未使用html_to_text处理
**影响**: HTML实体可能未正确转义

## GitHub 缺失细节

### 1. dateutil.parser
**Python (line 55)**:
```python
'publishedDate': parser.parse(item.get("updated_at") or item.get("created_at")),
```
**状态**: ❌ 仅存储字符串，未解析为日期
**影响**: 日期字段类型不一致

## Stack Overflow 缺失细节

### 1. HTML实体转义完整性
**当前实现**: 仅处理基本实体
**Python使用**: html.unescape (处理所有HTML实体)
**状态**: ⚠️ 部分实现
**影响**: 某些特殊字符可能无法正确显示

## Unsplash 缺失细节

### 1. URL清理的完整性
**Python使用**: urlparse + urlunparse 完整URL解析
**当前实现**: 简化的字符串处理
**状态**: ⚠️ 可能有边缘情况

## 360search 检查结果

✅ **完全一致** - 未发现缺失细节

## Wikidata 检查结果

⚠️ **使用不同API** - 这是有意的简化，但需要明确标注

## 关键问题总结

### 高优先级 (P0)
1. DuckDuckGo: quote_ddg_bangs 函数缺失
2. DuckDuckGo: form_data 未定义
3. Wikipedia: display_type 逻辑缺失
4. Wikipedia: html_to_text 未使用

### 中优先级 (P1)
5. Wikipedia: params设置缺失
6. GitHub: 日期解析缺失
7. Stack Overflow: HTML unescape 不完整

### 低优先级 (P2)
8. Unsplash: URL解析简化
9. DuckDuckGo: region获取简化

## 修复计划

### 第一阶段: 关键功能
1. 修复 DuckDuckGo 的 quote_ddg_bangs
2. 修复 Wikipedia 的 display_type 逻辑
3. 添加 html_to_text 工具函数

### 第二阶段: 细节完善
4. 完善 HTML unescape
5. 添加日期解析
6. 完善错误处理

### 第三阶段: 边缘情况
7. 完善 URL 解析
8. 添加 region 处理
