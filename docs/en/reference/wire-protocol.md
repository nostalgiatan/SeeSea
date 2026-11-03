# Node protocol reference: seesea-wire/1

In a multi-node deployment, nodes talk to each other with `seesea-wire/1`, an application-layer protocol over QUIC. It is not HTTP. For the reasoning see [Cluster design](../explanation/cluster-design.md).

## Transport

| Item | Value |
|---|---|
| Transport | QUIC (UDP), quinn 0.11, rustls 0.23 |
| TLS | TLS 1.3 only |
| ALPN | `seesea/1`, which is also the version |
| Address form | `quic://host:port`, default port 7443 |
| Idle timeout | 10 seconds |
| Keep-alive | every 3 seconds |
| Concurrent streams per connection | 256 |
| Open connections per node | 512 |

A request uses one bidirectional stream: the client writes a request frame, the server writes a reply frame, and the stream ends.

## Frame format

```
tag:u8 | head_len:u32 | body_len:u32 | head | body
```

Lengths are big-endian. `head` is JSON (it may be empty); `body` is raw bytes. In a request frame `tag` is the operation code; in a reply frame it is the status code.

The head is limited to 64 KiB. The body limit depends on the operation, and replies are limited to 64 MiB:

| Operation | Request body limit |
|---|---|
| `Events` | 32 MiB |
| `LearnMerge` | 64 MiB |
| `Http` | 2 MiB |
| Others | 64 KiB |

The server reads a body in 256 KiB chunks, so announcing a huge length costs nothing by itself. Reading a request takes at most 30 seconds and handling it at most 125 seconds.

## Operation codes

| Code | Name | Purpose |
|---|---|---|
| 0 | `Ping` | Liveness check |
| 1 | `Events` | A search node pushes events (pages read, usage records, host-standing observations) to the flywheel |
| 2 | `LearnGet` | Read the merged host standings |
| 3 | `LearnMerge` | Merge host standings |
| 4 | `IndexSearch` | Query the local index on the flywheel |
| 5 | `Http` | A user request forwarded by a gateway |
| 6 | `Admit` | A gateway asks the flywheel to check a key and its limits |
| 7 | `Charge` | A gateway charges credits at the flywheel |
| 8 | `Refund` | Give credits back (when a concurrency limit refuses) |
| 9 | `Replicate` | A follower copies data from the holder: accounts, standings, index, journal |
| 10 | `Lease` | Votes and renewals for the write lease |
| 11 | `Nudge` | The holder tells a follower to copy accounts now |

`Charge` and `Refund` identify the user by name, because user ids can differ between flywheels.

## Status codes

| Name | Meaning |
|---|---|
| `Ok` | Success |
| `Error` | Failure; the head carries the reason |
| `Busy` | The node is at its concurrency limit and did nothing; the gateway tries the next node |
| `Forbidden` | The peer's role does not allow this operation |
| `NotLeader` | This node does not hold the write lease; the client tries the other flywheel |

## Identity and roles

Each node has its own key pair (`seesea cluster keygen`) and a self-signed certificate. In the handshake both sides must present a certificate whose SHA-256 fingerprint is in the other's `cluster_peers` list. The client also requires the peer's role to be the one it expects: a connection to a flywheel must reach a flywheel.

Permissions follow the role in the list, not what the peer claims:

| Peer role | Allowed operations |
|---|---|
| Any role | `Ping` |
| `gateway` | `Http`, `Admit`, `Charge`, `Refund` |
| `search` | `Events`, `LearnGet`, `IndexSearch` |
| `flywheel` | `Replicate`, `Lease`, `Nudge` |
| `operator` | `Events`, `LearnGet`, `LearnMerge` |
| `witness` | `Ping` only; the witness answers only a flywheel's `Lease` asks |

To revoke a node, delete its line from `cluster_peers`. The file is reread after a change (at most once a second) and a connection that is already open is closed at its next request.

## Delivering events

Events carry an `(epoch, seq)` number. The flywheel remembers, for each sender (by certificate fingerprint) and epoch, how far it has applied, and keeps that record in `applied.json` in its state directory. A resent batch is applied once.

How a search node sends events:

| Item | Value |
|---|---|
| Events per batch | 500 |
| In-memory queue limit | 20,000 events or 64 MB |
| Beyond that | Written to the `spool` directory, files of about 8 MB, 2 GB in total |
| After a restart | Sending continues from the spool |
| Still beyond | The oldest are dropped and counted |

A spool file is deleted only after every event in it has been acknowledged.

## Backpressure and retries

| Situation | Behaviour |
|---|---|
| A node at its concurrency limit | Replies `Busy` at once; the gateway tries the next search node, up to 3; when all are full it answers HTTP 503 with `Retry-After` |
| An unreachable node | Marked down for 10 seconds and skipped |
| `NotLeader` or no connection | `Failover` retries every 0.5 seconds, at most 5 times |
| A failed read request (GET) | The gateway retries on the next node |
| Overall deadline | 115 seconds for a search request, 10 seconds for a flywheel request |

A client stays on the last flywheel that answered. It does not switch back by itself when the old primary returns.

## Write lease

Two flywheels and a witness make three voters. Only the flywheel holding a majority of votes holds the write lease and handles reads and writes; the other replies `NotLeader`.

| Parameter | Value |
|---|---|
| A voter's promise | Not to vote for anyone else for 4 seconds |
| How long the holder counts itself the holder | 2.5 seconds |
| Renewal interval | 0.5 seconds |
| Timeout of one ask | 1 second |
| At start | The node with the larger fingerprint waits 2 seconds, so both do not grab at once |

On a clean shutdown the holder sends `{"release": true}` to hand the lease over.

A flywheel without the lease acts as a follower and copies data:

| Data | Period |
|---|---|
| Accounts | Every 5 seconds; before answering an account change, the holder waits up to 3 seconds for the follower to pull it (`Nudge`) |
| Host standings | Every minute |
| Search records (journal) | Every 30 seconds, deduplicated by source id |
| Index items | Every 10 minutes |

## Status

`GET /v1/cluster/status` returns the role and lease state; see the [HTTP API reference](http-api.md#cluster-status).
