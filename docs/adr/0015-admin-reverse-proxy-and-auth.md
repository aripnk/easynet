# ADR-0015: Loopback reverse proxy, local IPC, and admin HTTP shapes

- **Status:** Accepted
- **Date:** 2026-11-14
- **Supersedes:** the "proxy or 302" sentence in ADR-0004 §3 (still cited for who may bind by ADR-0008). Who binds which address is unchanged.

## Context

"Proxy or 302" is two products. A redirect to `http://100.64.x.x:8080` breaks `SameSite=Strict` cookies in phone webviews and puts a mesh IP in the address bar. The admin JSON routes had no bodies, the first-boot bearer had no lifetime, and Windows had no IPC path. Owner locked this on 2026-11-14.

## Decision

1. On a node that is not serving the admin portal, `127.0.0.1:8080` is an HTTP **reverse proxy** to `http://<lowest-node_id ALIVE admin VIP>:8080`. It is not a 302. An admin node serves the portal itself on that loopback address. The browser `Host` stays `127.0.0.1:8080`.
2. Local admin IPC, same HTTP routes, admin authority, no cookie:
   - Linux and macOS: Unix socket `/run/easynet/admin.sock` mode `0600`.
   - Windows: named pipe `\\.\pipe\easynet-admin`, DACL limited to Administrators and SYSTEM.
3. `easynet init` writes 32 random bytes, hex-encoded, and stores only `SHA-256` plus a unix expiry in `state_dir/bootstrap_token.sha256`. The token lasts **24 hours** and is **single-use**. `POST /api/v1/auth/bootstrap` checks it, sets cookie `easynet_sess` (`HttpOnly; SameSite=Strict; Path=/; Max-Age=43200`), and deletes the file before the success response.
4. Request and response JSON, the approve/reject/revoke routes, and the HMAC webhook are [api.md §8](../api.md#8-admin-http-api-m4). The webhook header is `X-EasyNet-Signature: sha256=<hex>` over the raw body with `webhook_secret`. An empty `webhook_url` means do not POST.

## Consequences

- M4 cards must not add a redirect, a second cookie name, or a JSON field that §8 does not list.
- `GET /api/v1/status`, `GET /api/v1/me/cert`, and `GET /api/v1/topology` stay member-readable. Their JSON keys are still only the nouns in design §5; M4-00 writes those three objects into §8 before any handler card.

## Alternatives considered

- **302 to the admin VIP** — cookie and webview failure; rejected.
- **Unix socket on Windows** — not a Windows IPC; rejected.
