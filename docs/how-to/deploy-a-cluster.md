# 部署多节点集群

本页说明如何把 seesea 拆成网关、搜索节点和 flywheel 节点，并让两个 flywheel 加一个见证节点防止脑裂。各角色做什么、为什么这样设计，见[多节点设计](../explanation/cluster-design.md)。

默认的 `role = "single"` 一个进程做所有事，不需要这一页。

## 拓扑

| 角色 | 数量 | 作用 |
|---|---|---|
| `gateway` | 任意多个，放在负载均衡后面 | 面向用户的 HTTP；不保存账号 |
| `search` | 一个或多个 | 跑流水线和模型 |
| `flywheel` | 两个 | 保存账号、索引、站点评分和使用记录；同一时刻只有一个写 |
| `witness` | 一个 | 只投票，不存数据，不加载模型 |

节点之间走 UDP 端口 7443（QUIC）。网关对用户监听 HTTP。

## 1. 为每个节点生成身份

在任意机器上为每个节点各生成一对密钥，名字和角色按实际填写：

```bash
seesea cluster keygen fly1 --role flywheel --out keys
seesea cluster keygen fly2 --role flywheel --out keys
seesea cluster keygen wit  --role witness  --out keys
seesea cluster keygen s0   --role search   --out keys
seesea cluster keygen gw0  --role gateway  --out keys
```

每条命令写出 `keys/<名字>.pem`（证书）和 `keys/<名字>.key`（私钥），并打印一行：

```
<SHA-256 指纹> <角色> <名字>
```

把所有节点的这些行合在一个 `peers` 文件里，发给每个节点。私钥只留在它自己的节点上，权限设成只有属主可读，否则启动时会有警告。

## 2. 写每个节点的配置

所有节点共有的三项：

```toml
node_cert = "/etc/seesea/node.pem"
node_key = "/etc/seesea/node.key"
cluster_peers = "/etc/seesea/peers"
```

下面是各角色额外的配置，地址换成你的。

**flywheel（fly1，fly2 对称，`partner` 互指）**

```toml
role = "flywheel"
cluster_listen = "0.0.0.0:7443"
partner = "quic://10.0.0.5:7443"     # 另一个 flywheel
witness = "quic://10.0.0.6:7443"
```

两个 flywheel 的 `[plans]` 要写成一样的。

**witness**

```toml
role = "witness"
cluster_listen = "0.0.0.0:7443"
```

**search**

```toml
role = "search"
cluster_listen = "0.0.0.0:7443"
flywheel = "quic://10.0.0.3:7443"
flywheel_standby = "quic://10.0.0.5:7443"
```

**gateway**

```toml
role = "gateway"
search_nodes = ["quic://10.0.0.2:7443", "quic://10.0.0.4:7443"]
flywheel = "quic://10.0.0.3:7443"
flywheel_standby = "quic://10.0.0.5:7443"
```

网关对用户监听非回环地址，需要 TLS 或前置代理，见[运行 HTTP 服务](run-the-server.md#对外开放)。

## 3. 创建账号并启动

账号在 flywheel 上管理。在其中一个 flywheel 的机器上：

```bash
seesea users add ann
seesea keys create ann
```

另一个 flywheel 会在几秒内通过复制拿到账号。想让它启动时就有账号，可以先导出再导入：

```bash
seesea cluster export bundle              # 在第一个 flywheel 上
seesea cluster import bundle              # 在第二个 flywheel 上
```

在每台机器上启动，先起 witness 和 flywheel：

```bash
seesea serve --addr 127.0.0.1:8080
```

网关把 `--addr` 换成对外的地址。

## 4. 检查

```bash
curl -H "Authorization: Bearer ss_..." http://网关:8080/v1/cluster/status
```

网关的回复里 `search_nodes.healthy` 应等于搜索节点数。在两个 flywheel 上各查一次，应有且只有一个 `writer` 为 `true`。

## 日常操作

**滚动重启 flywheel。** 用 SIGTERM 或 Ctrl-C 正常停止，持有写租约的节点会主动交出租约，另一个几乎立刻接手。

**吊销一个节点。** 从所有节点的 `peers` 文件里删掉它的那一行。文件变化后自动重读，它已建立的连接在下一个请求时被关闭。

**把一个节点换成新机器。** 为新机器 `cluster keygen`，把新行加进所有节点的 `peers`，删掉旧行。

**故障转移之后。** 租约自动转移，旧 leader 恢复后变成跟随者，不需要改配置。意外宕机的转移窗口约 4 到 5 秒，期间请求会重试，多数只是变慢。

## 只有一主一备、没有见证节点

可以只给备用 flywheel 配 `standby_of = "quic://主:7443"`。这种方式不防脑裂：网络分区时两边都可能接受写入。备用在主不可达期间接受过写入时，主一恢复它就在日志里报警，并在 `/v1/cluster/status` 里标 `standby.diverged: true`。故障转移后需要手动把备当作新的主：旧主改配 `standby_of` 指向它，再把两边的 `flywheel` 和 `flywheel_standby` 对调。有条件时用上面的见证节点方案。
