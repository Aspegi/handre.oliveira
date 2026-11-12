#![cfg_attr(not(test), no_std)]
//! MAVLink 2 message signing (authentication + anti-replay).
//!
//! MAVLink without signing allows command injection and telemetry spoofing.
//! This crate implements the MAVLink 2 signing scheme:
//!   signature[0..6] = SHA-256(packet || secret_key || timestamp || link_id)[0..6]
//!   signature[6..13] = link_id(1) || timestamp(6, little-endian)
//!
//! The verifier checks the signature AND that the timestamp is within
//! a window (default ±10s) to prevent replay attacks.

use sha2::{Digest, Sha256};

pub const SIGNATURE_LEN: usize = 13;
pub const KEY_LEN: usize = 32;
pub const DEFAULT_WINDOW_US: u64 = 10_000_000; // 10 seconds

#[derive(Clone)]
pub struct SigningKey {
    bytes: [u8; KEY_LEN],
}

impl SigningKey {
    pub fn new(bytes: [u8; KEY_LEN]) -> Self {
        Self { bytes }
    }

    pub fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() != KEY_LEN {
            return None;
        }
        let mut b = [0u8; KEY_LEN];
        b.copy_from_slice(s);
        Some(Self { bytes: b })
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.bytes
    }
}

pub struct Signature {
    pub bytes: [u8; SIGNATURE_LEN],
    pub link_id: u8,
    pub timestamp_us: u64,
}

impl Signature {
    pub fn from_bytes(b: [u8; SIGNATURE_LEN]) -> Self {
        let link_id = b[6];
        let mut ts = [0u8; 6];
        ts.copy_from_slice(&b[7..13]);
        let mut ts8 = [0u8; 8];
        ts8[..6].copy_from_slice(&ts);
        let timestamp_us = u64::from_le_bytes(ts8);
        Self {
            bytes: b,
            link_id,
            timestamp_us,
        }
    }
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LEN] {
        self.bytes
    }
}

/// Compute the 13-byte MAVLink 2 signature for a packet.
pub fn sign_message(packet: &[u8], key: &SigningKey, timestamp_us: u64, link_id: u8) -> Signature {
    let mut h = Sha256::new();
    h.update(packet);
    h.update(key.as_bytes());
    h.update(&timestamp_us.to_le_bytes());
    h.update(&[link_id]);
    let digest = h.finalize();

    let mut sig = [0u8; SIGNATURE_LEN];
    sig[0..6].copy_from_slice(&digest[0..6]);
    sig[6] = link_id;
    let ts_bytes = timestamp_us.to_le_bytes();
    sig[7..13].copy_from_slice(&ts_bytes[0..6]);

    Signature {
        bytes: sig,
        link_id,
        timestamp_us,
    }
}

/// Verify a signature against the packet, key, and current time.
/// Rejects if: signature mismatch, OR timestamp outside the anti-replay window.
pub fn verify_message(
    packet: &[u8],
    signature: &Signature,
    key: &SigningKey,
    now_us: u64,
    window_us: u64,
) -> VerifyResult {
    let expected = sign_message(packet, key, signature.timestamp_us, signature.link_id);
    if expected.bytes[0..6] != signature.bytes[0..6] {
        return VerifyResult::BadSignature;
    }
    if signature.timestamp_us > now_us + window_us {
        return VerifyResult::FutureTimestamp;
    }
    if signature.timestamp_us + window_us < now_us {
        return VerifyResult::ReplayTooOld;
    }
    VerifyResult::Ok
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyResult {
    Ok,
    BadSignature,
    FutureTimestamp,
    ReplayTooOld,
}

/// Parse a SETUP_SIGNING message (msgid 256).
/// Returns the key, or None if the payload is malformed.
pub fn parse_setup_signing(payload: &[u8]) -> Option<SigningKey> {
    // MAVLink SETUP_SIGNING: target_system(1) target_component(1)
    // secret_key(32) initial_timestamp(8)
    if payload.len() < 42 {
        return None;
    }
    SigningKey::from_slice(&payload[2..34])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::new([0x42; KEY_LEN])
    }

    #[test]
    fn signature_is_13_bytes() {
        let sig = sign_message(b"hello", &key(), 1_000_000, 0);
        assert_eq!(sig.bytes.len(), SIGNATURE_LEN);
    }

    #[test]
    fn verify_accepts_valid() {
        let k = key();
        let sig = sign_message(b"packet", &k, 5_000_000, 1);
        assert_eq!(
            verify_message(b"packet", &sig, &k, 5_000_000, DEFAULT_WINDOW_US),
            VerifyResult::Ok
        );
    }

    #[test]
    fn verify_rejects_tampered_packet() {
        let k = key();
        let sig = sign_message(b"packet", &k, 5_000_000, 1);
        assert_eq!(
            verify_message(b"PACKET", &sig, &k, 5_000_000, DEFAULT_WINDOW_US),
            VerifyResult::BadSignature
        );
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let k1 = SigningKey::new([0x42; KEY_LEN]);
        let k2 = SigningKey::new([0x43; KEY_LEN]);
        let sig = sign_message(b"packet", &k1, 5_000_000, 1);
        assert_eq!(
            verify_message(b"packet", &sig, &k2, 5_000_000, DEFAULT_WINDOW_US),
            VerifyResult::BadSignature
        );
    }

    #[test]
    fn verify_rejects_replay_too_old() {
        let k = key();
        let sig = sign_message(b"p", &k, 1_000_000, 0);
        assert_eq!(
            verify_message(b"p", &sig, &k, 20_000_000, DEFAULT_WINDOW_US),
            VerifyResult::ReplayTooOld
        );
    }

    #[test]
    fn verify_rejects_future_timestamp() {
        let k = key();
        let sig = sign_message(b"p", &k, 50_000_000, 0);
        assert_eq!(
            verify_message(b"p", &sig, &k, 5_000_000, DEFAULT_WINDOW_US),
            VerifyResult::FutureTimestamp
        );
    }

    #[test]
    fn verify_accepts_within_window() {
        let k = key();
        let sig = sign_message(b"p", &k, 5_000_000, 0);
        assert_eq!(
            verify_message(b"p", &sig, &k, 12_000_000, DEFAULT_WINDOW_US),
            VerifyResult::Ok
        );
    }

    #[test]
    fn signature_deterministic() {
        let s1 = sign_message(b"x", &key(), 100, 2);
        let s2 = sign_message(b"x", &key(), 100, 2);
        assert_eq!(s1.bytes, s2.bytes);
    }

    #[test]
    fn different_link_ids_different_signatures() {
        let s0 = sign_message(b"x", &key(), 100, 0);
        let s1 = sign_message(b"x", &key(), 100, 1);
        assert_ne!(s0.bytes, s1.bytes);
    }

    #[test]
    fn parse_setup_signing_valid() {
        let mut payload = [0u8; 42];
        payload[0] = 1;
        payload[1] = 1;
        for i in 0..32 {
            payload[2 + i] = 0xAB;
        }
        let k = parse_setup_signing(&payload).unwrap();
        assert_eq!(k.as_bytes()[0], 0xAB);
        assert_eq!(k.as_bytes()[31], 0xAB);
    }

    #[test]
    fn parse_setup_signing_too_short() {
        assert!(parse_setup_signing(&[0u8; 10]).is_none());
    }
}
