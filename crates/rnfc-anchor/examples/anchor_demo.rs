//! Anchor demo: shows chain rewrite and truncation detected.
//! Run: cargo run -p rnfc-anchor --example anchor_demo

use rnfc_anchor::{anchor_chain, verify_anchor, AnchorKey, AnchorStore, VerifyResult, KEY_LEN};
use rnfc_certchain::{CertChain, SafetyState};

fn chain_n(n: usize) -> CertChain {
    let mut c = CertChain::new(1, "anchor_demo");
    for i in 0..n {
        c.append(i as f32, 1, 0xA0 + i as u64, SafetyState::Nominal)
            .unwrap();
    }
    c
}

fn hex(b: &[u8]) -> String {
    b.iter().take(8).map(|x| format!("{:02x}", x)).collect()
}

fn main() {
    println!("=== Anchor Demo: Detecting Chain Rewrite ===\n");

    let key = AnchorKey::new([0x42; KEY_LEN]);
    let chain = chain_n(3);
    let mut store = AnchorStore::new();

    println!(
        "[1] Legitimate chain: 3 blocks, root=0x{:016x}",
        chain.root_hash()
    );
    let anchor = anchor_chain(&chain, &key, 1_000_000);
    println!(
        "    anchor: block_count={} hash=0x{:016x}",
        anchor.block_count, anchor.chain_hash_at_block
    );
    println!("    anchor_hash = {}", hex(&anchor.anchor_hash));
    store.store(anchor.clone());

    println!("\n[2] Verify original chain against anchor");
    let r = verify_anchor(&chain, &anchor, &key);
    println!(
        "    verify = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "PASS"
        } else {
            "FAIL"
        }
    );
    assert_eq!(r, VerifyResult::Ok);

    println!("\n[3] ATTACK: Attacker rewrites ENTIRE chain with forged history");
    let mut forged = CertChain::new(1, "anchor_demo");
    forged.append(1.0, 1, 0xDEAD, SafetyState::Nominal).unwrap();
    forged.append(2.0, 1, 0xBEEF, SafetyState::Nominal).unwrap();
    forged.append(3.0, 1, 0xCAFE, SafetyState::Nominal).unwrap();
    println!(
        "    forged chain: 3 blocks, root=0x{:016x}",
        forged.root_hash()
    );
    println!(
        "    (internally consistent, would pass naive verify: {})",
        forged.verify()
    );
    let r = verify_anchor(&forged, &anchor, &key);
    println!(
        "    verify against anchor = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::ChainRewritten);

    println!("\n[4] ATTACK: Attacker truncates chain (hides final block)");
    let truncated = chain_n(1);
    println!("    truncated chain: 1 block");
    let r = verify_anchor(&truncated, &anchor, &key);
    println!(
        "    verify against anchor = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::ChainShortened);

    println!("\n[5] ATTACK: Attacker without the anchor key");
    let wrong_key = AnchorKey::new([0xAA; KEY_LEN]);
    let r = verify_anchor(&chain, &anchor, &wrong_key);
    println!(
        "    verify with wrong key = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::BadSignature);

    println!("\n[6] Legitimate extension: add block after anchor");
    let mut extended = chain.clone();
    extended
        .append(99.0, 1, 0xFF, SafetyState::Nominal)
        .unwrap();
    let r = verify_anchor(&extended, &anchor, &key);
    println!(
        "    extended chain (4 blocks) vs old anchor = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "PASS"
        } else {
            "FAIL"
        }
    );
    assert_eq!(r, VerifyResult::Ok);

    println!("\n=== SUMMARY ===");
    println!("  Original chain:       PASS");
    println!("  Rewritten chain:      REJECTED (ChainRewritten)");
    println!("  Truncated chain:      REJECTED (ChainShortened)");
    println!("  Wrong key:            REJECTED (BadSignature)");
    println!("  Legitimate extension: PASS");
    println!(
        "\n[OK] 3/3 attacks rejected. Anchors stored: {}",
        store.count()
    );
}
