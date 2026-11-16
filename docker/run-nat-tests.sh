#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compose() { docker compose -f docker/nat-lab.yml "$@"; }

fail_ping() {
  if compose exec -T "$1" ping -c 1 -W 2 "$2"; then
    echo "unexpected reachability $1 -> $2" >&2
    exit 1
  fi
}

fail_ping node-a 10.10.2.2
fail_ping node-a 10.10.3.2
fail_ping node-b 10.10.1.2
fail_ping node-b 10.10.3.2
fail_ping node-c 10.10.1.2
fail_ping node-c 10.10.2.2

for n in node-a node-b node-c; do
  compose exec -T "$n" sh -c 'echo x | nc -u -w1 172.30.0.10 3478'
done
compose exec -T router-a sh -c 'conntrack -L -p udp | grep -q 172.30.0.10'
compose exec -T router-b sh -c 'conntrack -L -p udp | grep -q 172.30.0.10'
compose exec -T router-c sh -c 'conntrack -L -p udp | grep -q 172.30.0.10'

compose exec -T node-c sh -c 'echo x | nc -u -w1 172.30.0.10 3478; echo y | nc -u -w1 172.30.0.10 3479'
compose exec -T router-c conntrack -L -p udp | awk '
  /dst=172.30.0.10/ && /dport=3478/ {
    for (i = 1; i <= NF; i++) if ($i ~ /^sport=/) s1 = substr($i, 7)
  }
  /dst=172.30.0.10/ && /dport=3479/ {
    for (i = 1; i <= NF; i++) if ($i ~ /^sport=/) s2 = substr($i, 7)
  }
  END { if (s1 == "" || s2 == "" || s1 == s2) exit 1 }
'

echo nat-lab-topology-ok
