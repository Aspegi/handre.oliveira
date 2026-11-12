use rnfc_certis::*;

fn main() {
    println!("=== CERTIS Demo — Simulated Mission ===");
    let mut rec = CertisRecorder::new("demo_2026_001");

    // Simular eventos de uma missão realista
    rec.record(12.3, SafetyEventType::SensorOutlier);
    rec.record(45.7, SafetyEventType::SensorSpoof);
    rec.record(
        46.0,
        SafetyEventType::FailsafeTriggered {
            action: FailsafeActionKind::ReturnToLaunch,
        },
    );
    rec.record(78.2, SafetyEventType::MotorLossDetected { motor: 2 });
    rec.record(78.3, SafetyEventType::LnnRecoveryStarted);
    rec.record(90.5, SafetyEventType::LnnRecoverySucceeded);
    rec.record(120.1, SafetyEventType::ZombieDetected);
    rec.record(
        120.2,
        SafetyEventType::FailsafeTriggered {
            action: FailsafeActionKind::ReturnToLaunch,
        },
    );
    rec.record(
        145.3,
        SafetyEventType::PilotOverride {
            command: "manual_return".into(),
        },
    );

    println!("Events recorded: {}", rec.events.len());
    println!("Root hash: {:#018x}", rec.root_hash());
    println!("Chain valid: {}\n", rec.verify_chain());

    let sc = SafetyCase::generate(&rec);
    let markdown = sc.to_markdown();

    std::fs::write("safety_case_demo.md", &markdown).unwrap();
    println!(
        "Safety case written to safety_case_demo.md ({} bytes)",
        markdown.len()
    );
    println!("\n--- First 25 lines ---");
    for line in markdown.lines().take(25) {
        println!("{}", line);
    }
}
