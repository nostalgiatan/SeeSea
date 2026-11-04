# Connect an MCP client

seesea offers three MCP tools: `search`, `extract` and `sources`. Their parameters are in the [MCP reference](../reference/mcp.md).

## Start over standard input and output

Most MCP clients configure servers in a file with an `mcpServers` section. Have the client start seesea as a child process:

```json
{
  "mcpServers": {
    "seesea": {
      "command": "/absolute/path/seesea",
      "args": ["mcp"]
    }
  }
}
```

If you need SearXNG, a proxy or models, set them in the config file, or pass environment variables in `env`:

```json
{
  "mcpServers": {
    "seesea": {
      "command": "/absolute/path/seesea",
      "args": ["mcp"],
      "env": {"SEESEA_SEARXNG_URL": "http://localhost:8888"}
    }
  }
}
```

## Connect over HTTP

`seesea serve` offers the same tools at `POST /mcp`. When the server has keys on, the client must send `Authorization: Bearer <key>` with each request.

```bash
seesea serve --addr 127.0.0.1:8080
```

This suits several clients sharing one server, since the models load only once.

## Check that it works

Have the client call the `sources` tool. If the list matches `seesea sources`, the connection works.
