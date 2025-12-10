# SeeSea Web 前端

SeeSea 搜索引擎的现代化 Web 前端，基于 SvelteKit + Tailwind CSS 构建。

## ✨ 功能特性

- 🔍 **多引擎搜索** - 聚合搜索，支持 Pro 增强搜索（向量重排序）
- 🔥 **实时热榜** - 多平台热门内容聚合
- 📡 **RSS 订阅** - RSS 源内容获取和展示
- 📊 **系统统计** - 实时监控搜索性能和系统指标
- 📱 **响应式设计** - PC 和移动端独立适配
- 🎨 **现代化 UI** - 流畅动画，美观界面

## 🚀 快速开始

### 环境要求

- Node.js 18+
- pnpm (推荐)

### 安装依赖

```bash
pnpm install
```

### 配置环境变量

复制 `.env.example` 为 `.env`，并根据需要修改：

```bash
cp .env.example .env
```

主要配置项：

```bash
# SeeSea API 服务器地址
VITE_API_BASE_URL=http://127.0.0.1:8080
```

### 启动开发服务器

```bash
pnpm dev
```

访问 http://localhost:5173

### 构建生产版本

```bash
pnpm build
```

### 预览生产版本

```bash
pnpm preview
```

## 📁 项目结构

```
src/
├── lib/
│   ├── api/           # API 客户端
│   │   ├── client.ts  # API 请求封装
│   │   └── index.ts
│   ├── components/    # 通用组件
│   │   ├── SearchBox.svelte
│   │   ├── SearchResultCard.svelte
│   │   ├── HotTrendCard.svelte
│   │   ├── Navbar.svelte
│   │   ├── Loading.svelte
│   │   ├── ErrorMessage.svelte
│   │   ├── EmptyState.svelte
│   │   ├── StatCard.svelte
│   │   └── index.ts
│   └── index.ts
├── routes/
│   ├── +layout.svelte # 全局布局
│   ├── +page.svelte   # 首页
│   ├── search/        # 搜索页
│   ├── hot/           # 热榜页
│   ├── rss/           # RSS 页
│   └── stats/         # 统计页
└── app.html
```

## 🎨 组件说明

| 组件 | 说明 |
|------|------|
| `SearchBox` | 搜索输入框，支持加载状态和清除功能 |
| `SearchResultCard` | 搜索结果卡片，显示标题、描述、来源等 |
| `HotTrendCard` | 热榜卡片，展示单个平台的热门内容 |
| `Navbar` | 顶部导航栏，支持桌面端和移动端 |
| `Loading` | 加载动画，支持多种尺寸 |
| `ErrorMessage` | 错误提示，支持重试功能 |
| `EmptyState` | 空状态展示，支持多种图标 |
| `StatCard` | 统计数据卡片，支持多种颜色主题 |

## 🔗 API 端点

前端对接的主要 API 端点：

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/search` | GET/POST | 搜索 |
| `/api/pro/search` | GET/POST | Pro 增强搜索 |
| `/api/hot/all` | GET | 获取所有平台热榜 |
| `/api/hot/platforms` | GET | 获取支持的平台列表 |
| `/api/hot/{platform}` | GET | 获取指定平台热榜 |
| `/api/rss/feeds` | GET | RSS 源列表 |
| `/api/rss/fetch` | POST | 获取 RSS 内容 |
| `/api/stats` | GET | 统计信息 |
| `/api/metrics/realtime` | GET | 实时指标 |
| `/api/cache/stats` | GET | 缓存统计 |

## 🛠️ 技术栈

- **框架**: SvelteKit 2.x
- **UI**: Tailwind CSS 4.x
- **语言**: TypeScript 5.x
- **包管理**: pnpm
- **构建工具**: Vite 7.x

## 📝 开发说明

### 添加新页面

1. 在 `src/routes/` 下创建新目录
2. 添加 `+page.svelte` 文件
3. 在 `Navbar.svelte` 中添加导航链接

### 添加新组件

1. 在 `src/lib/components/` 下创建新的 `.svelte` 文件
2. 在 `src/lib/components/index.ts` 中导出

### 扩展 API 客户端

在 `src/lib/api/client.ts` 中添加新的 API 方法。

## 📄 许可证

AGPL-3.0

