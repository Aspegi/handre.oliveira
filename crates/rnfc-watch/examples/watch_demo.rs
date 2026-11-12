//! Watch demo: motion detection -> zoom -> photo -> anomaly report.
//! Run: cargo run -p rnfc-watch --example watch_demo

use rnfc_certchain::CertChain;
use rnfc_watch::WatchManager;
use std::fs;
use std::io::Write;

fn static_frame() -> Vec<u8> { vec![128u8; 64 * 48 * 3] }

fn motion_frame() -> Vec<u8> {
    let mut f = static_frame();
    for i in 5000..8000 { f[i] = 255; }
    f
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Watch Demo: Motion -> Zoom -> Photo -> Report ===\n");
    let mut chain = CertChain::new(1, "watch_mission");
    let mut mgr = WatchManager::new();

    println!("[t=0.0s] Drone at [20.0, 10.0] @ 15m, FOV {:.0}°", mgr.zoom.current_fov_deg());
    let _ = mgr.on_frame(&static_frame(), 0.0, [20.0, 10.0], 15.0, 0.0, 0.0, &mut chain);

    println!("[t=1.0s] Motion detected at sector 135°, distance 18m");
    let report = mgr.on_frame(&motion_frame(), 1.0, [22.0, 11.0], 15.0, 135.0, 18.0, &mut chain)
        .ok_or("no report generated")?;

    println!("[t=1.0s] Zoom: FOV {:.0}° (target sector {:.0}°)",
        mgr.zoom.current_fov_deg(), report.zone.angle_deg);
    println!("[t=1.0s] Photo: {} (64x48 RGB PPM)", report.photo_file);
    println!("[t=1.0s] Classification: {}", report.classification);
    println!("[t=1.0s] Proof: 0x{:016x}", report.chain_proof);

    // Write photo
    let photo = rnfc_watch::PhotoCapture::generate(1001, 64, 48);
    fs::write(&report.photo_file, photo.to_ppm())?;

    // Write report
    let mut f = fs::File::create("anomaly_report_001.md")?;
    writeln!(f, "# Anomaly Report #{:03}\n", report.report_id)?;
    writeln!(f, "- **Timestamp**: t={:.1}s", report.timestamp_s)?;
    writeln!(f, "- **Drone position**: [{:.1}, {:.1}] @ {:.0}m",
        report.drone_pos[0], report.drone_pos[1], report.drone_alt_m)?;
    writeln!(f, "- **Detected zone**: sector {:.0}°, distance ~{:.0}m",
        report.zone.angle_deg, report.zone.distance_m)?;
    writeln!(f, "- **Confidence**: {:.2}", report.zone.confidence)?;
    writeln!(f, "- **Classification**: {}", report.classification)?;
    writeln!(f, "- **Photo**: `{}` (64x48 RGB PPM)", report.photo_file)?;
    writeln!(f, "- **Chain proof**: `0x{:016x}`", report.chain_proof)?;
    writeln!(f, "- **Action**: Continued mission (no threat)")?;

    println!("\n[OK] {} written", report.photo_file);
    println!("[OK] anomaly_report_001.md written");
    println!("Chain: {} blocks, verified: {}", chain.block_count(), chain.verify());
    Ok(())
}
