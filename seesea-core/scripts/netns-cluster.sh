#!/usr/bin/env bash
# Runs a whole seesea cluster on one machine, each node in its own network namespace with its own IP, joined by
# a virtual switch with added latency: a flywheel and its standby, two search nodes, a gateway. Needs no root
# and no Docker: it uses an unprivileged user+network namespace (`unshare -Urnm`).
#
#   scripts/netns-cluster.sh target/debug/seesea            # runs the scenarios below
#
# The nodes cannot reach the internet from here, so searches return no hits; what is exercised is the cluster
# (wire protocol, accounts, forwarding, failover, loss), not the web.
set -euo pipefail
BIN=$(realpath "${1:-target/debug/seesea}")
if [ -z "${NETNS_INNER:-}" ]; then
  exec env NETNS_INNER=1 unshare -Urnm bash "$0" "$BIN"
fi
mount -t tmpfs tmpfs /run; mkdir -p /run/netns
W=$(mktemp -d); trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$W"' EXIT
ip link add br0 type bridge; ip link set br0 up; ip addr add 10.9.0.1/24 dev br0; ip link set lo up
declare -A IP=([fly]=10.9.0.10 [fly2]=10.9.0.11 [w]=10.9.0.12 [s0]=10.9.0.20 [s1]=10.9.0.21 [gw]=10.9.0.30)
for n in "${!IP[@]}"; do
  ip netns add $n; ip link add v_$n type veth peer name p_$n; ip link set p_$n netns $n
  ip link set v_$n master br0; ip link set v_$n up
  ip netns exec $n ip link set lo up
  ip netns exec $n ip addr add ${IP[$n]}/24 dev p_$n; ip netns exec $n ip link set p_$n up
  tc qdisc add dev v_$n root netem delay 2ms   # each hop adds 2 ms; a request crosses two
done
mkdir -p $W/keys; : > $W/peers
declare -A ROLE=([fly]=flywheel [fly2]=flywheel [w]=witness [s0]=search [s1]=search [gw]=gateway)
for n in "${!IP[@]}"; do $BIN cluster keygen $n --role ${ROLE[$n]} --out $W/keys | tail -1 >> $W/peers; done
cfg() { # name extra
  cat > $W/$1.toml <<EOT
role = "${ROLE[$1]}"
node_cert = "$W/keys/$1.pem"
node_key = "$W/keys/$1.key"
cluster_peers = "$W/peers"
$2
EOT
}
cfg fly  "cluster_listen = \"${IP[fly]}:7443\"
partner = \"quic://${IP[fly2]}:7443\"
witness = \"quic://${IP[w]}:7443\""
cfg fly2 "cluster_listen = \"${IP[fly2]}:7443\"
partner = \"quic://${IP[fly]}:7443\"
witness = \"quic://${IP[w]}:7443\""
cfg w "cluster_listen = \"${IP[w]}:7443\""
for s in s0 s1; do cfg $s "cluster_listen = \"${IP[$s]}:7443\"
flywheel = \"quic://${IP[fly]}:7443\"
flywheel_standby = \"quic://${IP[fly2]}:7443\""; done
cfg gw "search_nodes = [\"quic://${IP[s0]}:7443\", \"quic://${IP[s1]}:7443\"]
flywheel = \"quic://${IP[fly]}:7443\"
flywheel_standby = \"quic://${IP[fly2]}:7443\""
declare -A PID
start() { n=$1; ip netns exec $n env XDG_DATA_HOME=$W/data-$n SEESEA_CONFIG=$W/$n.toml $BIN serve --addr 127.0.0.1:8080 > $W/$n.log 2>&1 & PID[$n]=$!; }
# accounts live on the primary flywheel
KEY=$(XDG_DATA_HOME=$W/data-fly SEESEA_CONFIG=$W/fly.toml $BIN users add ann >/dev/null && XDG_DATA_HOME=$W/data-fly SEESEA_CONFIG=$W/fly.toml $BIN keys create ann | grep -o 'ss_[A-Za-z0-9_]*' | head -1)
# the second flywheel node starts with the same accounts (later changes travel by replication)
XDG_DATA_HOME=$W/data-fly SEESEA_CONFIG=$W/fly.toml $BIN cluster export $W/bundle >/dev/null
XDG_DATA_HOME=$W/data-fly2 SEESEA_CONFIG=$W/fly2.toml $BIN cluster import $W/bundle >/dev/null
for n in w fly fly2 s0 s1 gw; do start $n; done
sleep 9
api() { ip netns exec gw curl -s -m 20 -o /dev/null -w '%{http_code} %{time_total}s\n' -H "Authorization: Bearer $KEY" "http://127.0.0.1:8080$1"; }
ok=0; bad=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; ok=$((ok+1)); else echo "FAIL $1: got '$2' want '$3'"; bad=$((bad+1)); for f in $W/*.log; do echo "--- $(basename $f)"; tail -n 4 $f; done; fi; }
code() { api "$1" | cut -d' ' -f1; }

check "gateway sees the cluster" "$(ip netns exec gw curl -s -H "Authorization: Bearer $KEY" http://127.0.0.1:8080/v1/cluster/status | grep -o '"search_nodes":{"healthy":[0-9]*')" '"search_nodes":{"healthy":2'
check "a wrong key is refused" "$(ip netns exec gw curl -s -o /dev/null -w '%{http_code}' -H 'Authorization: Bearer ss_nope_nope' http://127.0.0.1:8080/v1/account)" 401
check "account through the gateway" "$(code /v1/account)" 200
check "search through gateway and search node" "$(code '/v1/search?q=rust&mode=fast&sources=local')" 200
echo "latency of 20 sequential accounts calls (gateway -> flywheel, 2 hops of 2 ms each way):"
t0=$(date +%s.%N); for i in $(seq 20); do code /v1/account >/dev/null; done; t1=$(date +%s.%N)
echo "  $(echo "($t1 - $t0) * 1000 / 20" | bc -l | cut -c1-6) ms per call"
writer() { ip netns exec $1 curl -s -m 3 -H "Authorization: Bearer $KEY" http://127.0.0.1:8080/v1/cluster/status | grep -o '"writer":[a-z]*' | cut -d: -f2; }
w1=$(writer fly); w2=$(writer fly2)
check "exactly one flywheel node holds the write lease" "$w1/$w2" "$([ "$w1" = true ] && echo true/false || echo false/true)"
if [ "$w1" = true ]; then L=fly; F=fly2; else L=fly2; F=fly; fi
echo "leader: $L  follower: $F"
sleep 40   # the follower copies the accounts and the journal
jl() { cat $W/data-$1/seesea/journal/*.jsonl 2>/dev/null | grep -c '"type":"search"' || true; }
check "the follower has the leader's journal" "$([ "$(jl $L)" -ge 1 ] && [ "$(jl $L)" = "$(jl $F)" ] && echo same)" same
# a key created through the gateway, and the leader killed at once: the key must survive (accounts are copied before the answer)
NEW=$(ip netns exec gw curl -s -m 20 -X POST -H "Authorization: Bearer $KEY" http://127.0.0.1:8080/v1/account/keys -H 'content-type: application/json' -d '{"name":"late"}' | grep -o 'ss_[A-Za-z0-9_]*' | awk '{ if (length($0) > length(m)) m = $0 } END { print m }')
kill -9 ${PID[$L]}; sleep 8
check "a key created just before the leader died works on the new leader" "$(ip netns exec gw curl -s -m 20 -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $NEW" http://127.0.0.1:8080/v1/account)" 200
check "leader killed: the follower takes the lease within 8 s" "$(writer $F)" true
check "leader killed: accounts still answer" "$(code /v1/account)" 200
check "leader killed: searches still answer" "$(code '/v1/search?q=rust&mode=fast&sources=local')" 200
start $L; sleep 10
check "old leader back: it follows, it does not take the lease back" "$(writer $L)/$(writer $F)" false/true
ip link set v_$F down; sleep 10
check "leader cut off from the others but alive: it stops writing" "$(writer $F)" false
check "the other side took over" "$(writer $L)" true
check "accounts answer through the new leader" "$(code /v1/account)" 200
ip link set v_$F up; sleep 8
check "after healing there is still exactly one writer" "$(writer $L)/$(writer $F)" true/false
kill -9 ${PID[s0]}; sleep 9
for i in 1 2 3 4; do r=$(code '/v1/search?q=rust&mode=fast&sources=local'); [ "$r" = 200 ] || break; done
check "a search node killed: the other answers" "$r" 200
tc qdisc change dev v_s1 root netem delay 2ms loss 30%
c200=0; for i in $(seq 10); do [ "$(code '/v1/search?q=rust&mode=fast&sources=local')" = 200 ] && c200=$((c200+1)); done
echo "30% packet loss on the last search node: $c200/10 requests answered"
check "most requests survive 30% loss (QUIC retransmits)" "$([ $c200 -ge 8 ] && echo yes)" yes
ip link set v_s1 down; sleep 1
check "search node partitioned: clean error, not a hang" "$(code '/v1/search?q=rust&mode=fast')" 502
echo "$ok passed, $bad failed"; [ $bad -eq 0 ]
