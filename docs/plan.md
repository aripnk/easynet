# Historical design dump (not the session contract)

This file is the original product/design brainstorm. It is **not**
authoritative for agents.

| Need | Read instead |
|---|---|
| Session contract | [`../AGENTS.md`](../AGENTS.md) |
| Current behavior | [`spec.md`](spec.md) |
| Wire shapes | [`api.md`](api.md) |
| Admin UI | [`design.md`](design.md) |
| Decisions | [`adr/README.md`](adr/README.md) |
| What to do next | [`progress.md`](progress.md) |
| Dev / NAT lab | [`dev.md`](dev.md) |

If this dump disagrees with `spec.md`, the **spec wins**.

---

# Decentralized Zero-Server P2P Mesh Network Specification

## 1. Overview & Design Goals

A decentralized, zero-subscription peer-to-peer (P2P) overlay VPN mesh designed to provide standard Layer-3 network connectivity (`ping`, `ssh`, HTTP, SMB) across desktop, server, and mobile devices without relying on a central coordination server or third-party cloud control plane.

* **Zero Central Infrastructure:** Eliminates dedicated signaling servers and paid relay subscriptions by combining out-of-band pairing tokens (QR codes / Base64 strings), public STUN/UPnP discovery, and peer-assisted introductions.
* **No Device or User Caps:** Membership is governed purely by cryptographic keys and signed admin certificates.
* **Resilient to Mobile & Intermittent Nodes:** Uses a SWIM-style gossip protocol rather than quorum-based consensus (Raft) so single awake admin nodes can approve new peers even when other nodes are offline or asleep.
* **Seamless Symmetric NAT Traversal:** Combines single-socket UDP multiplexing, Birthday Paradox port probing, and automatic traffic relaying through any reachable Cone-NAT peer.

---

## 2. System Architecture

Every node runs an identical user-space daemon (`meshnetd`) organized into four decoupled layers:

```
+-------------------------------------------------------------------+
|  OS Applications (ping, ssh, curl, browser, SMB)                  |
+-------------------------------------------------------------------+
                                 |  Raw IPv4/IPv6 Packets (MTU 1380)
                                 v
+-------------------------------------------------------------------+
|  Layer 4: Virtual Network Interface (TUN / utun / Wintun / VPN)   |
|  - Deterministic Virtual IP Mapping (100.64.0.0/10)               |
+-------------------------------------------------------------------+
                                 |
                                 v
+-------------------------------------------------------------------+
|  Layer 3: Control Plane & State Sync (SWIM Gossip + Admin Certs)  |
|  - Peer Table & Endpoint Roaming (Monotonic Incarnation Counters) |
|  - Signed Admin Approval Certificates & Revocation Tombstones     |
|  - Local HTTP Admin Portal & Deterministic Webhook Notifier       |
+-------------------------------------------------------------------+
                                 |
                                 v
+-------------------------------------------------------------------+
|  Layer 2: NAT Traversal, Crypto & Relay Engine                    |
|  - Noise IK / ChaCha20-Poly1305 End-to-End Encryption             |
|  - Simultaneous UDP Hole Punching & Birthday Paradox Probing      |
|  - Cone-NAT Peer Relaying (Fallback for Dual Symmetric NAT)       |
+-------------------------------------------------------------------+
                                 |  Single Bound UDP Socket (:51820)
                                 v
+-------------------------------------------------------------------+
|  Layer 1: Transport & Discovery                                   |
|  - Out-of-Band Token / QR Exchange (Channel-Agnostic)             |
|  - UPnP IGD / NAT-PMP / Public STUN Endpoint Resolution           |
+-------------------------------------------------------------------+

```

---

## 3. Identity, Cryptography & Trust Model

### 3.1 Node Identity & Virtual IP Derivation

* **Keypair:** Each node generates an `X25519` static keypair (for Noise session encryption) and an `Ed25519` signing keypair (for gossip records and admin certificates).
* **Deterministic Virtual IP:** To prevent IP collisions without a central DHCP server, each node derives its `100.64.0.0/10` address directly from its `Ed25519` public key:

$$\text{IPv4} = \texttt{100.64.} \text{SHA256}(\text{PubKey})[0] \texttt{.} \text{SHA256}(\text{PubKey})[1]$$

*(Reserved `.0` and `.255` host suffixes are incremented by `1`.)*

**Superseded guidance:** see `spec.md` §5 and ADR-0008 — do not treat `100.64.0.1` as a privileged admin address.

### 3.2 Roles & Signed Membership Certificates

Instead of voting on membership, trust is anchored by an **Admin Root Keypair** generated when the network is created:

1. **Admin Nodes (e.g., Node A & Node B):** Hold the Network Root Signing Key (or delegated Admin Keys signed by Root). Any single awake Admin node can issue approvals.
2. **Member Certificate (`PeerCert`):**
```json
{
  "mesh_id": "net-9f8a21",
  "peer_pub_key": "<base64_ed25519_pub>",
  "x25519_pub_key": "<base64_x25519_pub>",
  "virtual_ip": "100.64.14.88",
  "role": "member",
  "issued_at": 1794660000,
  "expires_at": 1826196000,
  "issuer_pub_key": "<base64_admin_pub>",
  "signature": "<ed25519_signature>"
}

```


3. **Revocation Tombstones:** Removing a compromised or lost device broadcasts a signed `RevokeCert` record containing `{peer_pub_key, revoked_at, admin_signature}`. Peers immediately drop packets from revoked keys and propagate the tombstone via gossip.

---

## 4. Wire Protocol & Single-Socket Multiplexing

See authoritative framing in [`api.md`](api.md).

### 4.1 Packet Framing Layout

Every UDP datagram begins with a 1-byte magic header identifying the sub-protocol:

| Byte `0` (Type) | Bytes `1..4` | Bytes `5..20` | Bytes `21..N` |
| --- | --- | --- | --- |
| `0x01` **PUNCH** | Protocol Version + Flags (`SYN` / `ACK`) | Sender Node ID (16-byte truncated hash) | Signed Timestamp + Nonce + Observed Source `IP:Port` |
| `0x02` **GOSSIP** | Message Subtype (`PING`, `ACK`, `PING_REQ`) | Sender Node ID | Encrypted & Signed SWIM State Delta (`PeerTable`, `PeerCerts`, `Revocations`) |
| `0x03` **DATA** | Receiver Session Index | 12-byte Nonce / Counter | `ChaCha20-Poly1305` Encrypted Raw `TUN` IP Packet |
| `0x04` **RELAY** | Hop TTL (`1`) | Target Destination Node ID | Unmodified `0x03 DATA` Frame (End-to-End Encrypted for Target) |

*(Note: Standard STUN Binding Requests/Responses use RFC 5389 magic cookie bytes in bytes `4..7` and `0x00`/`0x01` in byte `0` with message length in bytes `2..3`, allowing the socket demultiplexer to distinguish STUN replies cleanly from mesh packets.)*

---

## 5. Connection & NAT Traversal Workflows

### 5.1 Phase 1: Initial Network Bootstrap (`Node A` $\leftrightarrow$ `Node B`)

1. **Bind & Discover:** Both A and B bind UDP `:51820`, attempt local UPnP/NAT-PMP port mapping, and query a public STUN server (`stun.l.google.com:19302` or `stun.cloudflare.com:3478`) to collect candidate endpoints (`[Public_IP:Port, LAN_IP:51820, IPv6:51820]`).
2. **Hold NAT Mapping:** Both nodes start a background 15-second keepalive ping to the STUN endpoint so their routers do not expire the port mapping during pairing.
3. **Out-of-Band Token Swap:** A and B exchange compact Base64 invite tokens (`meshnet://pair?d=<base64>`) via QR scan, SSH, or messaging.
4. **Simultaneous Punch:** Upon importing tokens, both nodes send `0x01 PUNCH (SYN)` packets every `250 ms` to all candidate endpoints.
5. **Lock-In:** Upon receiving a valid `PUNCH (SYN)`, each peer responds with `PUNCH (ACK)` to the packet's actual source `IP:Port`, stops probing, and begins 15-second `0x02 GOSSIP PING` keepalives.

### 5.2 Phase 2: Adding Node C (Phone / Laptop) via Admin Approval

1. **Join Request Token:** Node C discovers its `Public_IP:Port` via STUN and displays a **Join Request QR Code** containing `{mesh_id, peer_pub_key, x25519_pub_key, candidates}`.
2. **Scan / Submit to Node A (or B):** You scan C's QR code using Node A's mobile/web UI (or paste it into A's CLI/webhook).
3. **Admin Approval & Roster Export:**
* Node A places C in `pending_peers` and gossips the pending request to B.
* You click **Accept** on A's (or B's) local Admin Portal (`http://<admin_vip>:8080` — VIP is derived, not `.1`).
* The approving Admin node signs C's `PeerCert`, gossips it to the mesh, and displays a **Mesh Roster QR Code** containing `{PeerCert_C, [Node_A_Info, Node_B_Info]}`.


4. **Mesh Convergence:**
* Node A immediately sends `0x01 PUNCH` packets to C and notifies B via gossip.
* Node C scans the Roster QR Code and simultaneously punches toward both A and B.



### 5.3 Phase 3: Handling Symmetric NAT & Cellular Roaming

* **Asymmetric Pairing (Cone + Symmetric NAT):** If Node C is on 5G Symmetric NAT and Node A is on Cone NAT, C opens 128 UDP sockets sending probes to A's known `IP_A:Port_A` while A sprays 1,024 probes across random ports on `IP_C` from its single socket (Birthday Paradox collision $\approx 86\%–99\%$ within 1 second).
* **Dual Symmetric NAT Relay Fallback:** If Node C and Node D both sit behind strict Symmetric NATs and fail to exchange `PUNCH (ACK)` within `2.0 seconds`, C automatically wraps traffic for D inside `0x04 RELAY` packets routed through **Node A** (which acts as a blind, zero-knowledge forwarder).
* **Wi-Fi $\leftrightarrow$ 5G Roaming:** When Node C's IP or port changes, it increments its monotonic `incarnation` counter, signs the new endpoint record, and sends a packet to any reachable peer (Node A). Node A verifies the cryptographic signature, updates C's routing entry, and gossips the new endpoint to B.

---

## 6. Control Plane: SWIM Gossip & Admin Portal

### 6.1 Gossip Dissemination Rules

* **Cycle Interval:** Every `2 seconds`, each node picks one random `ALIVE` peer and sends a `0x02 GOSSIP PING` piggybacking recent state deltas (`PeerEndpoint`, `PeerCert`, `RevokeCert`).
* **Conflict Resolution (CRDT Merge):**
* Endpoint updates are resolved by highest signed `incarnation` integer.
* Membership state is resolved by `RevokeCert` (always wins) > valid `PeerCert` signed by a trusted Admin key.


* **Anchor Recovery (Simultaneous Reboot Protection):** At least one node (e.g., Home Server A) should configure UPnP/NAT-PMP + Dynamic DNS (`anchor.duckdns.org:51820`) so that if all nodes reboot simultaneously with new dynamic IPs, peers can resolve the anchor's DNS record to re-enter the gossip ring.

### 6.2 Admin Portal & Single-Alert Webhook Rule

* **Local Web UI:** Each Admin node binds an HTTP server strictly on its virtual `TUN` IP—accessible only to authenticated mesh members—to view topology, approve pending joins, and revoke keys.
* **Duplicate-Free Notifications:** To prevent duplicate Telegram/webhook alerts when a join request reaches multiple Admin nodes, only the currently `ALIVE` Admin node with the **lexicographically lowest Node ID** fires external webhook alerts.

---

## 7. Virtual Network Interface (`TUN`) Configuration

* **Interface Name:** `mesh0` (Linux/Windows) or `utunX` (macOS/iOS).
* **Subnet:** `100.64.0.0/10` (RFC 6598 Shared Address Space, avoiding conflicts with `10.0.0.0/8` and `192.168.1.0/16`).
* **MTU:** **`1380` bytes** (accounts for outer IPv4/IPv6 + UDP + Noise AEAD overhead + 1-byte mux header without IP fragmentation).
* **Optional LAN Subnet Routing:** Enabling IP forwarding and `iptables`/`nftables` NAT masquerading on Node A allows any mesh peer to reach non-mesh devices on Node A's home LAN (`192.168.1.0/24`).

---

## 8. Recommended Tech Stack & Implementation Roadmap

Language is **Rust** — see `spec.md` and ADR-0005 / ADR-0011. Authoritative milestones: [`roadmap.md`](roadmap.md).
