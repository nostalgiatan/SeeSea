# 接入 MCP 客户端

seesea 提供 `search`、`extract` 和 `sources` 三个 MCP 工具，参数见 [MCP 参考](../reference/mcp.md)。

## 用标准输入输出启动

大多数 MCP 客户端的配置文件用 `mcpServers` 这种格式。让客户端把 seesea 当作子进程启动：

```json
{
  "mcpServers": {
    "seesea": {
      "command": "/绝对路径/seesea",
      "args": ["mcp"]
    }
  }
}
```

需要 SearXNG、代理或模型时，通过配置文件设置，或在 `env` 里传环境变量：

```json
{
  "mcpServers": {
    "seesea": {
      "command": "/绝对路径/seesea",
      "args": ["mcp"],
      "env": {"SEESEA_SEARXNG_URL": "http://localhost:8888"}
    }
  }
}
```

## 通过 HTTP 接入

`seesea serve` 在 `POST /mcp` 上提供同样的工具。服务开启了密钥时，客户端要在请求头里带 `Authorization: Bearer <密钥>`。

```bash
seesea serve --addr 127.0.0.1:8080
```

这种方式适合多个客户端共用一个服务，模型只加载一次。

## 确认它在工作

让客户端调用 `sources` 工具。返回的来源列表和 `seesea sources` 的一致就说明连通了。
