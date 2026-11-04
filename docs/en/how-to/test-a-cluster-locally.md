# Test a cluster on one machine

`scripts/netns-cluster.sh` starts a whole cluster on one machine, with no root and no Docker. Each node sits in its own network namespace with its own IP, joined by a virtual switch that adds 2 milliseconds per hop.

## Requirements

- Linux that allows unprivileged user namespaces (the script uses `unshare -Urnm`).
- `iproute2` (`ip`, `tc`), `curl` and `bc` installed.
- A built `seesea`.

## Run

```bash
cd seesea-core
cargo build -p seesea-cli
scripts/netns-cluster.sh target/debug/seesea
```

The script starts 6 nodes: two flywheels, a witness, two search nodes and a gateway. It then runs its checks one by one, printing `ok` or `FAIL` for each, and ends with the number passed and failed. On a failure it prints the last lines of each node's log.

## What it checks

There are 18 checks:

- The gateway sees both search nodes; a wrong key is refused.
- The account endpoint and search work through the gateway.
- Exactly one of the two flywheels holds the write lease.
- The follower has the leader's search journal.
- A key created and then followed at once by `kill -9` of the leader works on the new leader.
- After the leader is killed, the follower takes the lease within 8 seconds, and accounts and searches still answer.
- When the old leader returns, it follows and does not take the lease back.
- A leader cut off from the others but still running stops writing, the other side takes over, accounts answer through the new leader, and after the network heals there is still exactly one writer.
- After one search node is killed, the other answers.
- Most requests succeed under 30% packet loss (QUIC retransmits).
- When one search node is isolated, the gateway returns a clean error and does not hang.

## Keep in mind

The nodes cannot reach the internet from their namespaces, so searches return no web results. The script checks the cluster itself (protocol, accounts, forwarding, failover and loss), not search quality. It is no substitute for testing on real machines or across data centres.
