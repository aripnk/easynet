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
# Docker engine's architecture. The staged binary is removed after node-a
# receives it.
mkdir -p target
docker run --rm \
  -v "$PWD":/src \
  -w /src \
  -e CARGO_TARGET_DIR=/tmp/build \
  rust:1.85.0-alpine \
  sh -c "apk add --no-cache musl-dev >/dev/null && rustup target add '$rust_target' && cargo build --release --target '$rust_target' -p easynet-cli --bin easynet && cp /tmp/build/$rust_target/release/easynet /src/target/easynet-lab"
docker run --rm -v "$PWD":/src alpine:3.20 sh -c "chown '$(id -u):$(id -g)' /src/target/easynet-lab && if [ -f /src/Cargo.lock ]; then chown '$(id -u):$(id -g)' /src/Cargo.lock; fi"
rm -f Cargo.lock
cid=$(compose ps -q node-a)
docker cp target/easynet-lab "$cid:/tmp/easynet"
rm -f target/easynet-lab
compose exec -T node-a chmod +x /tmp/easynet
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
