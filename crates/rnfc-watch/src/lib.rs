//! Watch — motion detection with zoom, photo capture, and anomaly reports.
//!
//! When external movement is detected in the camera frame, the drone:
//! 1. Narrows FOV (120° -> 30°) on the detected sector
//! 2. Captures a photo (RGB PPM 64x48)
//! 3. Generates a timestamped anomaly report
//! 4. Records a cryptographic proof in a CertChain
//!
//! This is simulation: frames are synthetic, photos are generated RGB.

#[allow(unused_imports)]
use rnfc_certchain::{CertChain, SafetyState};

#[derive(Clone, Debug, PartialEq)]
pub struct WatchZone {
    pub angle_deg: f32,
    pub distance_m: f32,
    pub confidence: f32,
}

#[derive(Clone, Debug)]
pub struct MotionDetector {
    pub threshold: f32,
    pub last_frame: Option<Vec<u8>>,
}

impl MotionDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            last_frame: None,
        }
    }

    pub fn detect(&mut self, frame: &[u8]) -> Option<f32> {
        if let Some(prev) = &self.last_frame {
            if prev.len() == frame.len() {
                let mut diff = 0u64;
                for (a, b) in prev.iter().zip(frame.iter()) {
                    diff += (*a as i32 - *b as i32).unsigned_abs() as u64;
                }
                let avg = diff as f32 / frame.len() as f32;
                self.last_frame = Some(frame.to_vec());
                if avg > self.threshold {
                    return Some(avg);
                }
                return None;
            }
        }
        self.last_frame = Some(frame.to_vec());
        None
    }
}

#[derive(Clone, Debug)]
pub struct ZoomController {
    pub base_fov_deg: f32,
    pub zoomed_fov_deg: f32,
    pub active: bool,
    pub target: Option<WatchZone>,
    pub remaining_s: f32,
    pub timeout_s: f32,
}

impl ZoomController {
    pub fn new() -> Self {
        Self {
            base_fov_deg: 120.0,
            zoomed_fov_deg: 30.0,
            active: false,
            target: None,
            remaining_s: 0.0,
            timeout_s: 5.0,
        }
    }

    pub fn zoom_to(&mut self, zone: WatchZone) {
        self.active = true;
        self.target = Some(zone);
        self.remaining_s = self.timeout_s;
    }

    pub fn current_fov_deg(&self) -> f32 {
        if self.active {
            self.zoomed_fov_deg
        } else {
            self.base_fov_deg
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.active {
            self.remaining_s -= dt;
            if self.remaining_s <= 0.0 {
                self.active = false;
                self.target = None;
                self.remaining_s = 0.0;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PhotoCapture {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl PhotoCapture {
    pub fn generate(seed: u64, width: usize, height: usize) -> Self {
        let mut data = Vec::with_capacity(width * height * 3);
        let mut s = seed | 1;
        for _ in 0..(width * height * 3) {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            data.push((s & 0xFF) as u8);
        }
        Self {
            width,
            height,
            data,
        }
    }

    pub fn to_ppm(&self) -> Vec<u8> {
        let header = format!("P6\n{} {}\n255\n", self.width, self.height);
        let mut out = header.into_bytes();
        out.extend_from_slice(&self.data);
        out
    }
}

#[derive(Clone, Debug)]
pub struct AnomalyReport {
    pub report_id: u32,
    pub timestamp_s: f32,
    pub drone_pos: [f32; 2],
    pub drone_alt_m: f32,
    pub zone: WatchZone,
    pub photo_file: String,
    pub chain_proof: u64,
    pub classification: String,
}

pub struct WatchManager {
    pub detector: MotionDetector,
    pub zoom: ZoomController,
    pub reports: Vec<AnomalyReport>,
    pub next_id: u32,
}

impl WatchManager {
    pub fn new() -> Self {
        Self {
            detector: MotionDetector::new(10.0),
            zoom: ZoomController::new(),
            reports: Vec::new(),
            next_id: 1,
        }
    }

    pub fn on_frame(
        &mut self,
        frame: &[u8],
        t_s: f32,
        pos: [f32; 2],
        alt_m: f32,
        motion_angle_deg: f32,
        motion_dist_m: f32,
        chain: &mut CertChain,
    ) -> Option<AnomalyReport> {
        let confidence = self.detector.detect(frame)?;
        let zone = WatchZone {
            angle_deg: motion_angle_deg,
            distance_m: motion_dist_m,
            confidence: (confidence / 50.0).min(1.0),
        };
        self.zoom.zoom_to(zone.clone());

        let photo = PhotoCapture::generate(t_s as u64 * 1000 + self.next_id as u64, 64, 48);
        let photo_file = format!("watch_{:03}.ppm", self.next_id);

        let classification = if zone.distance_m < 15.0 {
            "high_interest".to_string()
        } else if zone.distance_m < 40.0 {
            "medium_interest".to_string()
        } else {
            "low_interest".to_string()
        };

        let mut buf = Vec::new();
        buf.extend_from_slice(&chain.root_hash().to_le_bytes());
        buf.extend_from_slice(&t_s.to_le_bytes());
        buf.extend_from_slice(&pos[0].to_le_bytes());
        buf.extend_from_slice(&pos[1].to_le_bytes());
        buf.extend_from_slice(photo_file.as_bytes());
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &buf {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }

        let report = AnomalyReport {
            report_id: self.next_id,
            timestamp_s: t_s,
            drone_pos: pos,
            drone_alt_m: alt_m,
            zone,
            photo_file,
            chain_proof: h,
            classification,
        };

        let _ = chain.append(t_s, 1, h, SafetyState::Nominal);
        self.reports.push(report.clone());
        self.next_id += 1;
        let _ = photo;
        Some(report)
    }

    pub fn tick(&mut self, dt: f32) {
        self.zoom.tick(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> CertChain {
        let mut c = CertChain::new(1, "watch_test");
        let _ = c.append(0.0, 1, 0xAA, SafetyState::Nominal);
        c
    }

    fn static_frame() -> Vec<u8> {
        vec![128u8; 64 * 48 * 3]
    }

    fn motion_frame() -> Vec<u8> {
        let mut f = static_frame();
        for i in 0..1000 {
            f[i] = 255;
        }
        f
    }

    #[test]
    fn no_motion_no_trigger() {
        let mut d = MotionDetector::new(10.0);
        let f = static_frame();
        assert!(d.detect(&f).is_none());
        assert!(d.detect(&f).is_none());
    }

    #[test]
    fn motion_triggers() {
        let mut d = MotionDetector::new(10.0);
        let _ = d.detect(&static_frame());
        assert!(d.detect(&motion_frame()).is_some());
    }

    #[test]
    fn zoom_narrows_fov() {
        let mut z = ZoomController::new();
        assert_eq!(z.current_fov_deg(), 120.0);
        z.zoom_to(WatchZone {
            angle_deg: 135.0,
            distance_m: 20.0,
            confidence: 0.9,
        });
        assert_eq!(z.current_fov_deg(), 30.0);
        assert!(z.active);
    }

    #[test]
    fn zoom_restores_after_timeout() {
        let mut z = ZoomController::new();
        z.zoom_to(WatchZone {
            angle_deg: 90.0,
            distance_m: 15.0,
            confidence: 0.8,
        });
        for _ in 0..60 {
            z.tick(0.1);
        }
        assert_eq!(z.current_fov_deg(), 120.0);
        assert!(!z.active);
    }

    #[test]
    fn photo_has_ppm_header() {
        let p = PhotoCapture::generate(42, 64, 48);
        let ppm = p.to_ppm();
        let s = String::from_utf8_lossy(&ppm[..20]);
        assert!(s.starts_with("P6"));
        assert_eq!(p.data.len(), 64 * 48 * 3);
    }

    #[test]
    fn photo_deterministic() {
        let a = PhotoCapture::generate(42, 8, 8);
        let b = PhotoCapture::generate(42, 8, 8);
        assert_eq!(a.data, b.data);
    }

    #[test]
    fn no_false_positive_on_small_noise() {
        let mut d = MotionDetector::new(10.0);
        let _ = d.detect(&static_frame());
        let mut small = static_frame();
        for i in 0..10 {
            small[i] = 129;
        }
        assert!(d.detect(&small).is_none());
    }

    #[test]
    fn manager_creates_report_on_motion() {
        let mut m = WatchManager::new();
        let mut c = chain();
        let _ = m.on_frame(&static_frame(), 0.0, [10.0, 5.0], 15.0, 0.0, 0.0, &mut c);
        let r = m.on_frame(&motion_frame(), 1.0, [10.0, 5.0], 15.0, 135.0, 20.0, &mut c);
        assert!(r.is_some());
        assert_eq!(m.reports.len(), 1);
    }

    #[test]
    fn report_contains_all_fields() {
        let mut m = WatchManager::new();
        let mut c = chain();
        let _ = m.on_frame(&static_frame(), 0.0, [0.0, 0.0], 10.0, 0.0, 0.0, &mut c);
        let r = m
            .on_frame(&motion_frame(), 5.0, [12.0, 7.0], 20.0, 90.0, 25.0, &mut c)
            .unwrap();
        assert_eq!(r.timestamp_s, 5.0);
        assert_eq!(r.drone_pos, [12.0, 7.0]);
        assert_eq!(r.drone_alt_m, 20.0);
        assert_eq!(r.zone.angle_deg, 90.0);
        assert!(!r.photo_file.is_empty());
        assert_ne!(r.chain_proof, 0);
    }

    #[test]
    fn report_classification_by_distance() {
        let mut m = WatchManager::new();
        let mut c = chain();
        let _ = m.on_frame(&static_frame(), 0.0, [0.0, 0.0], 10.0, 0.0, 0.0, &mut c);
        let r = m
            .on_frame(&motion_frame(), 1.0, [0.0, 0.0], 10.0, 0.0, 10.0, &mut c)
            .unwrap();
        assert_eq!(r.classification, "high_interest");
    }

    #[test]
    fn report_recorded_in_chain() {
        let mut m = WatchManager::new();
        let mut c = chain();
        let blocks_before = c.block_count();
        let _ = m.on_frame(&static_frame(), 0.0, [0.0, 0.0], 10.0, 0.0, 0.0, &mut c);
        let _ = m.on_frame(&motion_frame(), 1.0, [0.0, 0.0], 10.0, 0.0, 20.0, &mut c);
        assert_eq!(c.block_count(), blocks_before + 1);
        assert!(c.verify());
    }

    #[test]
    fn watch_manager_end_to_end() {
        let mut m = WatchManager::new();
        let mut c = chain();
        let _ = m.on_frame(&static_frame(), 0.0, [0.0, 0.0], 10.0, 0.0, 0.0, &mut c);
        let r = m.on_frame(&motion_frame(), 2.0, [5.0, 5.0], 12.0, 45.0, 18.0, &mut c);
        assert!(r.is_some());
        assert!(m.zoom.active);
        assert_eq!(m.zoom.current_fov_deg(), 30.0);
        assert!(c.verify());
    }
}
