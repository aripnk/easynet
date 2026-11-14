//! `easynet.toml` schema from spec §9. Strings only; the CLI writes the file.

use serde::Deserialize;

/// Parsed `easynet.toml`.
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Config {
    pub node: Node,
    pub discovery: Discovery,
    pub relay: Relay,
    pub admin: Admin,
}

/// `[node]`
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Node {
    pub listen_port: u16,
    pub tun_name: String,
    pub tun_mtu: u16,
    pub state_dir: String,
}

/// `[discovery]`
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Discovery {
    pub stun_servers: Vec<String>,
    pub enable_upnp: bool,
    pub keepalive_secs: u64,
}

/// `[relay]`
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Relay {
    pub allow_relay: bool,
    pub relay_max_bps: u64,
    pub relay_burst_bytes: u64,
}

/// `[admin]`
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Admin {
    pub bind_port: u16,
    pub webhook_url: String,
    pub webhook_secret: String,
}

/// Unix sample from spec §9. Bytes are written unchanged by `easynet config init`.
pub fn default_toml() -> &'static str {
    "\
[node]
listen_port = 51820
tun_name = \"easynet0\"          # ignored on macOS (uses utun)
tun_mtu = 1380
state_dir = \"/var/lib/easynet\" # %ProgramData%\\EasyNet\\state on Windows

[discovery]
stun_servers = [
  \"stun.l.google.com:19302\",
  \"stun.cloudflare.com:3478\",
]
enable_upnp = true
keepalive_secs = 15

[relay]
allow_relay = true             # forced false automatically on iOS/Android
relay_max_bps = 1250000        # bytes/s; 10 Mbps
relay_burst_bytes = 2097152    # 2 MB burst

[admin]
bind_port = 8080
webhook_url = \"\"               # empty = do not POST
webhook_secret = \"\"            # HMAC-SHA256 key for the raw webhook body
"
}

/// Parse a config document. No filesystem access.
pub fn parse_toml(text: &str) -> Result<Config, String> {
    toml::from_str(text).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::{default_toml, parse_toml};

    #[test]
    fn default_sample_matches_spec_numbers() {
        let cfg = parse_toml(default_toml()).expect("sample parses");
        assert_eq!(cfg.node.listen_port, 51820);
        assert!(cfg.relay.allow_relay);
        assert_eq!(cfg.relay.relay_max_bps, 1_250_000);
        assert_eq!(cfg.relay.relay_burst_bytes, 2_097_152);
    }

    #[test]
    fn reject_truncated_document() {
        let err = parse_toml("[node]\nlisten_port = 1\n").expect_err("missing tables");
        assert!(!err.is_empty());
    }
}
