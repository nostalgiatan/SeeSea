# 把单节点的数据迁到集群

单节点升级成集群时，已有的账号、站点评分、索引条目和搜索记录可以带过去。已发出的密钥在新节点上照常可用。

## 导出

在原来的单节点机器上：

```bash
seesea cluster export bundle
```

`bundle` 是一个文件夹，里面有账号（只含密钥哈希）、站点评分、索引条目和搜索记录。

## 导入到新 flywheel

把文件夹复制到 flywheel 的机器上：

```bash
seesea cluster import bundle
```

重复导入不会重复写入条目，只有站点评分的计数会每次累加。账号导入到 flywheel 上，因为网关不保存账号。

## 推给运行中的 flywheel

不方便复制文件夹时，可以从任意一台机器推过去：

```bash
seesea cluster push bundle --to quic://10.0.0.3:7443
```

推送使用本机配置的节点密钥，它必须在 flywheel 的 `peers` 文件里被标成 `operator`。生成这样的身份：

```bash
seesea cluster keygen admin --role operator --out keys
```

把打印出来的那一行加进 flywheel 的 `peers` 文件。`push` 推送索引条目、搜索记录和站点评分，不推送账号，账号用上面的 `import` 导入。
