# Developer environment & NAT lab

How to iterate on EasyNet without breaking the host network.
Architectural rules stay in [`AGENTS.md`](../AGENTS.md) and ADRs.
This file is the ops cookbook for toolchain + lab. [ADR-0011](adr/0011-dev-toolchain-and-nat-lab.md).

## 1. Why this shape

- TUN needs **`CAP_NET_ADMIN` / root** — avoid creating host `utun`/`wintun`
  during every vibecode loop.
- Hole punching and Symmetric NAT **cannot** be proven on `127.0.0.1` alone.
- Agents need one check command and a listed recipe set (`just --list`).

## 2. Toolchain (host or devcontainer)

| Piece | Role |
|---|---|
| `rust-toolchain.toml` | Pin rustc + `rustfmt` + `clippy` (+ CI cross targets) |
| `just` / `justfile` | Task runner; `set dotenv-load` |
| `cargo-nextest` | Parallel, process-isolated tests |
| `.cargo/config.toml` | Comments only. System linker. Do not set `-fuse-ld` |
| `deny.toml` + `cargo-deny` | License / duplicate / advisory gate when deps change |
| `.devcontainer/` | Linux TUN + NET_ADMIN inside Cursor/VS Code |

### Session check

```bash
just check
```

Expands to:

```text
bash scripts/check-card-scope.sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

`just scope-check` is only the first line. It must print `card-scope-ok`.
The card id is the committed **Next task**, and the paths are that card's
`**Files:**` list. [ADR-0017](adr/0017-card-scope-gate.md).

### Lab recipes (after M0 lab card)

```bash
just lab-up     # docker compose -f docker/nat-lab.yml up -d --build
just lab-test   # sh docker/run-nat-tests.sh
just lab-down   # docker compose -f docker/nat-lab.yml down -v
just deny       # cargo deny check (CI + when adding crates)
```

## 3. Docker NAT lab

M0-04 copies the files in this section **verbatim**. Do not swap images,
subnets, or iptables. Docker does not promise that `eth0` is the WAN, so
the router script selects interfaces by address. LAN docker gateways are
`.254` so the router can hold `.1`.

| Name | Addresses |
|---|---|
| `wan_net` | `172.30.0.0/24`. `stun` `172.30.0.10`. Routers `.11` `.12` `.13` |
| `lan_a` | `10.10.1.0/24`. Gateway `.254`. `router-a` `10.10.1.1`. `node-a` `10.10.1.2` |
| `lan_b` | `10.10.2.0/24`. Gateway `.254`. `router-b` `10.10.2.1`. `node-b` `10.10.2.2` |
| `lan_c` | `10.10.3.0/24`. Gateway `.254`. `router-c` `10.10.3.1`. `node-c` `10.10.3.2` |

`router-a` is endpoint-independent (MASQUERADE only). `router-b` is
port-restricted. `router-c` is symmetric (`MASQUERADE --random-fully`).
STUN is `coturn/coturn:4.6.2-alpine` with `--stun-only --no-auth
--listening-port=3478 --listening-ip=172.30.0.10`. No public STUN in CI.

`just lab-test` runs `sh docker/run-nat-tests.sh` on the host. M0-04 asserts
topology only. M1+ may append VIP ping asserts to that script and must leave
the topology asserts in place.

Chaos is not part of M0-04. A later card may add `tc qdisc add dev "$WAN" root netem delay 40ms loss 2%` inside a router.

### `docker/router.Dockerfile`

```dockerfile
FROM alpine:3.20
RUN apk add --no-cache iptables iproute2 conntrack-tools
COPY router-entry.sh /entry.sh
RUN chmod +x /entry.sh
ENTRYPOINT ["/bin/sh", "/entry.sh"]
```

### `docker/node.Dockerfile`

```dockerfile
FROM alpine:3.20
RUN apk add --no-cache iproute2
COPY node-entry.sh /entry.sh
RUN chmod +x /entry.sh
ENTRYPOINT ["/bin/sh", "/entry.sh"]
```

### `docker/router-entry.sh`

```sh
#!/bin/sh
set -eu
MODE=$1
CIDR=$2
WAN=$(ip -o -4 addr show | awk '/172\.30\.0\./ {print $2; exit}')
LAN=$(ip -o -4 addr show | awk '/10\.10\./ {print $2; exit}')
if [ -z "$WAN" ] || [ -z "$LAN" ]; then
  echo "router missing wan or lan address" >&2
  exit 1
fi
case "$MODE" in
  cone)
    iptables -t nat -A POSTROUTING -s "$CIDR" -o "$WAN" -j MASQUERADE
    ;;
  restricted)
    iptables -t nat -A POSTROUTING -s "$CIDR" -o "$WAN" -j MASQUERADE
    iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
    iptables -A FORWARD -i "$WAN" -o "$LAN" -j DROP
    ;;
  symmetric)
    iptables -t nat -A POSTROUTING -s "$CIDR" -o "$WAN" -j MASQUERADE --random-fully
    iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
    iptables -A FORWARD -i "$WAN" -o "$LAN" -j DROP
    ;;
  *)
    echo "bad mode $MODE" >&2
    exit 1
    ;;
esac
exec sleep infinity
```

### `docker/node-entry.sh`

```sh
#!/bin/sh
set -eu
ip route replace default via "$1"
exec sleep infinity
```

### `docker/nat-lab.yml`

```yaml
name: easynet-nat-lab

networks:
  wan_net:
    ipam:
      config:
        - subnet: 172.30.0.0/24
          gateway: 172.30.0.1
  lan_a:
    ipam:
      config:
        - subnet: 10.10.1.0/24
          gateway: 10.10.1.254
  lan_b:
    ipam:
      config:
        - subnet: 10.10.2.0/24
          gateway: 10.10.2.254
  lan_c:
    ipam:
      config:
        - subnet: 10.10.3.0/24
          gateway: 10.10.3.254

services:
  stun:
    image: coturn/coturn:4.6.2-alpine
    networks:
      wan_net:
        ipv4_address: 172.30.0.10
    command:
      - --stun-only
      - --no-auth
      - --listening-port=3478
      - --listening-ip=172.30.0.10

  router-a:
    build:
      context: .
      dockerfile: router.Dockerfile
    cap_add: [NET_ADMIN]
    sysctls:
      net.ipv4.ip_forward: "1"
    entrypoint: ["/bin/sh", "/entry.sh", "cone", "10.10.1.0/24"]
    networks:
      wan_net:
        ipv4_address: 172.30.0.11
      lan_a:
        ipv4_address: 10.10.1.1

  node-a:
    build:
      context: .
      dockerfile: node.Dockerfile
    cap_add: [NET_ADMIN]
    devices:
      - /dev/net/tun:/dev/net/tun
    entrypoint: ["/bin/sh", "/entry.sh", "10.10.1.1"]
    networks:
      lan_a:
        ipv4_address: 10.10.1.2

  router-b:
    build:
      context: .
      dockerfile: router.Dockerfile
    cap_add: [NET_ADMIN]
    sysctls:
      net.ipv4.ip_forward: "1"
    entrypoint: ["/bin/sh", "/entry.sh", "restricted", "10.10.2.0/24"]
    networks:
      wan_net:
        ipv4_address: 172.30.0.12
      lan_b:
        ipv4_address: 10.10.2.1

  node-b:
    build:
      context: .
      dockerfile: node.Dockerfile
    cap_add: [NET_ADMIN]
    devices:
      - /dev/net/tun:/dev/net/tun
    entrypoint: ["/bin/sh", "/entry.sh", "10.10.2.1"]
    networks:
      lan_b:
        ipv4_address: 10.10.2.2

  router-c:
    build:
      context: .
      dockerfile: router.Dockerfile
    cap_add: [NET_ADMIN]
    sysctls:
      net.ipv4.ip_forward: "1"
    entrypoint: ["/bin/sh", "/entry.sh", "symmetric", "10.10.3.0/24"]
    networks:
      wan_net:
        ipv4_address: 172.30.0.13
      lan_c:
        ipv4_address: 10.10.3.1

  node-c:
    build:
      context: .
      dockerfile: node.Dockerfile
    cap_add: [NET_ADMIN]
    devices:
      - /dev/net/tun:/dev/net/tun
    entrypoint: ["/bin/sh", "/entry.sh", "10.10.3.1"]
    networks:
      lan_c:
        ipv4_address: 10.10.3.2
```

### `docker/run-nat-tests.sh`

M0-04 topology contract. Exit 0 only after every assert. M1+ may append
`easynetd` ping checks below the `echo`.

```sh
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
```

### Later milestones

When `easynetd` exists, extend `run-nat-tests.sh` so `node-a` pings `node-b`'s
VIP and `node-c`'s VIP with `ping -c 3` and `0%` packet loss. Do not delete
the topology asserts above.

## 4. CI, deny, linker, devcontainer

Copy these files in M0-04. The lab job must not set `continue-on-error`.

### `.cargo/config.toml`

```toml
# System linker. Do not set linker or -fuse-ld in this file.
# Optional, only in a shell that already has lld:
#   RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

### `deny.toml`

```toml
[advisories]
vulnerability = "deny"

[licenses]
allow = [
  "MIT",
  "Apache-2.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unicode-3.0",
  "MPL-2.0",
  "Zlib",
]
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

If `cargo deny check` fails only because a direct dependency's SPDX id is
missing from `allow`, add that id and name it in the progress entry. Do not
set `vulnerability = "warn"`.

### `.github/workflows/ci.yml`

```yaml
name: ci
on:
  push:
  pull_request:
jobs:
  scope:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Fetch base
        run: |
          if [ -n "${{ github.base_ref }}" ]; then
            git fetch origin "${{ github.base_ref }}"
            echo "RANGE=origin/${{ github.base_ref }}...HEAD" >> "$GITHUB_ENV"
          elif git rev-parse --verify origin/main >/dev/null 2>&1; then
            echo "RANGE=origin/main...HEAD" >> "$GITHUB_ENV"
          fi
      - name: Card allowlist
        run: |
          if [ -n "${RANGE:-}" ]; then
            bash scripts/check-card-scope.sh --range "$RANGE"
          else
            echo "card-scope-ok (no base ref)"
          fi
  check:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.85.0
          components: rustfmt, clippy
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest
      - run: cargo fmt --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo nextest run --workspace
  lab:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: extractions/setup-just@v2
      - run: just lab-up
      - run: just lab-test
      - if: always()
        run: just lab-down
```

### `.devcontainer/devcontainer.json`

```json
{
  "name": "easynet",
  "image": "mcr.microsoft.com/devcontainers/base:bookworm",
  "runArgs": ["--cap-add=NET_ADMIN", "--device=/dev/net/tun"],
  "postCreateCommand": "apt-get update && apt-get install -y iptables iproute2 conntrack curl ca-certificates && curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain none && . \"$HOME/.cargo/env\" && cargo install cargo-nextest --locked && cargo install cargo-deny --locked && cargo install just --locked"
}
```

`postCreateCommand` installs rustup with no default toolchain. The repo
`rust-toolchain.toml` selects 1.85.0 the first time `cargo` runs. Do not
change that command to a floating `stable` toolchain.

## 5. Agent rules (pointer)

Do not duplicate the session contract here. Agents read [`AGENTS.md`](../AGENTS.md):

- One card, allowlisted files, then **`just check`**
- Wire constants and VIP hash are in `docs/api.md` / ADR-0008 / ADR-0010
- No second primary UDP socket; keep state machines in `easynet-core`
- Prefer lab recipes over host TUN when the card names `just lab-test`

## 6. Deferred: Q15 mesh DNS

**Out of M4.** Do not implement during portal work. Sketch:
[`api.md` §10](api.md#10-deferred-q15-mesh-dns). M4 uses loopback portal helper
+ `easynet status` instead of OS split-DNS.
