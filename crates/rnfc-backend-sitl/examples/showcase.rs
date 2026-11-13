//! RNFC Showcase — runs every capability in sequence, produces a report.

use rnfc_alert::{AlertLevel, AlertManager};
use rnfc_anchor::{anchor_chain, verify_anchor, AnchorKey, VerifyResult as AnchorVerdict, KEY_LEN};
use rnfc_certchain::{CertChain, SafetyState};
use rnfc_counterfactual::{CounterfactualEngine, Outcome, TrajectorySample};
use rnfc_decision::{DecisionContext, DecisionEngine};
use rnfc_fdir::SpoofingDetector;
use rnfc_mavsign::{sign_message, verify_message, SigningKey, VerifyResult as MavVerdict, KEY_LEN as MAV_KEY_LEN};
use rnfc_omni::{OmniAction, OmniHealth, OmniSensorSuite};
use rnfc_scc::{compile_requirements, generate_gsn, parse_requirements};
use rnfc_watch::WatchManager;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RNFC v6.9.0 — SHOWCASE ===\n");
    let mut report = String::from("# RNFC v6.9.0 — Showcase Report\n\n| # | Capability | Status |\n|---|---|---|\n");
    let mut passed = 0u32;
    let mut total = 0u32;

    // 1. FDIR: SpoofingDetector detects 10 rad divergence
    total += 1;
    let mut spoofing = SpoofingDetector::new();
    let mut detected = false;
    for i in 0..50 {
        let (mag, gyro) = if i >= 20 { (10.0, 0.0) } else { (0.0, 0.0) };
        if spoofing.update(mag, gyro) { detected = true; break; }
    }
    if detected { passed += 1; println!("[1/9] FDIR spoofing: detected  OK"); }
    else { println!("[1/9] FDIR spoofing: NOT detected  FAIL"); }
    report.push_str(&format!("| 1 | FDIR spoofing | {} |\n", if detected { "PASS" } else { "FAIL" }));

    // 2. Decision: obstacle forces detour
    total += 1;
    let de = DecisionEngine::new();
    let ctx = DecisionContext { wind: [0.0, 0.0], obstacles: [([50.0, 0.0], 8.0); 3], battery_pct: 100.0 };
    let d = de.decide([0.0, 0.0], [100.0, 0.0], &ctx);
    let detour = d.chosen != "direct";
    if detour { passed += 1; println!("[2/9] Decision: chose {}  OK", d.chosen); }
    else { println!("[2/9] Decision: stayed direct  FAIL"); }
    report.push_str(&format!("| 2 | Decision detour | {} |\n", if detour { "PASS" } else { "FAIL" }));

    // 3. Watch: motion -> report
    total += 1;
    let mut watch = WatchManager::new();
    let mut chain = CertChain::new(1, "showcase");
    let static_f: Vec<u8> = vec![128u8; 64 * 48 * 3];
    let mut motion_f = static_f.clone(); for i in 5000..8000 { motion_f[i] = 255; }
    let _ = watch.on_frame(&static_f, 0.0, [0.0, 0.0], 10.0, 0.0, 0.0, &mut chain);
    let w = watch.on_frame(&motion_f, 1.0, [0.0, 0.0], 10.0, 135.0, 18.0, &mut chain);
    let watch_ok = w.is_some();
    if watch_ok { passed += 1; println!("[3/9] Watch: motion->report  OK"); }
    else { println!("[3/9] Watch: no report  FAIL"); }
    report.push_str(&format!("| 3 | Watch motion report | {} |\n", if watch_ok { "PASS" } else { "FAIL" }));

    // 4. Omni: camera failure -> Hover
    total += 1;
    let mut suite = OmniSensorSuite::new(3, 120.0);
    suite.fail_camera(0);
    let ring = suite.sense(&[]);
    let omni_ok = ring.safety_action() == OmniAction::Hover && ring.health == OmniHealth::Degraded;
    if omni_ok { passed += 1; println!("[4/9] Omni: camera fail->Hover  OK"); }
    else { println!("[4/9] Omni: no hover  FAIL"); }
    report.push_str(&format!("| 4 | Omni camera fail->Hover | {} |\n", if omni_ok { "PASS" } else { "FAIL" }));

    // 5. Alert: altitude violation -> Emergency
    total += 1;
    let mut am = AlertManager::new(5.0, 120.0);
    let a = am.check_altitude(135.0, 1.0);
    let alert_ok = a.as_ref().map(|x| x.level == AlertLevel::Emergency).unwrap_or(false);
    if alert_ok { passed += 1; println!("[5/9] Alert: 135m->Emergency  OK"); }
    else { println!("[5/9] Alert: no emergency  FAIL"); }
    report.push_str(&format!("| 5 | Alert altitude | {} |\n", if alert_ok { "PASS" } else { "FAIL" }));

    // 6. MAVLink signing: 3 attacks rejected
    total += 1;
    let key = SigningKey::new([0x42; MAV_KEY_LEN]);
    let sig = sign_message(b"ARM", &key, 1_000_000, 0);
    let r1 = verify_message(b"ARM", &sig, &key, 1_000_000, rnfc_mavsign::DEFAULT_WINDOW_US);
    let r2 = verify_message(b"DISARM", &sig, &key, 1_000_000, rnfc_mavsign::DEFAULT_WINDOW_US);
    let r3 = verify_message(b"ARM", &sig, &key, 20_000_000, rnfc_mavsign::DEFAULT_WINDOW_US);
    let mav_ok = r1 == MavVerdict::Ok && r2 == MavVerdict::BadSignature && r3 == MavVerdict::ReplayTooOld;
    if mav_ok { passed += 1; println!("[6/9] MAVSign: 3/3 attacks rejected  OK"); }
    else { println!("[6/9] MAVSign: attack not rejected  FAIL"); }
    report.push_str(&format!("| 6 | MAVLink signing | {} |\n", if mav_ok { "PASS" } else { "FAIL" }));

    // 7. Anchor: chain rewrite detected
    total += 1;
    let akey = AnchorKey::new([0x42; KEY_LEN]);
    let anch = anchor_chain(&chain, &akey, 1000);
    let mut forged = CertChain::new(1, "showcase");
    forged.append(9.0, 1, 0xDEAD, SafetyState::Nominal).unwrap();
    let anch_ok = verify_anchor(&forged, &anch, &akey) == AnchorVerdict::ChainRewritten;
    if anch_ok { passed += 1; println!("[7/9] Anchor: rewrite detected  OK"); }
    else { println!("[7/9] Anchor: not detected  FAIL"); }
    report.push_str(&format!("| 7 | Anchor chain rewrite | {} |\n", if anch_ok { "PASS" } else { "FAIL" }));

    // 8. CSR: counterfactual proof
    total += 1;
    let traj: Vec<TrajectorySample> = (0..=10).map(|i| TrajectorySample {
        t_s: i as f32, pos: [5.0 * i as f32, 0.15 * i as f32], vel: [5.0, 0.15],
        wind: [0.0, 0.15], altitude: 10.0,
    }).collect();
    let cfe = CounterfactualEngine::new();
    let cf = cfe.what_if_wind(&traj, [50.0, 0.0], 20.0);
    let pr = cfe.prove(&chain, &cf);
    let csr_ok = cfe.verify(&pr, &chain, &cf) && matches!(cf.outcome, Outcome::Failed { .. });
    if csr_ok { passed += 1; println!("[8/9] CSR: wind 20x->Failed+proof  OK"); }
    else { println!("[8/9] CSR: proof failed  FAIL"); }
    report.push_str(&format!("| 8 | CSR counterfactual | {} |\n", if csr_ok { "PASS" } else { "FAIL" }));

    // 9. SCC: 3 req -> 3 blocks
    total += 1;
    let input = "speed_max: 10.0 HIGH\naltitude_range: 5.0 120.0 HIGH\nobstacle_clearance: 2.0 MEDIUM\n";
    let reqs = parse_requirements(input).unwrap_or_default();
    let chain2 = compile_requirements(&reqs).unwrap_or_else(|_| CertChain::new(1, "fail"));
    let scc_ok = reqs.len() == 3 && chain2.block_count() == 3 && chain2.verify();
    if scc_ok { passed += 1; println!("[9/9] SCC: 3 req->3 blocks  OK"); }
    else { println!("[9/9] SCC: artefacts missing  FAIL"); }
    let _ = generate_gsn(&reqs[0], 0);
    report.push_str(&format!("| 9 | SCC 3 req->3 blocks | {} |\n", if scc_ok { "PASS" } else { "FAIL" }));

    println!("\n=== Result: {}/{} capabilities passed ===", passed, total);
    report.push_str(&format!("\n## Result: {}/{} capabilities passed\n", passed, total));
    if passed == total { report.push_str("\n**ALL CAPABILITIES DEMONSTRATED**\n"); }
    fs::write("showcase_report.md", &report)?;
    println!("[OK] showcase_report.md written ({} bytes)", report.len());
    Ok(())
}
