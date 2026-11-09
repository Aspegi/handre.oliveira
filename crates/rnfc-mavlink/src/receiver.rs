//! MAVLink v2 receiver with signature verification and IDS.
//! Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdsAlert {
    Flooding { sysid: u8, rate: u32 },
    UnknownSysid { sysid: u8 },
    Replay { sysid: u8, ts: u64 },
    TimestampGap { sysid: u8, gap: u64 },
    InvalidSignature { sysid: u8 },
    Bruteforce { sysid: u8, failures: u32 },
}

impl std::fmt::Display for IdsAlert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdsAlert::Flooding { sysid, rate } => write!(f, "FLOODING sysid={} rate={}/s", sysid, rate),
            IdsAlert::UnknownSysid { sysid } => write!(f, "UNKNOWN_SYSID sysid={}", sysid),
            IdsAlert::Replay { sysid, ts } => write!(f, "REPLAY sysid={} ts={}", sysid, ts),
            IdsAlert::TimestampGap { sysid, gap } => write!(f, "TIMESTAMP_GAP sysid={} gap={}", sysid, gap),
            IdsAlert::InvalidSignature { sysid } => write!(f, "INVALID_SIGNATURE sysid={}", sysid),
            IdsAlert::Bruteforce { sysid, failures } => write!(f, "BRUTEFORCE sysid={} failures={}", sysid, failures),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IdsConfig {
    pub authorized_sysids: Vec<u8>,
    pub max_rate_per_sec: u32,
    pub max_ts_gap: u64,
    pub max_invalid_sigs: u32,
    pub invalid_sig_window_s: u64,
}

impl Default for IdsConfig {
    fn default() -> Self {
        Self {
            authorized_sysids: vec![1, 255],
            max_rate_per_sec: 50,
            max_ts_gap: 100,
            max_invalid_sigs: 10,
            invalid_sig_window_s: 10,
        }
    }
}

pub struct MavlinkReceiver {
    socket: UdpSocket,
    secret_key: [u8; 32],
    #[allow(dead_code)]
    link_id: u8,
    last_timestamp: HashMap<(u8, u8, u8), u64>,
    rate_counter: HashMap<u8, (u32, Instant)>,
    invalid_sig_counter: HashMap<u8, (u32, Instant)>,
    pub config: IdsConfig,
    pub alerts: Vec<IdsAlert>,
    pub valid_frames: u64,
    pub invalid_frames: u64,
}

impl MavlinkReceiver {
    pub fn new(bind_addr: &str, secret_key: [u8; 32], link_id: u8) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        Ok(Self {
            socket,
            secret_key,
            link_id,
            last_timestamp: HashMap::new(),
            rate_counter: HashMap::new(),
            invalid_sig_counter: HashMap::new(),
            config: IdsConfig::default(),
            alerts: Vec::new(),
            valid_frames: 0,
            invalid_frames: 0,
        })
    }

    pub fn try_receive(&mut self) -> std::io::Result<bool> {
        let mut buf = [0u8; 512];
        match self.socket.recv_from(&mut buf) {
            Ok((n, _addr)) => {
                self.process_frame(&buf[..n]);
                Ok(true)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn process_frame(&mut self, data: &[u8]) {
        if data.len() < 12 || data[0] != 0xFD { return; }
        let payload_len = data[1] as usize;
        let incompat = data[2];
        let sysid = data[5];
        let compid = data[6];
        let signed = (incompat & 0x01) != 0;
        let base_len = 10 + payload_len + 2;
        if data.len() < base_len { self.invalid_frames += 1; return; }

        self.check_rate(sysid);
        if !self.config.authorized_sysids.contains(&sysid) {
            self.alerts.push(IdsAlert::UnknownSysid { sysid });
        }

        if !signed {
            self.alerts.push(IdsAlert::InvalidSignature { sysid });
            self.count_invalid_sig(sysid);
            self.invalid_frames += 1;
            return;
        }

        if data.len() < base_len + 13 { self.invalid_frames += 1; return; }
        let sig_start = base_len;
        let link_id = data[sig_start];
        let ts_bytes = &data[sig_start + 1..sig_start + 7];
        let sig_bytes = &data[sig_start + 7..sig_start + 13];

        let ts = u64::from_le_bytes([
            ts_bytes[0], ts_bytes[1], ts_bytes[2], ts_bytes[3],
            ts_bytes[4], ts_bytes[5], 0, 0,
        ]);

        let frame_without_sig = &data[..sig_start];
        let mut hasher = Sha256::new();
        hasher.update(&self.secret_key);
        hasher.update(frame_without_sig);
        hasher.update([link_id]);
        hasher.update(&ts_bytes[..6]);
        let expected = hasher.finalize();
        if &expected[..6] != sig_bytes {
            self.alerts.push(IdsAlert::InvalidSignature { sysid });
            self.count_invalid_sig(sysid);
            self.invalid_frames += 1;
            return;
        }

        let key = (sysid, compid, link_id);
        let last = self.last_timestamp.get(&key).copied().unwrap_or(0);
        if ts <= last {
            self.alerts.push(IdsAlert::Replay { sysid, ts });
            self.invalid_frames += 1;
            return;
        }

        if last > 0 && ts - last > self.config.max_ts_gap {
            self.alerts.push(IdsAlert::TimestampGap { sysid, gap: ts - last });
        }

        self.last_timestamp.insert(key, ts);
        self.valid_frames += 1;
    }

    fn check_rate(&mut self, sysid: u8) {
        let now = Instant::now();
        let entry = self.rate_counter.entry(sysid).or_insert((0, now));
        if now.duration_since(entry.1).as_secs() >= 1 { entry.0 = 0; entry.1 = now; }
        entry.0 += 1;
        if entry.0 > self.config.max_rate_per_sec {
            self.alerts.push(IdsAlert::Flooding { sysid, rate: entry.0 });
        }
    }

    fn count_invalid_sig(&mut self, sysid: u8) {
        let now = Instant::now();
        let entry = self.invalid_sig_counter.entry(sysid).or_insert((0, now));
        if now.duration_since(entry.1).as_secs() >= self.config.invalid_sig_window_s {
            entry.0 = 0; entry.1 = now;
        }
        entry.0 += 1;
        if entry.0 >= self.config.max_invalid_sigs {
            self.alerts.push(IdsAlert::Bruteforce { sysid, failures: entry.0 });
        }
    }
}