#![cfg_attr(not(test), no_std)]
//! Anchor — external anchoring of the CertChain head.
//!
//! A hash chain is only tamper-evident against naive edits. An attacker
//! with local write access can rewrite the ENTIRE chain with a forged but
//! internally-consistent history. To detect this, the chain head is
//! periodically anchored to an external, trusted store (server, blockchain,
//! transparency log). If the chain is rewritten, the recomputed anchor will
//! not match the externally stored one.
//!
//! Anchor = SHA-256(block_count || block_hash_at_anchor || timestamp || key)
//! The verifier recomputes this from the current chain and compares.

extern crate alloc;
use alloc::vec::Vec;

use rnfc_certchain::CertChain;
use sha2::{Digest, Sha256};

pub const KEY_LEN: usize = 32;
pub const ANCHOR_HASH_LEN: usize = 32;

#[derive(Clone)]
pub struct AnchorKey { bytes: [u8; KEY_LEN] }

impl AnchorKey {
    pub fn new(bytes: [u8; KEY_LEN]) -> Self { Self { bytes } }
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] { &self.bytes }
}

#[derive(Clone, Debug)]
pub struct Anchor {
    pub block_count: usize,
    pub chain_hash_at_block: u64,
    pub timestamp_us: u64,
    pub anchor_hash: [u8; ANCHOR_HASH_LEN],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyResult {
    Ok,
    ChainShortened,
    ChainRewritten,
    BadSignature,
}

/// Anchor the current state of the chain at a given timestamp.
pub fn anchor_chain(chain: &CertChain, key: &AnchorKey, timestamp_us: u64) -> Anchor {
    let block_count = chain.block_count();
    let chain_hash_at_block = if block_count > 0 {
        chain.blocks[block_count - 1].hash
    } else { 0 };

    let mut h = Sha256::new();
    h.update(&(block_count as u64).to_le_bytes());
    h.update(&chain_hash_at_block.to_le_bytes());
    h.update(&timestamp_us.to_le_bytes());
    h.update(key.as_bytes());
    let digest = h.finalize();
    let mut anchor_hash = [0u8; ANCHOR_HASH_LEN];
    anchor_hash.copy_from_slice(&digest);

    Anchor { block_count, chain_hash_at_block, timestamp_us, anchor_hash }
}

/// Verify an anchor against the current chain.
pub fn verify_anchor(chain: &CertChain, anchor: &Anchor, key: &AnchorKey) -> VerifyResult {
    // 1. Recompute anchor hash from metadata + key
    let mut h = Sha256::new();
    h.update(&(anchor.block_count as u64).to_le_bytes());
    h.update(&anchor.chain_hash_at_block.to_le_bytes());
    h.update(&anchor.timestamp_us.to_le_bytes());
    h.update(key.as_bytes());
    let expected = h.finalize();
    if expected[..] != anchor.anchor_hash[..] {
        return VerifyResult::BadSignature;
    }

    // 2. Chain must still contain the anchored block
    if chain.block_count() < anchor.block_count {
        return VerifyResult::ChainShortened;
    }

    // 3. The anchored block must still have the same hash
    if anchor.block_count > 0 {
        let block_hash = chain.blocks[anchor.block_count - 1].hash;
        if block_hash != anchor.chain_hash_at_block {
            return VerifyResult::ChainRewritten;
        }
    }

    VerifyResult::Ok
}

/// Local simulated external anchor store (in real life: server / blockchain / log).
pub struct AnchorStore {
    pub anchors: Vec<Anchor>,
}

impl AnchorStore {
    pub fn new() -> Self { Self { anchors: Vec::new() } }
    pub fn store(&mut self, a: Anchor) { self.anchors.push(a); }
    pub fn latest(&self) -> Option<&Anchor> { self.anchors.last() }
    pub fn count(&self) -> usize { self.anchors.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnfc_certchain::SafetyState;

    fn key() -> AnchorKey { AnchorKey::new([0x42; KEY_LEN]) }

    fn chain_n(n: usize) -> CertChain {
        let mut c = CertChain::new(1, "anchor_test");
        for i in 0..n {
            c.append(i as f32, 1, 0xA0 + i as u64, SafetyState::Nominal).unwrap();
        }
        c
    }

    #[test]
    fn anchor_empty_chain() {
        let c = CertChain::new(1, "empty");
        let a = anchor_chain(&c, &key(), 1000);
        assert_eq!(a.block_count, 0);
        assert_eq!(a.chain_hash_at_block, 0);
    }

    #[test]
    fn anchor_single_block() {
        let c = chain_n(1);
        let a = anchor_chain(&c, &key(), 1000);
        assert_eq!(a.block_count, 1);
        assert_eq!(a.chain_hash_at_block, c.blocks[0].hash);
    }

    #[test]
    fn anchor_deterministic() {
        let c = chain_n(3);
        let a1 = anchor_chain(&c, &key(), 1000);
        let a2 = anchor_chain(&c, &key(), 1000);
        assert_eq!(a1.anchor_hash, a2.anchor_hash);
    }

    #[test]
    fn verify_accepts_valid() {
        let c = chain_n(3);
        let a = anchor_chain(&c, &key(), 1000);
        assert_eq!(verify_anchor(&c, &a, &key()), VerifyResult::Ok);
    }

    #[test]
    fn verify_accepts_extended_chain() {
        let mut c = chain_n(3);
        let a = anchor_chain(&c, &key(), 1000);
        c.append(99.0, 1, 0xFF, SafetyState::Nominal).unwrap();
        // Adding blocks after anchor does not invalidate it
        assert_eq!(verify_anchor(&c, &a, &key()), VerifyResult::Ok);
    }

    #[test]
    fn verify_rejects_shortened_chain() {
        let c = chain_n(3);
        let a = anchor_chain(&c, &key(), 1000);
        let short = chain_n(1);
        assert_eq!(verify_anchor(&short, &a, &key()), VerifyResult::ChainShortened);
    }

    #[test]
    fn verify_rejects_rewritten_chain() {
        let c = chain_n(3);
        let a = anchor_chain(&c, &key(), 1000);
        // Attacker rewrites the chain with forged but consistent blocks
        let mut forged = CertChain::new(1, "anchor_test");
        forged.append(1.0, 1, 0xDEAD, SafetyState::Nominal).unwrap();
        forged.append(2.0, 1, 0xBEEF, SafetyState::Nominal).unwrap();
        forged.append(3.0, 1, 0xCAFE, SafetyState::Nominal).unwrap();
        assert_eq!(verify_anchor(&forged, &a, &key()), VerifyResult::ChainRewritten);
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let c = chain_n(3);
        let a = anchor_chain(&c, &key(), 1000);
        let wrong = AnchorKey::new([0xAA; KEY_LEN]);
        assert_eq!(verify_anchor(&c, &a, &wrong), VerifyResult::BadSignature);
    }

    #[test]
    fn verify_rejects_tampered_anchor() {
        let c = chain_n(3);
        let mut a = anchor_chain(&c, &key(), 1000);
        a.anchor_hash[0] ^= 0xFF;
        assert_eq!(verify_anchor(&c, &a, &key()), VerifyResult::BadSignature);
    }

    #[test]
    fn store_and_retrieve() {
        let c = chain_n(2);
        let mut store = AnchorStore::new();
        let a = anchor_chain(&c, &key(), 1000);
        store.store(a.clone());
        assert_eq!(store.count(), 1);
        assert!(store.latest().is_some());
        assert_eq!(verify_anchor(&c, store.latest().unwrap(), &key()), VerifyResult::Ok);
    }

    #[test]
    fn multiple_anchors_ordered() {
        let mut c = chain_n(1);
        let mut store = AnchorStore::new();
        store.store(anchor_chain(&c, &key(), 1000));
        c.append(2.0, 1, 0xB2, SafetyState::Nominal).unwrap();
        store.store(anchor_chain(&c, &key(), 2000));
        assert_eq!(store.count(), 2);
        assert_eq!(store.anchors[0].timestamp_us, 1000);
        assert_eq!(store.anchors[1].timestamp_us, 2000);
        assert_eq!(store.anchors[1].block_count, 2);
    }

    #[test]
    fn timestamp_preserved() {
        let c = chain_n(2);
        let a = anchor_chain(&c, &key(), 123_456_789);
        assert_eq!(a.timestamp_us, 123_456_789);
    }
}
