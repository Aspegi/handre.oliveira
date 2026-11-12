//! Alert demo: 4 altitude levels, 3 channels each.
//! Run: cargo run -p rnfc-alert --example alert_demo

use rnfc_alert::{AlertLevel, AlertManager};
use rnfc_certchain::CertChain;

fn show(a: &rnfc_alert::Alert, m: &mut AlertManager, c: &mut CertChain) {
    let st = a.to_statustext();
    let severity = st[0];
    let text = String::from_utf8_lossy(&st[1..]);
    let haptic = a.to_haptic_strong();
    let proof = m.record_in_chain(c, a);
    println!("  level={:?}", a.level);
    println!("    MAVLink STATUSTEXT: severity={} text=\"{}\"", severity, text);
    println!("    Haptic strong:     {}", haptic);
    println!("    Chain proof:       0x{:016x}", proof);
}

fn main() {
    println!("=== Alert Demo: Altitude Envelope Violations ===\n");
    println!("Envelope: 5.0m – 120.0m (EASA BVLOS)\n");

    let mut mgr = AlertManager::new(5.0, 120.0);
    let mut chain = CertChain::new(1, "alert_demo");

    println!("[t=0.0s] altitude = 50.0m (inside)");
    if let Some(a) = mgr.check_altitude(50.0, 0.0) { show(&a, &mut mgr, &mut chain); }
    else { println!("  no alert (inside envelope)\n"); }

    println!("[t=1.0s] altitude = 110.0m (near max)");
    if let Some(a) = mgr.check_altitude(110.0, 1.0) { show(&a, &mut mgr, &mut chain); }

    println!("\n[t=2.0s] altitude = 125.0m (exceeds max)");
    if let Some(a) = mgr.check_altitude(125.0, 2.0) { show(&a, &mut mgr, &mut chain); }

    println!("\n[t=3.0s] altitude = 135.0m (exceeds by >10%)");
    if let Some(a) = mgr.check_altitude(135.0, 3.0) {
        show(&a, &mut mgr, &mut chain);
        if a.level == AlertLevel::Emergency {
            println!("    → EmergencyLand triggered by Failsafe");
        }
    }

    println!("\n=== SUMMARY ===");
    println!("  Alerts emitted: {}", mgr.emitted);
    println!("  Chain: {} blocks, verified={}", chain.block_count(), chain.verify());
    println!("\nThree channels per alert: MAVLink (GCS) + haptic (pilot) + chain (proof).");
}
