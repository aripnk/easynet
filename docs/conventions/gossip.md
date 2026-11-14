# Gossip / SWIM (easynet-core)

Read this when editing `crates/easynet-core/src/swim.rs` (or equivalent path).

- Two-byte `0x02` header, then postcard. Timers are `PROTOCOL_PERIOD` 1000 ms,
  `PING_TIMEOUT` 300 ms, `INDIRECT_K` 2, `SUSPECT_TIMEOUT` 5000 ms, at most
  8 deltas ([api.md §4](../api.md)). Do not restore the old 20-byte header.
- Membership mutations require a valid admin signature chaining through
  `AdminCert` to `root_pub`. Revoke beats cert. Endpoint merge uses signed
  `incarnation`. Relay eligibility: `role == Member` ∧ `relay_capable`
  ([ADR-0009](../adr/0009-relay-two-knob-policy.md)).
  Link: [ADR-0014](../adr/0014-swim-failure-detector.md).

```rust
// BAD: peer_table.insert(cert); // no signature / AdminCert check
```

```rust
// GOOD: verify_admin_chain(&cert)?; if revoke_set.contains(key) { return };
//       merge_endpoint_by_incarnation(record)
```
