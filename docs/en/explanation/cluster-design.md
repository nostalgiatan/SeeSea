# Cluster design

A single `seesea` process can do everything. When you need more throughput, higher availability or separated duties, you can split it into node types by duty. This page explains why it is split this way and what that costs. The steps are in [Deploy a cluster](../how-to/deploy-a-cluster.md), and the protocol is in the [node protocol reference](../reference/wire-protocol.md).

## Why four roles

The heaviest parts of a search are model inference and page downloads, and the state that must be kept in one place is accounts, host standings and the index. Separating the two lets the heavy part scale out while state lives on a few nodes.

| Role | Why it exists |
|---|---|
| Gateway | Faces users and scales with traffic; keeps no accounts, so you can add as many as you like |
| Search node | Runs the pipeline and models and stores nothing; when it breaks, replace the machine |
| Flywheel | Keeps the index, the merged host standings, every node's usage records and the accounts; the name comes from the data flywheel: use produces data, data improves ranking |
| Witness | Exists only to vote between the two flywheels; stores no data and uses almost no resources |

## Why its own protocol, on QUIC

Plaintext between nodes is forbidden. QUIC has TLS 1.3 built into every connection, so there is no encryption switch to forget to turn on. Each request takes its own stream, so a slow request does not block others, and QUIC handles loss recovery and connection migration. The application layer only needs to define frames, operations and permissions; that is `seesea-wire/1`.

Why not plain HTTP: what nodes exchange is event batches, replication data and lease votes, not resources. HTTP would mean designing paths, status codes and headers over again, and the only result is more places to get something wrong. The custom frame is a one-byte operation code, two lengths, a JSON head and a raw byte body.

## Identity: each node has its own key, nothing shared

An early version had all nodes share one cluster secret. Its flaw is that one leaked node means every node is leaked, and you cannot revoke a single node. Now each node has its own self-signed key pair, and the other side's list holds only the SHA-256 fingerprint of the certificate and the role it may play.

Two consequences:

- Permissions follow the list, not what the other side claims. A search node can only send events, read standings and query the index; a gateway can only forward user requests and ask about accounts; nobody can go beyond that.
- Revoking is deleting one line from the list. The file is reread after a change, and open connections are closed at the next request.

Clients check the other side's role too: whoever connects to a flywheel must be a flywheel, so a configuration mistake cannot send data to the wrong node.

## Why gateways keep no accounts

A gateway faces the internet and is the most likely to be compromised. If accounts lived on gateways, every additional gateway would add a copy of the key hashes and rate-limit state to keep in sync. Now a gateway asks the flywheel through `Admit`, `Charge` and `Refund`, and accounts, plans and token buckets exist only on the flywheel. The cost is one more round trip to the flywheel per request: in the test network with 2 milliseconds of latency, an account call through a gateway takes about 14 milliseconds (including the overhead of the client tool itself).

The concurrency limit and the failed-key count stay local to each gateway, since they are not worth another round trip. The price is that a user hitting N gateways at once can have up to N times the concurrency.

## Back-pressure instead of queues

When a node's concurrency is full, it answers `Busy` at once and does nothing. The gateway tries the next search node and answers HTTP 503 with `Retry-After` only when all are full. Queueing looks friendlier, but the longer the queue, the longer every request waits, until all of them time out. Rejecting at once tells the caller immediately to retry or switch nodes.

## Reliable delivery of events

Search nodes push the pages they read, usage records and standing observations to the flywheel in batches. Nothing may be lost or duplicated when the network flickers or the flywheel restarts:

- Events carry an `(epoch, seq)` number. The flywheel records the applied position of each sender and saves it to disk, so a resent batch is applied once.
- When the in-memory queue is full, events go to a disk spool, and the spool file is deleted only when all its events are acknowledged.
- Only beyond 2 GB is the oldest dropped, and the drops are counted.

## Preventing split brain: why a third node

Replication between two flywheels in a primary and standby pair is asynchronous, and it has a problem it cannot solve: in a network partition both sides think the other is dead and both accept writes, which cannot be merged afterwards.

Preventing that needs a majority. So a witness is added, and the three nodes vote on who may write. A flywheel holds the write lease, and handles reads and writes, only when it has its own vote and one other. The other flywheel answers `NotLeader`. An old leader cut off by a partition stops writing by itself after at most 2.5 seconds, and the other side takes over after about 4 seconds at most.

The relation between the two time constants is the heart of the design. A voter promises not to give its vote to anyone else for 4 seconds, while the holder considers itself the holder for only 2.5 seconds and renews every 0.5 seconds. The holder's belief period is shorter than the promise period, so at any moment when it believes it holds the lease, nobody else can have obtained the vote. This reasoning depends on the machines' clocks running at the same speed; it does not need them to agree on the time.

A flywheel without the lease is a follower and replicates from the holder. A recovered old leader becomes a follower by itself, does not take the lease back and needs no configuration change.

## What is synchronous and what is asynchronous

Account changes are semi-synchronous. For write requests to `/v1/account*` and `/v1/admin*`, the leader waits for the follower to pull the accounts (up to 3 seconds; if the follower is absent, it does not wait) before replying. A key just created, a plan change or a revocation therefore survives an unexpected leader crash. The test script verifies this: right after creating a key it runs `kill -9` on the leader, and the new key works on the new leader.

Other data is replicated asynchronously, and a short recent stretch is lost when the leader crashes:

| Data | Period |
|---|---|
| Token buckets and usage counts | 5 seconds |
| Host standings | 1 minute |
| Search records | 30 seconds |
| Index items | 10 minutes |

This is deliberate: losing a small piece of analytics data costs much less than making every write wait for the follower.

## What happens during a failover

The window after an unexpected crash is about 4 to 5 seconds. During it, gateways and search nodes retry requests that got `NotLeader` or could not connect, every 0.5 seconds and at most 5 times, so most requests only slow down. Events from search nodes go to the spool and are retried. On a clean stop (SIGTERM or Ctrl-C), the leader hands over the lease itself, with almost no gap.

If any two of the three nodes (the two flywheels and the witness) fail at once, the cluster is unavailable.

## What is not done

- More than two flywheels: the data nodes cannot scale out.
- Tests on real machines or across data centres: so far there is only the network-namespace simulation on one machine, with 2 milliseconds per hop.
- Automatic switch back: a recovered old leader does not take the lease back. That is the design, not a gap.
