# Run the HTTP server and issue keys

This page shows how to start `seesea serve`, create keys for users and open the server to others. The endpoints are in the [HTTP API reference](../reference/http-api.md).

## Start locally

```bash
seesea serve --addr 127.0.0.1:8080
```

On a loopback address it works without a key:

```bash
curl "http://127.0.0.1:8080/v1/search?q=rust+async&mode=fast"
```

## Require a key

Once a root key is set, every endpoint except `/health` needs it:

```bash
export SEESEA_API_KEY=a-long-random-string
seesea serve --addr 127.0.0.1:8080
curl -H "Authorization: Bearer $SEESEA_API_KEY" "http://127.0.0.1:8080/v1/search?q=rust"
```

The root key can also go in an `X-API-Key` header. It can call the admin endpoints and `/v1/ingest`.

## Give users keys

```bash
seesea users add ann --plan default
seesea keys create ann --name laptop --expires-days 90
```

The secret is shown once, in the form `ss_<id>_<secret>`; the database keeps only its SHA-256. The user sends it in place of the root key to search and to call the account endpoints:

```bash
curl -H "Authorization: Bearer ss_..." "http://127.0.0.1:8080/v1/account"
```

Common admin tasks:

| Goal | Command |
|---|---|
| Revoke a key | `seesea keys revoke ss_xxxxxxxx` |
| Disable a user (all their keys are refused) | `seesea users disable ann` |
| Change plan | `seesea users plan ann free` |
| See usage | `seesea usage ann --days 14` |

Users and keys live in the SQLite file named by `auth_db`. Several `serve` processes can share one; a revocation or a disable takes effect in all of them at once.

## Define plans

Add a plan to the config file and give it to a user:

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

## Open it to others

Listening on a non-loopback address needs TLS. There are two ways.

Let seesea terminate TLS:

```toml
tls_cert = "/etc/seesea/server.pem"
tls_key = "/etc/seesea/server.key"
```

Or put it behind a reverse proxy that already terminates TLS, and say so:

```toml
behind_tls_proxy = true
trust_forwarded_for = true
```

`trust_forwarded_for` makes seesea take the client address from `X-Forwarded-For`, which it uses to limit failed key attempts per address. Turn it on only when the proxy really sets that header.

To also require client certificates, add `tls_client_ca`.

## Let users manage their own keys

A user can create and revoke keys with their own key, at most 20 active keys each:

```bash
curl -X POST -H "Authorization: Bearer ss_..." -d '{"name":"ci","expires_days":30}' \
  http://127.0.0.1:8080/v1/account/keys
```

## Keep feeds fresh in the background

`serve` polls feeds every 15 minutes by default. Use `--poll-every 0` to turn that off or `--poll-every 1h` to change the interval. Feed setup is in [Fill the local index](fill-the-local-index.md).
