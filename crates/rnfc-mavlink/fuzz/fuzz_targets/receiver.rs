//! Fuzz target for the MAVLink receiver.
//! Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
//! Objetivo: provar que nenhum pacote malformado faz o receiver entrar em panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rnfc_mavlink::receiver::MavlinkReceiver;

fuzz_target!(|data: &[u8]| {
    let key: [u8; 32] = {
        let mut k = [0u8; 32];
        let mut i = 0;
        while i < 32 {
            k[i] = i as u8;
            i += 1;
        }
        k
    };

    let mut rx = match MavlinkReceiver::new("127.0.0.1:0", key, 0) {
        Ok(r) => r,
        Err(_) => return,
    };

    rx.feed_bytes(data);
});