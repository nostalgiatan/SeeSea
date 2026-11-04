# 工作台

`seesea desk` 的参数、服务设置和它的内部接口。操作步骤见[使用工作台](../how-to/use-the-workbench.md)。

## 命令

| 参数 | 默认 | 说明 |
|---|---|---|
| `--addr` | `127.0.0.1:7878` | 工作台自己的监听地址，只接受回环地址 |
| `--no-open` | 关 | 不自动打开浏览器 |
| `--poll-every` | `15m` | 后台拉取订阅源的间隔；`0` 关闭 |

工作台持有和 `serve` 相同的管线、索引和模型，用本机的账号数据库。它自己不要密钥，并沿用本机服务的保护：`Host` 必须是本机名字，POST 必须声明 `Content-Type: application/json`。

## 页面

每个服务器在 `/` 提供这个页面，页面和 `/ui/` 下的文件不需要密钥，其余数据都通过需要密钥的接口取得。响应带 `Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'`，所以页面不会加载或发送到别处。

工作台页面分三层：左侧是分组导航，顶部是位置、命令框和三个开关（语言、明暗、密钥），主区是内容。搜索页在宽屏上右侧有检查面板，显示选中结果的分数构成、读到的章节和相关报道。服务页每个服务有三个标签：设置、控制台（向运行中的服务发一个请求并看返回）、请求日志（每 3 秒刷新）。账号页列出套餐的限额，每个用户有今日额度条、近 7 天用量和密钥表，新建密钥可选有效期，停用和吊销会先确认。右键菜单按点到的对象给出操作，输入框里仍是浏览器自带的菜单。

“管理”组有两页。索引页显示本地索引的大小，可以现在拉取订阅源、补算向量、清理旧文章、添加起步订阅源，并列出订阅源（添加、按站点发现、重新启用、移除）。设置页编辑配置文件里的常用项（来源与网络、缓存、模型与渲染器、排序、数据与索引），顶部列出模型和 Lightpanda 是否就绪，缺的可以直接下载。保存只改动你改过的键，文件里的注释和其他键保持原样；多数改动在重新打开工作台后生效。集群和 TLS 相关的键不在表单里，仍在文件中编辑。

在桌面版里窗口没有系统标题栏，页面顶部的栏就是标题栏：空白处可拖动，双击最大化，右端是最小化、最大化和关闭（关闭是收进托盘）。

## 服务设置

两个服务：`api`（完整的 JSON 接口，`/mcp` 也在其中）和 `mcp`（只有 `/health` 和 `/mcp`）。

| 字段 | 默认 | 说明 |
|---|---|---|
| `addr` | `api` 为 `127.0.0.1:8080`，`mcp` 为 `127.0.0.1:8090` | 监听地址；端口填 0 由系统选 |
| `auth` | `open` | `open` 无密钥；`key` 一个共享密钥；`accounts` 账号数据库里的密钥；`both` 两者皆可 |
| `root_key` | 自动生成 | `key` 和 `both` 用；至少 16 个字符，以 `root-` 开头的自动值 |
| `tls_cert`、`tls_key` | 无 | PEM 文件路径，两个一起填 |
| `behind_tls_proxy` | 关 | 前面有终止 TLS 的反向代理 |
| `allow_open` | 关 | 没有密钥时仍监听非回环地址 |
| `max_concurrent` | 64 | 1 到 4096 |
| `autostart` | 关 | 随工作台启动 |

监听规则同 `serve`：非回环地址需要 TLS 或 `behind_tls_proxy`，也需要密钥或 `allow_open`。`accounts` 在没有用户时拒绝启动。设置存在数据目录的 `desk.json`，权限 `0600`。

## 标注文件

打分记在数据目录的 `grades.json`（查询、网址、0 到 3）。同时重写 `judgments.jsonl`，格式是 `seesea eval` 读的那种：每行一个查询和它的相关结果，只含大于 0 的等级。

## 内部接口

只在工作台进程里存在，不属于 [HTTP 接口](http-api.md)。

| 接口 | 说明 |
|---|---|
| `GET /desk/api/state` | 版本、数据目录、两个服务的设置和运行状态（`running`、`addr`、`tls`、`problem`）、套餐名 |
| `POST /desk/api/services/{id}/config` | 用请求体替换设置，下次启动生效；返回新状态 |
| `POST /desk/api/services/{id}/start` | 启动；已在运行返回 400 |
| `POST /desk/api/services/{id}/stop` | 停止，先处理完在途请求 |
| `POST /desk/api/services/{id}/restart` | 停止再启动 |
| `POST /desk/api/services/{id}/rotate` | 换一个新的共享密钥 |
| `GET /desk/api/users` | 用户、密钥和今日用量，以及套餐名 |
| `POST /desk/api/users` | 建用户，体 `{"name", "plan"}`，返回 201 |
| `POST /desk/api/users/{name}/keys` | 建密钥，体 `{"name", "expires_days"}`，返回 201 和只出现这一次的 `key` |
| `POST /desk/api/users/{name}/disabled` | 体 `{"disabled": true}` |
| `POST /desk/api/keys/{id}/revoke` | 吊销密钥，返回 204 |
| `GET /desk/api/services/{id}/log` | 服务最近的请求：`summary`（总数、2xx、4xx、5xx、运行秒数）和最近 100 条 `recent`（时间、方法、路径、状态、毫秒、账号）。不含查询参数和密钥；服务没运行返回 404 |
| `POST /desk/api/services/{id}/try` | 从工作台向运行中的服务发一个请求，体 `{"method", "path", "body", "auth", "key"}`。`method` 是 GET 或 POST，`path` 以 `/v1/` 开头或是 `/mcp`、`/health`，`auth` 是 `none`、`shared` 或 `key`。返回 `{"status", "ms", "headers", "body", "truncated"}`，正文最多 64 KB |
| `POST /desk/api/users/{name}/plan` | 体 `{"plan"}`，换套餐 |
| `GET /desk/api/settings` | 配置文件路径、其中的值（`values`，不含 `brave_key`，它是否已设置在 `secrets_set` 里）和各默认位置 |
| `PUT /desk/api/settings` | 体是 `{"键": 值}`，值为 `null` 或空文本表示删除，点号表示小节（`models.dir`）。只接受表单里的键，写入前按配置格式校验；返回新的 `GET` 内容 |
| `GET /desk/api/models` | 模型和 Lightpanda 的状态，正在下载时带已下载字节数 |
| `POST /desk/api/models/{name}/download` | `name` 是 `embedding`、`reranker` 或 `renderer`；后台下载，返回 202 |
| `GET /desk/api/index` | 本地索引的统计、全部订阅源和后台任务的结果；索引关闭时 `enabled` 为 false |
| `POST /desk/api/index/{action}` | `add`、`remove`、`revive`（体 `{"url"}`），`discover`（体 `{"site"}`），`seed`，`prune`，以及后台运行的 `poll` 和 `embed` |
| `GET /desk/api/sources`、`POST /desk/api/sources/check` | 来源名单；逐个探测（后台运行，结果在 `check` 里） |

| `GET /desk/api/review?since=7d` | 日志汇总（`report`：搜索数、缓存、耗时、来源、读不到的主机、可疑搜索、待打分的结果，结果里带已有的 `grade`）、第一层学到的主机评分（`learn`）和标注文件情况（`judgments`）。`since` 是 `1h` 到 `366d` 的小时或天数 |
| `POST /desk/api/grade` | 体 `{"query", "url", "grade"}`，`grade` 为 0 到 3，`null` 取消；返回 `{"graded", "queries"}` |

`{id}` 是 `api` 或 `mcp`。无法应用的设置返回 400，错误码 `cannot_apply`。
