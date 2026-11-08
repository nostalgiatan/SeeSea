# 把钓鱼站挡在结果之外

目标：让搜索结果里的钓鱼站和仿冒站被剔除、降权或至少被标出来，并保持官方站点靠前。判断原理见[结果可信度判断](../explanation/vet.md)。

## 开启

默认就是开的。要让它用上热门度榜单和威胁列表，先更新一次数据：

```bash
seesea vet update
seesea vet status
```

`update` 下载 Tranco 榜单（约 20 MB）和 URLhaus 的在线地址列表，保存在数据目录的 `vet/` 下。某一项失败时旧文件保留。重启 `seesea serve` 后生效。

## 每天更新

威胁列表每天都在变，用定时任务刷新。cron 例子：

```bash
17 4 * * * /usr/local/bin/seesea vet update >> /var/log/seesea-vet.log 2>&1
```

## 看一个网址为什么被这样判断

```bash
seesea vet check https://accounts.google.com.session-check.example/signin https://www.paypal.com/signin
```

输出每个网址的等级、热门度档位和每条原因。搜索结果的 `vet.flags` 里是同一批原因代码。

## 信任或屏蔽某个站点

在配置文件里写：

```toml
[vet]
allow = ["docs.example.org"]      # 一直信任，优先于威胁列表
block = ["content-farm.example"]  # 一律不显示
```

`allow` 和 `block` 匹配主机和它的所有子域名。

## 只标记，不剔除

想先观察再决定，可以只标记：

```toml
[vet]
drop_listed = false
skip_risky = false
boost = 0
```

结果仍带 `vet` 字段，标出哪些网址可疑，但排序不变、页面照常读取。观察够了再把它们改回来。

## 增加威胁订阅源

`feeds` 可以写内置名字或一个纯网址列表的地址（每行一个网址或主机名）：

```toml
[vet]
feeds = ["urlhaus", "openphish", "https://example.org/my-blocklist.txt"]
```

商用服务先确认各订阅源的条款，见[结果可信度判断](../explanation/vet.md#限制)。

## 检查有没有误伤

搜索日志（数据目录的 `journal/`）里记了每个结果的 `vet`，包括等级、档位和原因代码。在里面找等级是 `suspicious`、但页面读完后相关度很高的结果，它们就是误伤：把站点加进 `allow`，并把例子反馈给项目，信号需要调整。
