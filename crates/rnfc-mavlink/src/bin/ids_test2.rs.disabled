//! Teste das regras IDS que faltam: flooding, gap de timestamp, bruteforce.
use rnfc_mavlink::receiver::{IdsAlert, MavlinkReceiver};
use rnfc_mavlink::MavlinkSender;
use std::thread;
use std::time::Duration;

fn main() {
    let key: [u8; 32] = { let mut k = [0u8; 32]; for i in 0..32 { k[i] = i as u8; } k };

    let mut rx = MavlinkReceiver::new("127.0.0.1:14601", key, 0).expect("receiver");
    println!("[TESTE2] Recetor em 127.0.0.1:14601");

    // ============================================================
    // TESTE 1: FLOODING (IDS-RULE-001)
    // Enviar 60 pacotes em <1s -> rate > 50/s -> FLOODING
    // ============================================================
    println!("\n[TESTE 1] FLOODING (60 pacotes rapidos)");
    let mut flood = MavlinkSender::new("127.0.0.1:14601", 1, 1).expect("sender");
    flood.enable_signing(key, 0);

    // Enviar 60 pacotes o mais rapido possivel (sem sleep)
    for _ in 0..60 {
        flood.send_heartbeat().ok();
    }
    // Drenar com timeout curto
    drain(&mut rx, "apos flooding", 300);

    let flooding = rx.alerts.iter().filter(|a| matches!(a, IdsAlert::Flooding { .. })).count();
    println!("  Alertas FLOODING: {}", flooding);

    // ============================================================
    // TESTE 2: GAP DE TIMESTAMP (IDS-RULE-004)
    // Enviar 1 pacote, saltar timestamp +200, enviar outro -> gap > 100
    // ============================================================
    println!("\n[TESTE 2] GAP DE TIMESTAMP (saltar +200)");
    let mut gap_sender = MavlinkSender::new("127.0.0.1:14601", 2, 1).expect("sender");
    gap_sender.enable_signing(key, 0);
    gap_sender.set_signing_timestamp(1000);

    gap_sender.send_heartbeat().ok();  // ts = 1001
    thread::sleep(Duration::from_millis(20));
    drain(&mut rx, "apos 1o pacote", 100);

    // Saltar o timestamp para 1201 (+200)
    gap_sender.set_signing_timestamp(1200);
    gap_sender.send_heartbeat().ok();  // ts = 1201
    thread::sleep(Duration::from_millis(20));
    drain(&mut rx, "apos gap", 200);

    let gaps = rx.alerts.iter().filter(|a| matches!(a, IdsAlert::TimestampGap { .. })).count();
    println!("  Alertas TIMESTAMP_GAP: {}", gaps);

    // ============================================================
    // TESTE 3: BRUTEFORCE (IDS-RULE-006)
    // Enviar 12 assinaturas invalidas -> >= 10 -> BRUTEFORCE
    // ============================================================
    println!("\n[TESTE 3] BRUTEFORCE (12 chaves erradas)");
    let mut attacker = MavlinkSender::new("127.0.0.1:14601", 3, 1).expect("sender");
    attacker.enable_signing([0xAA; 32], 0);  // chave errada

    for _ in 0..12 {
        attacker.send_heartbeat().ok();
    }
    drain(&mut rx, "apos bruteforce", 300);

    let bruteforce = rx.alerts.iter().filter(|a| matches!(a, IdsAlert::Bruteforce { .. })).count();
    println!("  Alertas BRUTEFORCE: {}", bruteforce);

    // ============================================================
    // RESUMO
    // ============================================================
    println!("\n=== RESUMO ===");
    println!("Frames validos: {}", rx.valid_frames);
    println!("Frames invalidos: {}", rx.invalid_frames);
    println!("Total alertas: {}", rx.alerts.len());

    let mut flooding_count = 0;
    let mut gap_count = 0;
    let mut bruteforce_count = 0;
    let mut invalid_sig_count = 0;
    for a in &rx.alerts {
        match a {
            IdsAlert::Flooding { .. } => flooding_count += 1,
            IdsAlert::TimestampGap { .. } => gap_count += 1,
            IdsAlert::Bruteforce { .. } => bruteforce_count += 1,
            IdsAlert::InvalidSignature { .. } => invalid_sig_count += 1,
            _ => {}
        }
    }
    println!("\nDistribuicao:");
    println!("  FLOODING: {}", flooding_count);
    println!("  TIMESTAMP_GAP: {}", gap_count);
    println!("  BRUTEFORCE: {}", bruteforce_count);
    println!("  INVALID_SIGNATURE: {}", invalid_sig_count);

    println!("\n=== VEREDICTO ===");
    if flooding_count > 0 { println!("  IDS-RULE-001 (flooding): OK"); } else { println!("  IDS-RULE-001 (flooding): FALHOU"); }
    if gap_count > 0 { println!("  IDS-RULE-004 (gap): OK"); } else { println!("  IDS-RULE-004 (gap): FALHOU"); }
    if bruteforce_count > 0 { println!("  IDS-RULE-006 (bruteforce): OK"); } else { println!("  IDS-RULE-006 (bruteforce): FALHOU"); }
}

fn drain(rx: &mut MavlinkReceiver, phase: &str, ms: u64) {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(ms) {
        if !rx.try_receive().unwrap_or(false) {
            thread::sleep(Duration::from_millis(5));
        }
    }
    println!("  [{}] validos={} invalidos={} alertas={}",
        phase, rx.valid_frames, rx.invalid_frames, rx.alerts.len());
}