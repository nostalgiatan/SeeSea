# China Mode - 中国模式

## 概述 (Overview)

China Mode (中国模式) 是 SeeSea 的特殊配置模式。根据实际测试，在中国大陆只有**9个搜索引擎**稳定可用。

**重要**: 本配置基于实际网络测试结果，只包含真正可用的引擎。

## 中国可用的搜索引擎 (9个)

### Bing 系列 (4个) - ✅ 可用
- **bing** - 通用搜索
- **bing images** - 图片搜索
- **bing news** - 新闻搜索
- **bing videos** - 视频搜索

### 360搜索 (2个) - ✅ 可用
- **360search** - 通用搜索 (中国本土)
- **360search videos** - 视频搜索

### 开发工具 (3个) - ✅ 可用
- **github** - 代码仓库 (可能不稳定)
- **github code** - GitHub代码搜索
- **stackoverflow** - 开发问答

## 在中国被墙的引擎 (已禁用)

以下引擎在中国被墙，已被禁用：

- ❌ **DuckDuckGo** - 被墙
- ❌ **Brave Search** - 被墙
- ❌ **Startpage** - 被墙
- ❌ **Wikipedia** - 被墙
- ❌ **Wikidata** - 被墙
- ❌ **Unsplash** - 被墙

## 全球模式引擎列表 (11个核心引擎)

在非中国地区，SeeSea 默认启用以下11个核心搜索引擎：

| 名称 | 类别 | 快捷码 | 中国可用 |
|------|------|--------|----------|
| Bing | 通用搜索 | `bi` | ✅ |
| DuckDuckGo | 通用搜索 | `ddg` | ❌ |
| Brave | 通用搜索 | `br` | ❌ |
| Startpage | 通用搜索 | `sp` | ❌ |
| 360搜索 | 通用搜索 | `360so` | ✅ |
| Wikipedia | 百科 | `wp` | ❌ |
| Wikidata | 知识库 | `wd` | ❌ |
| GitHub | 代码仓库 | `gh` | ✅ |
| Stack Overflow | 开发问答 | `st` | ✅ |
| Unsplash | 免费图库 | `us` | ❌ |

**说明**: 
- 全球模式: 24个引擎 (包括主引擎和images/news/videos变体)
- 中国模式: 9个引擎 (只保留可访问的引擎)

## 配置方式

### 启用中国模式

编辑 `config/default.toml`:

```toml
[general]
region_mode = "china"  # 可选: "global", "china", "custom"
```

### 中国模式配置

详见 `config/china_mode.toml`，包含：
- DNS 配置 (国内DNS优先)
- 网络优化 (延长超时、增加重试)
- 缓存设置 (2小时缓存)
- 引擎优先级

## 网络优化

### DNS 配置
使用国内 DNS 服务器:
```toml
preferred_dns = [
    "https://dns.alidns.com/dns-query",    # 阿里云
    "https://doh.pub/dns-query",            # 腾讯 DNSPod
    "https://doh.360.cn/dns-query",         # 360 DoH
]
```

### 超时设置
```toml
request_timeout = 45  # 秒
connect_timeout = 15  # 秒
max_retries = 4
```

### 缓存配置
```toml
enabled = true
ttl = 7200           # 2小时
max_size_mb = 2048
```

## 引擎优先级

```toml
"360search" = 1.5    # 中国本土引擎优先
bing = 1.0           # 国际引擎
github = 1.0
stackoverflow = 1.0
```

## 统计信息

- **全球模式**: 24个引擎
- **中国模式**: 9个引擎
- **被墙引擎**: 15个

## 使用建议

1. **中国用户**: 建议启用中国模式，只使用可访问的9个引擎
2. **国际用户**: 使用全球模式，享受完整的24个引擎
3. **GitHub访问**: 在中国可能不稳定，建议配置代理

## 注意事项

1. 网络环境随时可能变化
2. GitHub 在中国访问可能不稳定
3. 建议定期更新引擎配置
4. 遵守当地法律法规

## 更新日志

### v1.1.0 (2024-11-15)
- ✅ 重新调整为11个核心引擎
- ✅ 中国模式精简为9个真实可用引擎
- ✅ 移除不可用的引擎配置
- ✅ 更新文档说明

### v1.0.0 (2024-11-15)
- ✅ 初始发布 China Mode
- ✅ 网络优化配置
- ✅ DNS 和缓存设置


### 核心搜索引擎 (19个)

#### 主要搜索
- **Yandex** - 俄罗斯搜索引擎 (在中国可访问)
  - yandex (通用搜索)
  - yandex images (图片搜索)
  - yandex music (音乐搜索)

- **Bing** - 微软搜索 (在中国可访问)
  - bing (通用搜索)
  - bing images (图片搜索)
  - bing news (新闻搜索)
  - bing videos (视频搜索)

- **Baidu** - 百度 (中国最大搜索引擎)
  - baidu (通用搜索)
  - baidu images (图片搜索)
  - baidu kaifa (开发者搜索)

- **Sogou** - 搜狗 (中国搜索引擎)
  - sogou (通用搜索)
  - sogou images (图片搜索)
  - sogou videos (视频搜索)
  - sogou wechat (微信搜索)

- **360search** - 360搜索
  - 360search (通用搜索)
  - 360search videos (视频搜索)

- **ChinaSo** - 中国搜索
  - chinaso news (新闻搜索)
  - chinaso images (图片搜索)
  - chinaso videos (视频搜索)

### 学术研究平台 (8个)
- ArXiv - 论文预印本
- PubMed - 医学研究数据库
- Semantic Scholar - 学术搜索
- Crossref - DOI注册机构
- OpenAIRE Datasets - 开放科研数据
- OpenAIRE Publications - 开放科研出版物
- PDBe - 蛋白质数据库
- Astrophysics Data System - 天体物理学论文

### 开发技术平台 (7个)
- **GitHub** - 代码托管平台
- **GitHub Code** - GitHub代码搜索
- **Stack Overflow** - 编程问答社区
- **PyPI** - Python包索引
- **Docker Hub** - Docker镜像仓库
- **Crates.io** - Rust包仓库
- **MDN** - Web开发文档

### 中国视频平台 (2个)
- **Bilibili** - 哔哩哔哩
- **Acfun** - AcFun弹幕视频网

### 地图位置服务 (2个)
- **OpenStreetMap** - 开源地图
- **Photon** - 地理编码服务

### 工具服务 (6个)
- Currency Converter - 货币转换
- Wttr.in - 天气查询
- MyMemory Translated - 翻译服务
- DictZone - 词典
- Wordnik - 单词定义
- Etymonline - 词源查询

### 其他服务 (21个)
- Qwant 系列 (qwant, qwant images, qwant news, qwant videos)
- Ask Ubuntu, Super User
- Arch Linux Wiki, Alpine Linux Packages, Gentoo, Anaconda
- Hoogle (Haskell搜索), Mankier (Man页面)
- Anna's Archive - 学术资源
- ANSA, Reuters - 新闻
- APK Mirror - Android应用
- Bandcamp - 音乐
- Radio Browser - 网络电台
- TootFinder - Mastodon搜索
- Chefkoch - 菜谱
- Genius - 歌词

## 已排除的服务 (Blocked Services)

以下服务因在中国被防火墙屏蔽而**不包含**在中国模式中:

### 搜索引擎
- ❌ Google (及所有Google服务)
- ❌ DuckDuckGo
- ❌ Startpage
- ❌ Brave Search
- ❌ Yahoo

### 社交媒体
- ❌ Twitter
- ❌ Facebook
- ❌ Reddit
- ❌ Pinterest
- ❌ Mastodon
- ❌ Lemmy

### 视频/音频平台
- ❌ YouTube
- ❌ Vimeo
- ❌ Dailymotion
- ❌ SoundCloud
- ❌ Mixcloud
- ❌ Podcast Index

### 维基媒体项目
- ❌ Wikipedia
- ❌ Wikidata
- ❌ Wiktionary
- ❌ Wikinews
- ❌ Wikimedia Commons

### 图片平台
- ❌ Flickr
- ❌ DeviantArt
- ❌ Unsplash
- ❌ OpenVerse

### BT/种子网站
- ❌ The Pirate Bay
- ❌ KickAss Torrents
- ❌ 1337x
- ❌ BT4G
- ❌ Solid Torrents

### 其他
- ❌ Lingva Translate
- ❌ SepiaSearch

## 配置方式 (Configuration)

### 方法 1: 通过配置文件

编辑 `config/default.toml`:

```toml
[general]
# 设置区域模式为中国
region_mode = "china"
```

### 方法 2: 使用专用配置

SeeSea 提供了专门的 `config/china_mode.toml` 配置文件，包含详细的中国模式配置选项。

## 配置选项详解

### 区域设置
```toml
[china_mode.region_config]
location = "CN"
preferred_languages = ["zh", "zh-CN", "en"]
timezone = "Asia/Shanghai"
```

### DNS 配置
优先使用国内 DNS 服务器以提高解析速度和稳定性:

```toml
preferred_dns = [
    "https://dns.alidns.com/dns-query",      # 阿里云 DoH
    "https://doh.pub/dns-query",              # 腾讯 DNSPod DoH
    "https://doh.360.cn/dns-query",           # 360 DoH
]
```

### 网络优化
考虑到中国网络环境，延长超时时间并增加重试次数:

```toml
[china_mode.network]
request_timeout = 45  # 秒
connect_timeout = 15  # 秒
max_retries = 4
retry_strategy = "exponential_backoff"
```

### 缓存配置
建议启用缓存以提高性能和响应速度:

```toml
[china_mode.cache]
enabled = true
ttl = 7200  # 2小时
max_size_mb = 2048
```

## 引擎优先级 (Engine Priorities)

中国模式根据引擎的可靠性和相关性调整搜索结果优先级:

```toml
[china_mode.engine_priorities]
# 中国本土引擎 (优先级较高)
baidu = 1.5
"360search" = 1.4
sogou = 1.3
chinaso = 1.2
bilibili = 1.2
acfun = 1.1

# 国际可访问引擎
bing = 1.0
yandex = 1.0
qwant = 0.8

# 专业引擎
github = 1.2
stackoverflow = 1.1
arxiv = 1.1
pubmed = 1.1
```

## 统计信息 (Statistics)

- **SearXNG总引擎数**: 260+
- **中国可访问**: 65 个引擎 (真实验证)
- **已启用**: 65 个引擎
- **已排除被墙服务**: 45+ 个引擎

### 按类别分布
- 核心搜索: 19 个
- 学术研究: 8 个
- 开发技术: 7 个
- 中国视频: 2 个
- 地图位置: 2 个
- 工具服务: 6 个
- 其他服务: 21 个

## 使用建议 (Recommendations)

### 首次使用
1. 先测试主要搜索引擎 (Yandex, Bing, Baidu) 确保连接正常
2. 验证 DNS 配置是否正常工作
3. 检查缓存功能是否启用

### 性能优化
1. **启用缓存**: 显著提高响应速度，减少重复请求
2. **使用国内DNS**: 提高域名解析速度和成功率
3. **适当超时**: 如果网络不稳定，可以进一步增加超时时间

### 搜索技巧
1. 优先使用中国本土引擎 (Baidu, Sogou) 搜索中文内容
2. 学术资源使用 ArXiv, PubMed 等专业平台
3. 开发问题优先使用 GitHub, Stack Overflow
4. 需要国际视角时使用 Bing, Yandex

## 支持的搜索类别 (Supported Categories)

- 🔍 **通用搜索** (General) - Yandex, Bing, Baidu, Sogou, 360search
- 🖼️ **图片搜索** (Images) - Bing, Baidu, Yandex, Sogou
- 🎬 **视频搜索** (Videos) - Bilibili, Acfun, Bing, Sogou
- 📰 **新闻搜索** (News) - Bing, ChinaSo, Reuters, ANSA
- 📚 **学术搜索** (Academic) - ArXiv, PubMed, Semantic Scholar
- 💻 **代码搜索** (Code) - GitHub, Stack Overflow
- 🗺️ **地图搜索** (Maps) - OpenStreetMap
- 🎵 **音乐搜索** (Music) - Yandex Music, Bandcamp
- 🌐 **翻译服务** (Translation) - MyMemory
- 📖 **词典查询** (Dictionary) - DictZone, Wordnik

## 注意事项 (Important Notes)

### 网络限制
1. 部分国际服务可能因网络波动出现间歇性访问问题
2. GitHub 在某些地区和时段可能访问较慢
3. 建议配置合适的超时和重试参数

### 内容限制
1. 搜索结果受中国网络环境影响
2. 某些敏感内容可能无法获取
3. 建议使用多个引擎进行交叉验证

### 合规性
1. 使用搜索引擎需遵守当地法律法规
2. 尊重各搜索引擎的使用条款
3. 不得用于非法用途

## 故障排查 (Troubleshooting)

### 无法连接某个引擎
1. 检查网络连接是否正常
2. 验证 DNS 解析是否成功
3. 查看引擎是否在维护
4. 检查超时设置是否过短

### 搜索结果质量差
1. 尝试使用不同的搜索引擎
2. 调整搜索关键词
3. 使用专业引擎搜索特定领域内容

### 性能问题
1. 检查缓存是否正常工作
2. 减少并发引擎数量
3. 使用国内 DNS 服务器
4. 适当增加超时时间

## 技术支持 (Technical Support)

遇到问题时，请检查:
- 网络连接状态
- DNS 配置
- 引擎启用状态
- 日志文件中的错误信息
- 防火墙和代理设置

## 更新日志 (Changelog)

### v1.0.0 (2024-11-15)
- ✅ 初始发布 China Mode
- ✅ 验证并启用 65 个真实可访问的搜索引擎
- ✅ 排除所有被墙服务 (Google, Wikipedia, YouTube等)
- ✅ 添加核心中国搜索引擎 (Yandex, Bing, Baidu, Sogou, ChinaSo, 360search)
- ✅ 支持中国视频平台 (Bilibili, Acfun)
- ✅ 优化网络配置 (超时、重试、DNS)
- ✅ 添加引擎优先级配置
- ✅ 添加国内 DNS 服务器支持

## 贡献 (Contributing)

如果您发现:
- 某个引擎在中国已被墙但仍在列表中
- 某个引擎在中国可访问但未包含
- 配置需要优化

欢迎提交 Issue 或 Pull Request！

## 许可证 (License)

本项目遵循 MIT License
