//! Public halves of stored secrets. No disk and no RNG.

use ed25519_dalek::SigningKey;
use x25519_dalek::{PublicKey, StaticSecret};

/// Ed25519 verifying key for a 32-byte secret.
pub fn ed25519_public(secret: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(secret).verifying_key().to_bytes()
}

/// X25519 public key for a 32-byte static secret.
pub fn x25519_public(secret: &[u8; 32]) -> [u8; 32] {
    PublicKey::from(&StaticSecret::from(*secret)).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::{ed25519_public, x25519_public};

    #[test]
    fn public_halves_are_stable() {
        let secret = [9u8; 32];
        assert_eq!(ed25519_public(&secret), ed25519_public(&secret));
        assert_eq!(x25519_public(&secret), x25519_public(&secret));
        assert_ne!(ed25519_public(&secret), x25519_public(&secret));
    }
}
