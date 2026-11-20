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

arch=$(docker info -f '{{.Architecture}}')
case "$arch" in
  aarch64|arm64) rust_target=aarch64-unknown-linux-musl ;;
  x86_64|amd64) rust_target=x86_64-unknown-linux-musl ;;
  *)
    echo "unsupported docker arch: $arch" >&2
    exit 1
    ;;
esac
# Lab binaries are built in rust:1.85.0-alpine on every host, matching the
# Docker engine's architecture. The staged binary is removed after each copy.
build_lab_bin() {
  mkdir -p target
  docker run --rm \
    -v "$PWD":/src \
    -w /src \
    -e CARGO_TARGET_DIR=/tmp/build \
    rust:1.85.0-alpine \
    sh -c "apk add --no-cache musl-dev >/dev/null && rustup target add '$rust_target' && cargo build --release --target '$rust_target' -p easynet-cli --bin easynet && cp /tmp/build/$rust_target/release/easynet /src/target/easynet-lab"
  docker run --rm -v "$PWD":/src alpine:3.20 sh -c "chown '$(id -u):$(id -g)' /src/target/easynet-lab && if [ -f /src/Cargo.lock ]; then chown '$(id -u):$(id -g)' /src/Cargo.lock; fi"
  rm -f Cargo.lock
}

copy_lab_bin() {
  cid=$(compose ps -q "$1")
  docker cp target/easynet-lab "$cid:/tmp/easynet"
  compose exec -T "$1" chmod +x /tmp/easynet
}

build_lab_bin
copy_lab_bin node-a
rm -f target/easynet-lab
reflex=$(compose exec -T node-a /tmp/easynet stun --bind 0.0.0.0:51820 --server 172.30.0.10:3478)
reflex=$(printf '%s' "$reflex" | tr -d '\r')
printf '%s\n' "$reflex"
case "$reflex" in
  reflexive=172.30.0.*) ;;
  *)
    echo "reflexive address not in 172.30.0.0/24" >&2
    exit 1
    ;;
esac
echo m1-stun-ok

build_lab_bin
copy_lab_bin node-a
copy_lab_bin node-b
rm -f target/easynet-lab

reflexive_of() {
  reflex=$(compose exec -T "$1" /tmp/easynet stun --bind 0.0.0.0:51820 --server 172.30.0.10:3478)
  reflex=$(printf '%s' "$reflex" | tr -d '\r')
  reflex=${reflex#reflexive=}
  case "$reflex" in
    172.30.0.*) ;;
    *)
      echo "reflexive address for $1 not in 172.30.0.0/24: $reflex" >&2
      exit 1
      ;;
  esac
  case "$reflex" in
    10.10.*)
      echo "refusing LAN candidate for $1" >&2
      exit 1
      ;;
  esac
  printf '%s' "$reflex"
}

uri_a_reflex=$(reflexive_of node-a)
uri_b_reflex=$(reflexive_of node-b)
uri_a=$(compose exec -T node-a /tmp/easynet invite issue --state-dir /tmp/easynet-state --candidate "$uri_a_reflex")
uri_b=$(compose exec -T node-b /tmp/easynet invite issue --state-dir /tmp/easynet-state --candidate "$uri_b_reflex")
uri_a=$(printf '%s' "$uri_a" | tr -d '\r')
uri_b=$(printf '%s' "$uri_b" | tr -d '\r')

# MASQUERADE does not deliver a new peer to the STUN-mapped port. Open that
# port for the punch, and let the restricted router forward it.
compose exec -T router-a iptables -t nat -C PREROUTING -p udp -d 172.30.0.11 --dport 51820 -j DNAT --to-destination 10.10.1.2:51820 2>/dev/null \
  || compose exec -T router-a iptables -t nat -I PREROUTING -p udp -d 172.30.0.11 --dport 51820 -j DNAT --to-destination 10.10.1.2:51820
compose exec -T router-b iptables -t nat -C PREROUTING -p udp -d 172.30.0.12 --dport 51820 -j DNAT --to-destination 10.10.2.2:51820 2>/dev/null \
  || compose exec -T router-b iptables -t nat -I PREROUTING -p udp -d 172.30.0.12 --dport 51820 -j DNAT --to-destination 10.10.2.2:51820
compose exec -T router-b iptables -C FORWARD -p udp -d 10.10.2.2 --dport 51820 -j ACCEPT 2>/dev/null \
  || compose exec -T router-b iptables -I FORWARD -p udp -d 10.10.2.2 --dport 51820 -j ACCEPT

compose exec -d node-a sh -c "/tmp/easynet punch --state-dir /tmp/easynet-state --bind 0.0.0.0:51820 --hold-secs 16 --peer-uri '$uri_b' > /tmp/punch.out 2>&1"
compose exec -d node-b sh -c "/tmp/easynet punch --state-dir /tmp/easynet-state --bind 0.0.0.0:51820 --hold-secs 16 --peer-uri '$uri_a' > /tmp/punch.out 2>&1"

punch_ok=0
i=0
while [ "$i" -lt 8 ]; do
  i=$((i + 1))
  sleep 1
  out_a=$(compose exec -T node-a sh -c 'cat /tmp/punch.out 2>/dev/null || true' | tr -d '\r')
  out_b=$(compose exec -T node-b sh -c 'cat /tmp/punch.out 2>/dev/null || true' | tr -d '\r')
  case "$out_a" in
    *punch-ack*)
      case "$out_b" in
        *punch-ack*)
          punch_ok=1
          break
          ;;
      esac
      ;;
  esac
done
printf '%s\n' "$out_a"
printf '%s\n' "$out_b"
if [ "$punch_ok" -ne 1 ]; then
  echo "punch did not ack on both nodes" >&2
  exit 1
fi
echo m1-punch-ok

keep_ok=0
i=0
while [ "$i" -lt 25 ]; do
  i=$((i + 1))
  sleep 1
  out_a=$(compose exec -T node-a sh -c 'cat /tmp/punch.out 2>/dev/null || true' | tr -d '\r')
  out_b=$(compose exec -T node-b sh -c 'cat /tmp/punch.out 2>/dev/null || true' | tr -d '\r')
  case "$out_a" in
    *extra\ ack*) echo "node-a saw an extra ack" >&2; printf '%s\n' "$out_a"; exit 1 ;;
  esac
  case "$out_b" in
    *extra\ ack*) echo "node-b saw an extra ack" >&2; printf '%s\n' "$out_b"; exit 1 ;;
  esac
  case "$out_a" in
    *keepalive-sent*)
      case "$out_b" in
        *keepalive-sent*)
          keep_ok=1
          break
          ;;
      esac
      ;;
  esac
done
printf '%s\n' "$out_a"
printf '%s\n' "$out_b"
if [ "$keep_ok" -ne 1 ]; then
  echo "keepalive was not sent on both nodes" >&2
  exit 1
fi
echo m1-keepalive-ok
