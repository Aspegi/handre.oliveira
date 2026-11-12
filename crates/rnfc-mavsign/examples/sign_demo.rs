//! MAVLink signing demo: shows 4 attacks rejected.
//! Run: cargo run -p rnfc-mavsign --example sign_demo

use rnfc_mavsign::{
    parse_setup_signing, sign_message, verify_message, SigningKey, VerifyResult, DEFAULT_WINDOW_US,
    KEY_LEN,
};

fn main() {
    println!("=== MAVLink Signing Demo: 4 Attacks Rejected ===\n");

    let secret = [0x42u8; KEY_LEN];
    let key = SigningKey::new(secret);
    let packet = b"COMMAND_ARM throttle=50%";
    let now_us: u64 = 1_000_000_000;

    println!("[1] Operator signs packet with secret key");
    let sig = sign_message(packet, &key, now_us, 0);
    println!("    signature = {:02x?}", sig.bytes);
    println!("    timestamp = {} us", sig.timestamp_us);

    println!("\n[2] Legitimate reception");
    let r = verify_message(packet, &sig, &key, now_us, DEFAULT_WINDOW_US);
    println!(
        "    verify = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::Ok);

    println!("\n[3] ATTACK 1: Tampered command");
    let tampered = b"COMMAND_ARM throttle=100%";
    let r = verify_message(tampered, &sig, &key, now_us, DEFAULT_WINDOW_US);
    println!(
        "    verify = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::BadSignature);

    println!("\n[4] ATTACK 2: Wrong key (attacker without secret)");
    let attacker_key = SigningKey::new([0xAA; KEY_LEN]);
    let r = verify_message(packet, &sig, &attacker_key, now_us, DEFAULT_WINDOW_US);
    println!(
        "    verify = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::BadSignature);

    println!("\n[5] ATTACK 3: Replay (20s later)");
    let later = now_us + 20_000_000;
    let r = verify_message(packet, &sig, &key, later, DEFAULT_WINDOW_US);
    println!(
        "    verify = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::ReplayTooOld);

    println!("\n[6] ATTACK 4: Future timestamp (clock manipulation)");
    let future_sig = sign_message(packet, &key, now_us + 100_000_000, 0);
    let r = verify_message(packet, &future_sig, &key, now_us, DEFAULT_WINDOW_US);
    println!(
        "    verify = {:?}  {}",
        r,
        if r == VerifyResult::Ok {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );
    assert_eq!(r, VerifyResult::FutureTimestamp);

    println!("\n[7] Provisioning: parse SETUP_SIGNING payload");
    let mut payload = [0u8; 42];
    payload[0] = 1;
    payload[1] = 1;
    for i in 0..32 {
        payload[2 + i] = 0x55;
    }
    if let Some(k2) = parse_setup_signing(&payload) {
        println!("    key provisioned = 0x{:02x}...", k2.as_bytes()[0]);
    }

    println!("\n=== SUMMARY ===");
    println!("  Legitimate command:  ACCEPTED");
    println!("  Tampered command:    REJECTED (BadSignature)");
    println!("  Wrong-key command:   REJECTED (BadSignature)");
    println!("  Replay command:      REJECTED (ReplayTooOld)");
    println!("  Future timestamp:    REJECTED (FutureTimestamp)");
    println!("\n[OK] 4/4 attacks rejected.");
}
