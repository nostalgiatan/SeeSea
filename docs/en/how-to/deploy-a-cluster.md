# Deploy a cluster

This page shows how to split seesea into gateways, search nodes and flywheel nodes, with two flywheels and a witness so a network split cannot give you two writers. What each role does and why it is built this way is in [Cluster design](../explanation/cluster-design.md).

The default `role = "single"` does everything in one process and does not need this page.

## Layout

| Role | Count | Purpose |
|---|---|---|
| `gateway` | Any number, behind a load balancer | User-facing HTTP; keeps no accounts |
| `search` | One or more | Runs the pipeline and the models |
| `flywheel` | Two | Keeps accounts, index, host standings and usage records; only one writes at a time |
| `witness` | One | Votes only; stores no data and loads no models |

Nodes talk to each other on UDP port 7443 (QUIC). The gateway listens for users over HTTP.

## 1. Make an identity for each node

On any machine, make a key pair for each node, with your own names and roles:

```bash
seesea cluster keygen fly1 --role flywheel --out keys
seesea cluster keygen fly2 --role flywheel --out keys
seesea cluster keygen wit  --role witness  --out keys
seesea cluster keygen s0   --role search   --out keys
seesea cluster keygen gw0  --role gateway  --out keys
```

Each command writes `keys/<name>.pem` (the certificate) and `keys/<name>.key` (the private key) and prints one line:

```
<SHA-256 fingerprint> <role> <name>
```

Put all the lines in one `peers` file and send it to every node. Keep each private key only on its own node, readable by its owner only; otherwise startup warns.

## 2. Write each node's configuration

Three keys every node has:

```toml
node_cert = "/etc/seesea/node.pem"
node_key = "/etc/seesea/node.key"
cluster_peers = "/etc/seesea/peers"
```

Then, per role, with your own addresses:

**flywheel (fly1 and fly2 are alike, each pointing `partner` at the other)**

```toml
role = "flywheel"
cluster_listen = "0.0.0.0:7443"
partner = "quic://10.0.0.5:7443"     # the other flywheel
witness = "quic://10.0.0.6:7443"
```

The `[plans]` of the two flywheels must be the same.

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

A gateway listening for users on a non-loopback address needs TLS or a proxy in front; see [Run the HTTP server](run-the-server.md#open-it-to-others).

## 3. Create accounts and start

Accounts are managed on a flywheel. On one flywheel's machine:

```bash
seesea users add ann
seesea keys create ann
```

The other flywheel gets the accounts by replication within seconds. To have them at its first start, export and import:

```bash
seesea cluster export bundle              # on the first flywheel
seesea cluster import bundle              # on the second flywheel
```

Start on every machine, the witness and flywheels first:

```bash
seesea serve --addr 127.0.0.1:8080
```

On a gateway, replace `--addr` with the public address.

## 4. Check

```bash
curl -H "Authorization: Bearer ss_..." http://gateway:8080/v1/cluster/status
```

On a gateway, `search_nodes.healthy` should equal the number of search nodes. Ask each flywheel once: exactly one should report `writer` as `true`.

## Everyday operations

**Restart a flywheel in turn.** Stop it cleanly with SIGTERM or Ctrl-C. The node holding the write lease hands it over, and the other takes it almost at once.

**Revoke a node.** Delete its line from the `peers` file on every node. The file is reread after the change, and the node's open connections are closed at the next request.

**Replace a node with a new machine.** Run `cluster keygen` for the new machine, add its line to every node's `peers`, and delete the old line.

**After a failover.** The lease moves by itself. When the old leader returns it becomes a follower, with no configuration change. After an unplanned crash the failover takes about 4 to 5 seconds, during which requests are retried and most only slow down.

## One primary and one standby, no witness

You can give the standby flywheel `standby_of = "quic://primary:7443"` and nothing else. This does not prevent a split brain: in a network partition both sides can accept writes. When the standby accepted writes while it could not see the primary, it logs a warning as soon as the primary returns and shows `standby.diverged: true` in `/v1/cluster/status`. After a failover you have to treat the standby as the new primary by hand: point the old primary's `standby_of` at it, and swap `flywheel` and `flywheel_standby` on both sides. Use the witness setup above whenever you can.
