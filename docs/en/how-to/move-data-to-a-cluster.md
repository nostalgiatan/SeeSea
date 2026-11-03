# Move a single node's data into a cluster

When you turn a single node into a cluster, its accounts, host standings, index items and search records can come along. Keys already issued keep working on the new nodes.

## Export

On the original single-node machine:

```bash
seesea cluster export bundle
```

`bundle` is a folder holding the accounts (key hashes only), host standings, index items and search records.

## Import into the new flywheel

Copy the folder to the flywheel's machine:

```bash
seesea cluster import bundle
```

Importing again does not write items twice; only the counts in the host standings add up each time. Accounts are imported on the flywheel because gateways keep none.

## Push to a running flywheel

When copying a folder is awkward, push from any machine:

```bash
seesea cluster push bundle --to quic://10.0.0.3:7443
```

A push uses the node key configured on this machine, and that key must be listed as `operator` in the flywheel's `peers` file. Make such an identity with:

```bash
seesea cluster keygen admin --role operator --out keys
```

Add the line it prints to the flywheel's `peers` file. `push` sends index items, search records and host standings, not accounts; import accounts with `import` as above.
