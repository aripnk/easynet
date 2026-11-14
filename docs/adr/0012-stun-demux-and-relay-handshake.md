# ADR-0012: STUN-before-mesh demux, and RELAY may wrap HANDSHAKE

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

`docs/api.md` checked mesh type bytes before the STUN magic cookie. A STUN Binding Success (`0x0101`) has first byte `0x01`, the same byte as `PUNCH`, so reflexive responses would be parsed as mesh frames. Separately, `0x04 RELAY` allowed only an inner `0x03 DATA` frame. Two symmetric-NAT peers never get a direct path, so they could never complete the `0x05` Noise IK handshake that produces `DATA`. Owner locked both rules on 2026-11-14. This does not change ADR-0003 (one primary socket), ADR-0009 (relay eligibility), or ADR-0010 (`0x05` exists).

## Decision

1. On the primary UDP socket, drop datagrams shorter than 8 bytes. If `len >= 20` and bytes `4..8` are `21 12 A4 42`, the datagram is STUN, including when byte 0 is `0x00` or `0x01`. Otherwise dispatch byte 0: `0x01` PUNCH, `0x02` GOSSIP, `0x03` DATA, `0x04` RELAY, `0x05` HANDSHAKE. Anything else is dropped. Exact order: [api.md §1](../api.md#1-udp-demultiplex).
2. A `0x04 RELAY` inner frame must be unmodified `0x03 DATA` or `0x05 HANDSHAKE`. Never `0x01`, `0x02`, or a nested `0x04`.
3. Relayed `0x05` is at most **128 bytes** and at most **2 packets/sec** per `(src_node_id, dst_node_id)`. The 10 Mbps / 2 MB bucket applies to inner `0x03` only. Relays do not decrypt either inner type.
4. After a direct `PUNCH` ACK, peers send further `0x03 DATA` on that direct path. Layout and punch replay cache (30s, 4096 entries, `nonce` as `u64`): [api.md §3](../api.md#3-type-0x01-punch) and [§6](../api.md#6-type-0x04-relay).

## Consequences

- Golden tests must include a STUN success whose first byte is `0x01` and whose cookie is at bytes 4..8, and must show it is not parsed as `PUNCH`.
- M3 relay tests must show a 115-byte `HANDSHAKE_INIT` forwarded and a 129-byte inner `0x05` dropped.

## Alternatives considered

- **Classify `0x01` as mesh first** — drops STUN success responses; rejected.
- **Relay only `DATA`** — symmetric peers cannot finish IK; rejected.
