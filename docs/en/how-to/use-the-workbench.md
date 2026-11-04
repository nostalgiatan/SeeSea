# Use the workbench

The workbench is a web page built into `seesea`. In a browser you can search, read pages and watch the sources, and you can start the HTTP API and a remote MCP server from it.

## Open it

For a window and a tray icon instead of a browser tab, use the [desktop app](use-the-desktop-app.md).

```bash
seesea desk
```

The command opens a browser at `http://127.0.0.1:7878/`. Add `--no-open` to skip that. The workbench listens on loopback only and needs no key; it manages this machine.

`seesea serve` also serves the page at `/`, so you can look at a remote server. There is no Services or Accounts page then, and the page asks for an API key when it needs one. The key lives in the current tab only.

## Search and judge

1. Type a query, pick `Fast`, `Standard` or `Deep`, and press Enter.
2. The coloured bar at the top shows the time of each stage. Yellow lines are degradation notes, red lines are sources that failed.
3. Click Details to see how the score was made, which sections were read, and other reports of the same event.
4. Click Good or Bad to tell the first ranking layer about the host. This needs the root key; in a local workbench it works without one.

Keys: `/` focuses the search box, `j` and `k` select a result, `d` expands it, `g` and `b` judge it, `o` opens the link, `Ctrl K` opens the command palette.

## Review and grade

The Review page sums up the recent search journal: which sources failed, which hosts could not be read, which searches deserve another look (Search again returns to the search page with the query), and the host standings the first ranking layer has learned.

Below that, each query is listed with its top ten results. Click 0 to 3 on each one: 3 is exactly what was wanted, 2 answers it, 1 is related, 0 is not relevant. Click the chosen grade to take it back. Every click is written to the judgments file; the bottom of the page shows its path and the command to evaluate with:

```bash
seesea eval <judgments file>
```

After changing a ranking setting, evaluate with the same file again to compare the scores.

## Start the HTTP API

1. Open Services and pick an address and an authentication mode on the HTTP API card.
2. For use on this machine only, keep `127.0.0.1:8080` and "No key". For other machines, change the address to `0.0.0.0:8080`, pick a shared key or account authentication, and fill in a TLS certificate and key. If a reverse proxy in front ends TLS, tick that option instead.
3. Click Start. A `curl` command you can copy appears under the card.
4. Tick "Start with the workbench" to have it come up the next time you open the workbench.

An address that is not local is refused without TLS and a key, and the page says why.

## Start a remote MCP server

The Remote MCP card works the same way but serves only `/mcp`, nothing else of the API. Its default address is `127.0.0.1:8090`.

1. Pick "One shared key" and click Start. The key is generated for you; you can show it, copy it and replace it.
2. Under the card are a `claude mcp add` command for Claude Code and a general `mcp.json` snippet. Copy one into the client.
3. For a client on another machine, change the address to `0.0.0.0:8090` and set up TLS.

Changed settings take effect on "Apply and restart"; requests in flight finish first.

## Manage accounts

The Accounts page adds users, issues keys, disables users and revokes keys. A new key is shown once. These users matter when a service authenticates with "Keys from accounts" or "Either". A service set to accounts will not start while there are no users.

Settings are kept in `desk.json` in the data directory, readable by the current user only, since it may hold a shared key.
