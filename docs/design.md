# EasyNet — Admin portal & token UX design

**Status:** Living. Tokens, screens, states. Component essays belong in code review, not here.
User-visible strings: [`../uicopy/en.json`](../uicopy/en.json). Cards must name copy keys; they must not invent sentences.

## 1. Principles

1. **No master / leader IP.** SWIM gossip, not Raft — any `ALIVE` admin portal is valid. Approving on A or B has the same effect.
2. **Bind rules.** (a) Every node: `127.0.0.1:8080` reverse proxy (not a 302), or the portal itself when this node is the chosen admin. (b) `AdminCert` nodes only: portal on their own VIP `:8080`. (c) Member VIP `:8080` must not serve the portal. (d) Never `0.0.0.0` or a physical NIC.
3. **Auth is cryptographic.** Source VIP alone is insufficient. Mutating routes require a valid `AdminCert` session (challenge-response → HttpOnly cookie) or local Unix socket. [spec §5](spec.md), [api.md §8](api.md).
4. **One job per view.** Topology, pending joins, and revoke are separate views.
5. **Tokens are the pairing UX.** QR + pasteable `easynet://invite|join|roster/…` strings.
6. **No special VIP in chrome.** Never hard-code `100.64.0.1`. Status shows this node's VIP and lists `admin_urls`.
7. **Calm ops UI.** Dense, readable, desktop-first for M4; mobile app wrappers are M5.

## 1b. How to open the portal

| You type | From which device | What happens |
|---|---|---|
| `http://127.0.0.1:8080` | **Any** mesh device (admin or member, including phones/laptops) | Local `easynetd` reverse-proxies to the lowest-`node_id` `ALIVE` admin. The browser host stays `127.0.0.1:8080`. Not a redirect. |
| `http://<admin-VIP>:8080` | Any device that can reach that VIP over the mesh | Full admin portal on that admin node (A and B are equivalent) |
| `http://<member-VIP>:8080` | — | **Does not work** — members do not bind `:8080` on the mesh interface |

There is no “point at the master IP” step.

**Q15 (deferred — out of M4):** mesh DNS `admin.easynet` is post-M4 / M5
nice-to-have ([api.md §10](api.md#10-deferred-q15-mesh-dns)). M4 zero-memorization
UX is `http://127.0.0.1:8080` on every node (reverse proxy) plus `easynet status` admin URL list.

## 2. Tokens (CSS variables)

| Token | Role | Initial value |
|---|---|---|
| `--color-bg` | Page background | `#0f1419` |
| `--color-surface` | Panels / tables | `#1a222c` |
| `--color-text` | Primary text | `#e7ecf1` |
| `--color-muted` | Secondary text | `#8b9aab` |
| `--color-accent` | Links / primary actions | `#3d9cf0` |
| `--color-danger` | Revoke / destructive | `#e85d5d` |
| `--color-ok` | Alive / success | `#3ecf8e` |
| `--color-warn` | Suspect / pending | `#e6b84d` |
| `--font-sans` | UI | `"IBM Plex Sans", "Segoe UI", sans-serif` |
| `--font-mono` | Keys, IPs, tokens | `"IBM Plex Mono", ui-monospace, monospace` |
| `--radius` | Controls | `6px` |
| `--space-1..4` | Spacing scale | `4px / 8px / 16px / 32px` |

Dark ops console is intentional. No purple glow stacks or pill-badge clutter.

## 3. Screens

| ID | Route | Who | Purpose |
|---|---|---|---|
| `S-login` | `/login` | unauthenticated | Challenge / paste signed nonce / first-boot bearer |
| `S-status` | `/` | member or admin session | This node: mesh id, VIP, role, counts, admin URL list |
| `S-peers` | `/peers` | member or admin | Peer table (VIP, online, path mode) |
| `S-pending` | `/pending` | **admin only** | Approve / reject (hidden for members) |
| `S-peer-detail` | `/peers/:nodeId` | member read / admin revoke | Endpoint; revoke control admin-only |
| `S-topology` | `/topology` | member or admin | Direct vs relay edges |
| `S-tokens` | `/tokens` | **admin only** | Roster QR after accept |

CLI: `easynet status|approve|revoke|qr …`. Local admin IPC: `/run/easynet/admin.sock` (`0600`) on Linux and macOS; `\\.\pipe\easynet-admin` on Windows (Administrators and SYSTEM). Same HTTP routes, no cookie.

**Q14 resolved:**

- **Member session:** Ed25519 challenge-response with valid `PeerCert` → read-only `GET` status/peers/topology/own cert expiry. Pending joins and mutations → `403` / hidden in UI.
- **Admin session:** `AdminCert` (or Unix socket) → full approve/reject/revoke/webhook.
- **Bind rules unchanged:** members never host VIP `:8080`; any node may use `127.0.0.1:8080` helper.

## 4. Auth UX (Q7 locked)

1. **Localhost / CLI:** Unix socket or Windows named pipe; no browser cookie required for CLI.
2. **Browser:** `POST /api/v1/auth/challenge` → show nonce; user completes signature via local daemon helper or pasted CLI token → `POST /api/v1/auth/verify` → cookie `easynet_sess` (`HttpOnly; SameSite=Strict; Path=/; Max-Age=43200`). The UI stays on `127.0.0.1:8080` because loopback is a reverse proxy.
3. **First boot:** `easynet init` prints a 64-char hex token (32 bytes). `POST /api/v1/auth/bootstrap` exchanges it once within 24 hours (`auth.first_boot_help`). The SHA-256 file is deleted before the success response.

## 5. Screen layouts (detail)

### S-status

- Brand: `brand.name` as primary header.
- Supporting line: `status.subtitle`.
- Definition list: mesh id, node id (short), VIP, role, alive/peer counts, linked `admin_urls`.
- CTA if admin and pending>0: `status.cta_pending` → `S-pending`.

### S-pending / S-tokens / S-peers

Empty, busy, and error states use the copy keys. The Accept button (`pending.accept`) calls `POST /api/v1/pending/{peer_id_hex}/approve` and then shows the roster QR (`easynet://roster/…`). Revoke uses `peer.revoke_confirm`.

## 6. States

| State | UI |
|---|---|
| **Empty** | One muted sentence from copy |
| **Busy** | Disable mutating controls + spinner label |
| **Error** | Banner with `message_key`; no stack traces |
| **Unauthorized** | `auth.denied` |

## 7. Accessibility

Focus visible; QR `alt` = `tokens.qr_alt`; destructive confirm required; WCAG AA body contrast; usable at 320px width.

## 8. What the UI must not show

- Private keys, root key material, Noise session keys, first-boot bearer after use.
- Public WAN IP as portal address.
- Hard-coded `100.64.0.1`.
- Marketing caps / upgrade CTAs.

## 9. Pairing flows

1. **Bootstrap:** exchange `easynet://invite/…` QR; punch until ACK.
2. **Join:** joiner shows `easynet://join/…`; admin accept → `easynet://roster/…`.
3. **Roam:** automatic incarnation bump; no M4 toast required.
