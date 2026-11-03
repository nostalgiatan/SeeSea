# 运行 HTTP 服务并发放密钥

本页说明如何启动 `seesea serve`、为用户创建密钥，并对外开放。接口细节见 [HTTP 接口参考](../reference/http-api.md)。

## 本机启动

```bash
seesea serve --addr 127.0.0.1:8080
```

监听回环地址时，不设密钥也能使用：

```bash
curl "http://127.0.0.1:8080/v1/search?q=rust+async&mode=fast"
```

## 要求密钥

设置根密钥后，除 `/health` 以外的每个接口都要带它：

```bash
export SEESEA_API_KEY=一个足够长的随机字符串
seesea serve --addr 127.0.0.1:8080
curl -H "Authorization: Bearer $SEESEA_API_KEY" "http://127.0.0.1:8080/v1/search?q=rust"
```

根密钥也可以放在 `X-API-Key` 头里。它能调用管理接口和 `/v1/ingest`。

## 给用户发密钥

```bash
seesea users add ann --plan default
seesea keys create ann --name laptop --expires-days 90
```

密钥只显示一次，形如 `ss_<id>_<secret>`，库里只保存它的 SHA-256。用户用它代替根密钥调用搜索和账号接口：

```bash
curl -H "Authorization: Bearer ss_..." "http://127.0.0.1:8080/v1/account"
```

常用的管理操作：

| 目的 | 命令 |
|---|---|
| 吊销一把密钥 | `seesea keys revoke ss_xxxxxxxx` |
| 停用一个用户（拒绝他的所有密钥） | `seesea users disable ann` |
| 换套餐 | `seesea users plan ann free` |
| 看用量 | `seesea usage ann --days 14` |

用户和密钥保存在 `auth_db` 指定的 SQLite 文件里。多个 `serve` 进程可以共用一个，吊销和停用对所有进程立即生效。

## 定义套餐

在配置文件里加套餐，并让用户使用：

```toml
[plans.free]
per_minute = 20
burst = 10
per_day = 500
max_concurrent = 2
deep = false
```

```bash
seesea users plan ann free
```

## 对外开放

监听非回环地址需要 TLS。有两种做法。

直接让 seesea 终结 TLS：

```toml
tls_cert = "/etc/seesea/server.pem"
tls_key = "/etc/seesea/server.key"
```

或者放在已经终结 TLS 的反向代理后面，并声明这一点：

```toml
behind_tls_proxy = true
trust_forwarded_for = true
```

`trust_forwarded_for` 让 seesea 从 `X-Forwarded-For` 取客户端地址，用于按地址限制失败的密钥尝试。只有代理确实会设置这个头时才打开。

要求客户端证书时，再加 `tls_client_ca`。

## 让用户自己管理密钥

用户可以用自己的密钥创建和吊销密钥，每人最多 20 把有效密钥：

```bash
curl -X POST -H "Authorization: Bearer ss_..." -d '{"name":"ci","expires_days":30}' \
  http://127.0.0.1:8080/v1/account/keys
```

## 让订阅源在后台更新

`serve` 默认每 15 分钟拉取一次订阅源。用 `--poll-every 0` 关闭，或 `--poll-every 1h` 改间隔。订阅源的设置见[填充本地索引](fill-the-local-index.md)。
