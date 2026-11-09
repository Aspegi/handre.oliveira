//! MAVLink v2 encoder with optional signing for the RNFC.
//! Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]

pub mod receiver;

use sha2::{Digest, Sha256};
use std::net::UdpSocket;
use std::time::Instant;

pub struct MavlinkSender {
    socket: UdpSocket,
    target: String,
    sysid: u8,
    compid: u8,
    seq: u8,
    start: Instant,
    pub stats_heartbeats: u64,
    pub stats_attitudes: u64,
    pub stats_errors: u64,
    secret_key: Option<[u8; 32]>,
    link_id: u8,
    signing_timestamp: u64,
}

impl MavlinkSender {
    pub fn new(target: &str, sysid: u8, compid: u8) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket, target: target.to_string(), sysid, compid,
            seq: 0, start: Instant::now(),
            stats_heartbeats: 0, stats_attitudes: 0, stats_errors: 0,
            secret_key: None, link_id: 0, signing_timestamp: 0,
        })
    }

    pub fn enable_signing(&mut self, secret_key: [u8; 32], link_id: u8) {
        self.secret_key = Some(secret_key);
        self.link_id = link_id;
    }

    pub fn disable_signing(&mut self) { self.secret_key = None; }

    pub fn set_signing_timestamp(&mut self, ts: u64) { self.signing_timestamp = ts; }
    pub fn signing_timestamp(&self) -> u64 { self.signing_timestamp }
    pub fn is_signing_enabled(&self) -> bool { self.secret_key.is_some() }

    fn send_frame(&mut self, msg_id: u32, payload: &[u8], crc_extra: u8) -> std::io::Result<usize> {
        let signing = self.secret_key.is_some();
        let mut frame = Vec::with_capacity(12 + payload.len() + 2 + if signing { 13 } else { 0 });
        frame.push(0xFD);
        frame.push(payload.len() as u8);
        frame.push(if signing { 0x01 } else { 0 });
        frame.push(0);
        frame.push(self.seq);
        frame.push(self.sysid);
        frame.push(self.compid);
        frame.push((msg_id & 0xFF) as u8);
        frame.push(((msg_id >> 8) & 0xFF) as u8);
        frame.push(((msg_id >> 16) & 0xFF) as u8);
        frame.extend_from_slice(payload);

        let mut crc: u16 = 0xFFFF;
        for &b in &frame[1..] { crc_accumulate(&mut crc, b); }
        crc_accumulate(&mut crc, crc_extra);
        frame.push((crc & 0xFF) as u8);
        frame.push(((crc >> 8) & 0xFF) as u8);

        if let Some(ref key) = self.secret_key {
            self.signing_timestamp = self.signing_timestamp.wrapping_add(1);
            let link_id = self.link_id;
            let ts_bytes = self.signing_timestamp.to_le_bytes();
            let mut hasher = Sha256::new();
            hasher.update(key);
            hasher.update(&frame);
            hasher.update([link_id]);
            hasher.update(&ts_bytes[0..6]);
            let result = hasher.finalize();
            frame.push(link_id);
            frame.extend_from_slice(&ts_bytes[0..6]);
            frame.extend_from_slice(&result[0..6]);
        }

        let n = self.socket.send_to(&frame, &self.target)?;
        self.seq = self.seq.wrapping_add(1);
        Ok(n)
    }

    pub fn send_heartbeat(&mut self) -> std::io::Result<()> {
        let mut payload = Vec::with_capacity(9);
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.push(2); payload.push(0); payload.push(0);
        payload.push(4); payload.push(3);
        match self.send_frame(0, &payload, 50) {
            Ok(_) => { self.stats_heartbeats += 1; Ok(()) }
            Err(e) => { self.stats_errors += 1; Err(e) }
        }
    }

    pub fn send_attitude(
        &mut self,
        roll: f32, pitch: f32, yaw: f32,
        rollspeed: f32, pitchspeed: f32, yawspeed: f32,
    ) -> std::io::Result<()> {
        let t_ms = self.start.elapsed().as_millis() as u32;
        let mut payload = Vec::with_capacity(28);
        payload.extend_from_slice(&t_ms.to_le_bytes());
        payload.extend_from_slice(&roll.to_le_bytes());
        payload.extend_from_slice(&pitch.to_le_bytes());
        payload.extend_from_slice(&yaw.to_le_bytes());
        payload.extend_from_slice(&rollspeed.to_le_bytes());
        payload.extend_from_slice(&pitchspeed.to_le_bytes());
        payload.extend_from_slice(&yawspeed.to_le_bytes());
        match self.send_frame(30, &payload, 39) {
            Ok(_) => { self.stats_attitudes += 1; Ok(()) }
            Err(e) => { self.stats_errors += 1; Err(e) }
        }
    }
}

fn crc_accumulate(crc: &mut u16, byte: u8) {
    let mut tmp = byte as u16 ^ (*crc & 0xFF);
    tmp ^= tmp << 4;
    *crc = (*crc >> 8) ^ (tmp << 8) ^ (tmp << 3) ^ (tmp >> 4);
}