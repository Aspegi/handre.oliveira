//! Hardware driver for real quantum magnetometers (NV-centre, SERF).
//!
//! This crate provides:
//! 1. A trait `QuantumMagSensor` for any 3-axis magnetometer
//! 2. A driver for the Kwan-tek Pyxsee-Nano (NV-centre, I2C)
//! 3. A loader for real magnetic anomaly maps (MagNav.jl format)
//! 4. An estimator that uses real maps instead of synthetic ones

use embedded_hal::i2c::I2c;
use rnfc_magnav::{MagneticAnomalyMap, MagNavEstimator, QuantumMagSensor as SimSensor};

/// Trait for a real 3-axis quantum magnetometer.
pub trait QuantumMagSensor {
    /// Read the magnetic field in microtesla (x, y, z).
    fn read_microtesla(&mut self) -> Result<[f32; 3], SensorError>;
    /// Sensor model name.
    fn model(&self) -> &'static str;
}

#[derive(Debug)]
pub enum SensorError {
    I2cError,
    NotReady,
    OutOfRange,
}

/// Driver for the Kwan-tek Pyxsee-Nano (NV-centre, I2C address 0x1C).
pub struct PyxseeNano<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C: I2c> PyxseeNano<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self { i2c, address: 0x1C }
    }

    /// Read 3-axis field (6 bytes: x_hi, x_lo, y_hi, y_lo, z_hi, z_lo).
    pub fn read_raw(&mut self) -> Result<[i16; 3], SensorError> {
        let mut buf = [0u8; 6];
        self.i2c
            .read(self.address, &mut buf)
            .map_err(|_| SensorError::I2cError)?;
        let x = i16::from_be_bytes([buf[0], buf[1]]);
        let y = i16::from_be_bytes([buf[2], buf[3]]);
        let z = i16::from_be_bytes([buf[4], buf[5]]);
        Ok([x, y, z])
    }
}

impl<I2C: I2c> QuantumMagSensor for PyxseeNano<I2C> {
    fn read_microtesla(&mut self) -> Result<[f32; 3], SensorError> {
        let raw = self.read_raw()?;
        // Pyxsee-Nano: 1 LSB = 0.01 µT (typical for NV sensors)
        let scale = 0.01_f32;
        Ok([
            raw[0] as f32 * scale,
            raw[1] as f32 * scale,
            raw[2] as f32 * scale,
        ])
    }
    fn model(&self) -> &'static str { "Pyxsee-Nano (NV-centre)" }
}

/// Loader for real magnetic anomaly maps (MagNav.jl / DAF-MIT format).
pub struct RealAnomalyMap {
    pub map: MagneticAnomalyMap,
}

impl RealAnomalyMap {
    /// Load a map from a CSV (lat, lon, anomaly_nT).
    /// The map is interpolated onto a regular grid.
    pub fn from_csv(csv: &str, width: usize, height: usize) -> Result<Self, String> {
        // Parse CSV: header "lat,lon,anomaly_nT"
        let mut points = Vec::new();
        for line in csv.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 3 { continue; }
            let lat: f32 = parts[0].parse().map_err(|_| "bad lat")?;
            let lon: f32 = parts[1].parse().map_err(|_| "bad lon")?;
            let anomaly_nt: f32 = parts[2].parse().map_err(|_| "bad anomaly")?;
            points.push((lat, lon, anomaly_nt));
        }
        if points.is_empty() { return Err("no points".into()); }

        // Find bounds
        let (mut min_lat, mut max_lat) = (f32::INFINITY, f32::NEG_INFINITY);
        let (mut min_lon, mut max_lon) = (f32::INFINITY, f32::NEG_INFINITY);
        for (lat, lon, _) in &points {
            if *lat < min_lat { min_lat = *lat; }
            if *lat > max_lat { max_lat = *lat; }
            if *lon < min_lon { min_lon = *lon; }
            if *lon > max_lon { max_lon = *lon; }
        }
        let res_lat = (max_lat - min_lat) / height as f32;
        let res_lon = (max_lon - min_lon) / width as f32;

        // Simple nearest-neighbor interpolation onto grid
        let mut cells = vec![50.0_f32; width * height]; // baseline 50 µT
        for (lat, lon, anomaly_nt) in &points {
            let i = ((lat - min_lat) / res_lat) as usize;
            let j = ((lon - min_lon) / res_lon) as usize;
            if i < width && j < height {
                // anomaly in nT → µT: 1 nT = 0.001 µT
                cells[i * height + j] = 50.0 + anomaly_nt * 0.001;
            }
        }

        let map = MagneticAnomalyMap {
            origin: [min_lat, min_lon],
            resolution: res_lat.max(res_lon),
            width,
            height,
            cells,
        };
        Ok(Self { map })
    }
}

/// Estimator that fuses real sensor readings with a real anomaly map.
pub struct RealMagNavEstimator {
    pub inner: MagNavEstimator,
    pub map: RealAnomalyMap,
}

impl RealMagNavEstimator {
    pub fn new(map: RealAnomalyMap) -> Self {
        Self {
            inner: MagNavEstimator::new(),
            map,
        }
    }

    /// Estimate position from a real sensor reading and a prior.
    /// The reading is the total field magnitude (µT).
    pub fn estimate_from_sensor(
        &self,
        sensor: &mut dyn QuantumMagSensor,
        prior: [f32; 2],
    ) -> Result<[f32; 2], SensorError> {
        let b = sensor.read_microtesla()?;
        let total = libm::sqrtf(b[0]*b[0] + b[1]*b[1] + b[2]*b[2]);
        Ok(self.inner.estimate(&SimSensor { map: self.map.map.clone(), noise_ut: 0.1 }, total, prior))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_loader_works() {
        let csv = "lat,lon,anomaly_nT\n40.0,-74.0,100\n40.1,-74.0,-50\n40.0,-73.9,200\n40.1,-73.9,0\n";
        let map = RealAnomalyMap::from_csv(csv, 4, 4).unwrap();
        assert_eq!(map.map.width, 4);
        assert!(map.map.cells.iter().any(|&c| c > 50.0));
    }

    #[test]
    fn real_estimator_returns_prior_on_zero() {
        let csv = "lat,lon,anomaly_nT\n40.0,-74.0,0\n40.1,-74.0,0\n40.0,-73.9,0\n40.1,-73.9,0\n";
        let map = RealAnomalyMap::from_csv(csv, 2, 2).unwrap();
        let est = RealMagNavEstimator::new(map);
        let prior = [40.05, -73.95];
        // Mock sensor that returns a fixed total
        struct MockSensor;
        impl QuantumMagSensor for MockSensor {
            fn read_microtesla(&mut self) -> Result<[f32; 3], SensorError> { Ok([0.0, 0.0, 50.0]) }
            fn model(&self) -> &'static str { "mock" }
        }
        let mut mock = MockSensor;
        let est_pos = est.estimate_from_sensor(&mut mock, prior).unwrap();
        assert!((est_pos[0] - prior[0]).abs() < 0.1);
    }
}
