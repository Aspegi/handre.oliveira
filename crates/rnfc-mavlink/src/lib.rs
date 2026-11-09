// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
//! MAVLink v2 encoder and UDP sender for the RNFC.
//!
//! Implements only the messages we need (HEARTBEAT, ATTITUDE) so there
//! are no external dependencies. The wire format follows the MAVLink v2
//! specification: magic 0xFD, length, flags, seq, sysid, compid, msgid,
//! payload, CRC-16/MCRF4XX (with crc_extra byte appended).
#![deny(unsafe_code)]

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
}

impl MavlinkSender {
    pub fn new(target: &str, sysid: u8, compid: u8) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            socket,
            target: target.to_string(),
            sysid,
            compid,
            seq: 0,
            start: Instant::now(),
            stats_heartbeats: 0,
            stats_attitudes: 0,
            stats_errors: 0,
        })
    }

    fn send_frame(&mut self, msg_id: u32, payload: &[u8], crc_extra: u8) -> std::io::Result<usize> {
        let mut frame = Vec::with_capacity(12 + payload.len() + 2);
        frame.push(0xFD);                              // magic v2
        frame.push(payload.len() as u8);               // payload length
        frame.push(0);                                 // incompat_flags
        frame.push(0);                                 // compat_flags
        frame.push(self.seq);                          // sequence
        frame.push(self.sysid);                        // system id
        frame.push(self.compid);                       // component id
        frame.push((msg_id & 0xFF) as u8);             // msg id (LE, 3 bytes)
        frame.push(((msg_id >> 8) & 0xFF) as u8);
        frame.push(((msg_id >> 16) & 0xFF) as u8);
        frame.extend_from_slice(payload);

        // CRC over bytes [1..] (excluding magic) + crc_extra
        let mut crc: u16 = 0xFFFF;
        for &b in &frame[1..] {
            crc_accumulate(&mut crc, b);
        }
        crc_accumulate(&mut crc, crc_extra);
        frame.push((crc & 0xFF) as u8);
        frame.push(((crc >> 8) & 0xFF) as u8);

        let n = self.socket.send_to(&frame, &self.target)?;
        self.seq = self.seq.wrapping_add(1);
        Ok(n)
    }

    /// HEARTBEAT (msg id 0, crc_extra 50).
    /// Fields on the wire: custom_mode u32, type u8, autopilot u8,
    /// base_mode u8, system_status u8, mavlink_version u8.
    pub fn send_heartbeat(&mut self) -> std::io::Result<()> {
        let mut payload = Vec::with_capacity(9);
        payload.extend_from_slice(&0u32.to_le_bytes()); // custom_mode
        payload.push(2);                                 // MAV_TYPE_QUADROTOR
        payload.push(0);                                 // MAV_AUTOPILOT_GENERIC
        payload.push(0);                                 // base_mode
        payload.push(4);                                 // MAV_STATE_ACTIVE
        payload.push(3);                                 // mavlink_version
        match self.send_frame(0, &payload, 50) {
            Ok(_) => { self.stats_heartbeats += 1; Ok(()) }
            Err(e) => { self.stats_errors += 1; Err(e) }
        }
    }

    /// ATTITUDE (msg id 30, crc_extra 39).
    /// Fields on the wire: time_boot_ms u32, roll f32, pitch f32, yaw f32,
    /// rollspeed f32, pitchspeed f32, yawspeed f32.
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

/// CRC-16/MCRF4XX used by MAVLink (x25 with the MAVLink tweak).
fn crc_accumulate(crc: &mut u16, byte: u8) {
    let mut tmp = byte as u16 ^ (*crc & 0xFF);
    tmp ^= tmp << 4;
    *crc = (*crc >> 8) ^ (tmp << 8) ^ (tmp << 3) ^ (tmp >> 4);
}