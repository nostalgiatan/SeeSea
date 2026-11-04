# 配置搜索来源

本页说明如何加入、限制和检查搜索来源。来源的限制和已知问题见[已知限制](../reference/known-limits.md#搜索来源)。

## 加入自建的 SearXNG

SearXNG 实例需要开启 JSON 输出格式。把地址写进环境变量或配置文件：

```bash
export SEESEA_SEARXNG_URL=http://localhost:8888
```

```toml
# ~/.config/seesea/config.toml
searxng = "http://localhost:8888"
```

## 加入 Brave Search

```bash
export SEESEA_BRAVE_API_KEY=你的密钥
```

或在配置文件里写 `brave_key = "..."`。

## 只用指定的来源

用 `-s` 列出来源名，逗号分隔。来源名可以用 `seesea sources` 查到：

```bash
seesea search -s duckduckgo,wikipedia "query"
```

Stack Overflow、arXiv 和 GitHub 默认不参与，需要这样点名：

```bash
seesea search -s stackoverflow,arxiv,github "query"
```

用 GitHub 来源时设置 `SEESEA_GITHUB_TOKEN` 可以提高限额。

## 关闭内置来源

只想用 SearXNG 或 Brave 时，加 `--no-native`，或在配置文件里写 `no_native = true`。

## 通过代理访问

所有请求（来源和页面抓取）都可以走一个代理，包括指向 Tor 的 SOCKS5：

```bash
seesea --proxy socks5://127.0.0.1:9050 search "query"
```

配置文件里的键是 `proxy`。

## 检查来源是否可用

```bash
seesea sources --check
```

每个来源会收到一个测试问题（用 `-q` 换问题），输出它是否返回结果、耗时，以及失败原因。`Blocked` 表示对方返回了验证码或限流。

## 限定或排除站点

按站点过滤结果，参数接受主机名或上级域名：

```bash
seesea search --include docs.rs,tokio.rs "tokio select"
seesea search --exclude pinterest.com "query"
```

## 搜索近期事件

加 `--topic news`，新闻来源（`bingnews`）会自动参与，时效在排序里的权重变大：

```bash
seesea search --topic news "query"
```
