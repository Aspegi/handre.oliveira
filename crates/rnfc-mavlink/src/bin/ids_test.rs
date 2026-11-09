//! Teste de integracao do IDS: simula ataques e verifica alertas.
use rnfc_mavlink::receiver::{IdsAlert, MavlinkReceiver};
use rnfc_mavlink::MavlinkSender;
use std::thread;
use std::time::Duration;

fn main() {
    let key: [u8; 32] = { let mut k = [0u8; 32]; for i in 0..32 { k[i] = i as u8; } k };

    let mut rx = MavlinkReceiver::new("127.0.0.1:14600", key, 0).expect("receiver");
    println!("[TESTE] Recetor a escutar em 127.0.0.1:14600");

    let mut good = MavlinkSender::new("127.0.0.1:14600", 1, 1).expect("sender");
    good.enable_signing(key, 0);

    let mut bad = MavlinkSender::new("127.0.0.1:14600", 99, 1).expect("sender");
    bad.enable_signing([0xFF; 32], 0);

    println!("\n[TESTE 1] 5 pacotes legitimos");
    for _ in 0..5 {
        good.send_attitude(0.0, 0.0, 0.0, 0.0, 0.0, 0.0).ok();
        thread::sleep(Duration::from_millis(10));
    }
    drain(&mut rx, "apos legitimos");

    println!("\n[TESTE 2] chave errada (assinatura invalida)");
    for _ in 0..3 {
        bad.send_attitude(1.0, 1.0, 1.0, 0.0, 0.0, 0.0).ok();
        thread::sleep(Duration::from_millis(10));
    }
    drain(&mut rx, "apos chave errada");

    println!("\n[TESTE 3] mais legitimos (ts continua)");
    for _ in 0..2 {
        good.send_attitude(0.0, 0.0, 0.0, 0.0, 0.0, 0.0).ok();
        thread::sleep(Duration::from_millis(10));
    }
    drain(&mut rx, "apos mais legitimos");

    println!("\n[TESTE 4] sysid desconhecido");
    let mut unknown = MavlinkSender::new("127.0.0.1:14600", 42, 1).expect("sender");
    unknown.enable_signing(key, 0);
    unknown.send_heartbeat().ok();
    thread::sleep(Duration::from_millis(50));
    drain(&mut rx, "apos sysid desconhecido");

    println!("\n=== RESUMO ===");
    println!("Frames validos: {}", rx.valid_frames);
    println!("Frames invalidos: {}", rx.invalid_frames);
    println!("Alertas: {}", rx.alerts.len());
    for a in &rx.alerts {
        println!("  - {}", a);
    }
}

fn drain(rx: &mut MavlinkReceiver, phase: &str) {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_millis(200) {
        if !rx.try_receive().unwrap_or(false) {
            thread::sleep(Duration::from_millis(5));
        }
    }
    println!("  [{}] validos={} invalidos={} alertas={}",
        phase, rx.valid_frames, rx.invalid_frames, rx.alerts.len());
}