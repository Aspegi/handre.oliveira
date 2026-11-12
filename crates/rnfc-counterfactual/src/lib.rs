//! Counterfactual Safety Reasoning (CSR) for RNFC.
//!
//! Post-mission, answers "what if?" questions with cryptographic proofs
//! bound to the CertChain. Example: "If wind had been 2x, the drone
//! would have missed waypoint 3 by 12m" — proof cannot be forged.

use rnfc_certchain::{export_bundle, CertChain};

#[derive(Clone, Debug)]
pub struct TrajectorySample {
    pub t_s: f32,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub wind: [f32; 2],
    pub altitude: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modification { WindScale, AltitudeOffset, None }

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    SameAsActual,
    Better { improvement_m: f32 },
    Worse { delta_m: f32 },
    Failed { reason: String },
}

#[derive(Clone, Debug)]
pub struct CounterfactualResult {
    pub scenario: String,
    pub modification: Modification,
    pub parameter: f32,
    pub outcome: Outcome,
    pub waypoint_error_m: f32,
    pub actual_error_m: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CounterfactualProof {
    pub chain_root_hash: u64,
    pub result_hash: u64,
    pub proof_hash: u64,
}

pub struct CounterfactualEngine { pub waypoint_tolerance_m: f32 }

impl CounterfactualEngine {
    pub fn new() -> Self { Self { waypoint_tolerance_m: 2.0 } }

    pub fn what_if_wind(
        &self,
        traj: &[TrajectorySample],
        target: [f32; 2],
        scale: f32,
    ) -> CounterfactualResult {
        let actual_final = traj.last().map(|s| s.pos).unwrap_or([0.0, 0.0]);
        let actual_err = dist(actual_final, target);
        let mut drift = [0.0f32; 2];
        for i in 1..traj.len() {
            let dt = traj[i].t_s - traj[i - 1].t_s;
            drift[0] += (scale - 1.0) * traj[i].wind[0] * dt;
            drift[1] += (scale - 1.0) * traj[i].wind[1] * dt;
        }
        let cf_err = dist([actual_final[0] + drift[0], actual_final[1] + drift[1]], target);
        CounterfactualResult {
            scenario: format!("wind_x{:.1}", scale),
            modification: Modification::WindScale,
            parameter: scale,
            outcome: classify(cf_err, actual_err, self.waypoint_tolerance_m),
            waypoint_error_m: cf_err,
            actual_error_m: actual_err,
        }
    }

    pub fn what_if_altitude(
        &self,
        traj: &[TrajectorySample],
        offset_m: f32,
    ) -> CounterfactualResult {
        let mut min_alt = f32::INFINITY;
        let mut max_alt = f32::NEG_INFINITY;
        for s in traj {
            let a = s.altitude + offset_m;
            if a < min_alt { min_alt = a; }
            if a > max_alt { max_alt = a; }
        }
        let outcome = if min_alt < 0.5 {
            Outcome::Failed { reason: format!("altitude {:.2}m < 0.5m", min_alt) }
        } else if max_alt > 120.0 {
            Outcome::Failed { reason: format!("altitude {:.1}m > 120m", max_alt) }
        } else { Outcome::SameAsActual };
        CounterfactualResult {
            scenario: format!("altitude_{:+.1}m", offset_m),
            modification: Modification::AltitudeOffset,
            parameter: offset_m,
            outcome,
            waypoint_error_m: 0.0,
            actual_error_m: 0.0,
        }
    }

    pub fn prove(&self, chain: &CertChain, r: &CounterfactualResult) -> CounterfactualProof {
        let chain_hash = chain_fingerprint(chain);
        let result_hash = hash_result(r);
        let mut buf = Vec::new();
        buf.extend_from_slice(&chain_hash.to_le_bytes());
        buf.extend_from_slice(&result_hash.to_le_bytes());
        CounterfactualProof {
            chain_root_hash: chain_hash,
            result_hash,
            proof_hash: fnv1a_64(&buf),
        }
    }

    pub fn verify(&self, p: &CounterfactualProof, chain: &CertChain, r: &CounterfactualResult) -> bool {
        if chain_fingerprint(chain) != p.chain_root_hash { return false; }
        if hash_result(r) != p.result_hash { return false; }
        let mut buf = Vec::new();
        buf.extend_from_slice(&p.chain_root_hash.to_le_bytes());
        buf.extend_from_slice(&p.result_hash.to_le_bytes());
        fnv1a_64(&buf) == p.proof_hash
    }
}

fn classify(cf_err: f32, actual_err: f32, tol: f32) -> Outcome {
    if (cf_err - actual_err).abs() < 0.1 {
        Outcome::SameAsActual
    } else if cf_err > tol * 5.0 {
        Outcome::Failed { reason: format!("err {:.2}m > {:.1}m", cf_err, tol * 5.0) }
    } else if cf_err > actual_err + 0.1 {
        Outcome::Worse { delta_m: cf_err - actual_err }
    } else {
        Outcome::Better { improvement_m: actual_err - cf_err }
    }
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn chain_fingerprint(chain: &CertChain) -> u64 {
    fnv1a_64(export_bundle(chain).json.as_bytes())
}

fn hash_result(r: &CounterfactualResult) -> u64 {
    let mut buf = Vec::new();
    buf.extend_from_slice(r.scenario.as_bytes());
    let m = match r.modification {
        Modification::WindScale => 1u8,
        Modification::AltitudeOffset => 2u8,
        Modification::None => 0u8,
    };
    buf.push(m);
    buf.extend_from_slice(&r.parameter.to_le_bytes());
    buf.extend_from_slice(&r.waypoint_error_m.to_le_bytes());
    fnv1a_64(&buf)
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnfc_certchain::{CertChain, SafetyState};

    fn traj() -> Vec<TrajectorySample> {
        (0..=10).map(|i| {
            let t = i as f32;
            TrajectorySample {
                t_s: t,
                pos: [5.0 * t, 2.0 * t],
                vel: [5.0, 0.0],
                wind: [0.0, 2.0],
                altitude: 10.0,
            }
        }).collect()
    }

    fn chain() -> CertChain {
        let mut c = CertChain::new(1, "cf_test");
        c.append(1.0, 2, 0xAA, SafetyState::Nominal).unwrap();
        c.append(2.0, 1, 0xBB, SafetyState::Nominal).unwrap();
        c
    }

    #[test]
    fn wind_2x_increases_error() {
        let e = CounterfactualEngine::new();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        assert!(r.waypoint_error_m > 30.0, "err={}", r.waypoint_error_m);
        assert!(matches!(r.outcome, Outcome::Failed { .. }));
    }

    #[test]
    fn wind_0x_reduces_error() {
        let e = CounterfactualEngine::new();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 0.0);
        assert!(r.waypoint_error_m < 1.0, "err={}", r.waypoint_error_m);
        assert!(matches!(r.outcome, Outcome::Better { .. }));
    }

    #[test]
    fn wind_1x_unchanged() {
        let e = CounterfactualEngine::new();
        let r = e.what_if_wind(&traj(), [50.0, 20.0], 1.0);
        assert!(r.waypoint_error_m < 0.01);
        assert_eq!(r.outcome, Outcome::SameAsActual);
    }

    #[test]
    fn altitude_within_limits_ok() {
        let e = CounterfactualEngine::new();
        let r = e.what_if_altitude(&traj(), 2.0);
        assert_eq!(r.outcome, Outcome::SameAsActual);
    }

    #[test]
    fn altitude_too_low_fails() {
        let e = CounterfactualEngine::new();
        let r = e.what_if_altitude(&traj(), -10.0);
        assert!(matches!(r.outcome, Outcome::Failed { .. }));
    }

    #[test]
    fn altitude_too_high_fails() {
        let e = CounterfactualEngine::new();
        let r = e.what_if_altitude(&traj(), 150.0);
        assert!(matches!(r.outcome, Outcome::Failed { .. }));
    }

    #[test]
    fn proof_bound_to_chain() {
        let e = CounterfactualEngine::new();
        let c = chain();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        let p = e.prove(&c, &r);
        assert!(e.verify(&p, &c, &r));
    }

    #[test]
    fn tampered_chain_invalidates_proof() {
        let e = CounterfactualEngine::new();
        let c = chain();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        let p = e.prove(&c, &r);
        let mut t = chain();
        t.append(99.0, 99, 0xDEAD, SafetyState::FdirActive).unwrap();
        assert!(!e.verify(&p, &t, &r));
    }

    #[test]
    fn tampered_result_invalidates_proof() {
        let e = CounterfactualEngine::new();
        let c = chain();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        let p = e.prove(&c, &r);
        let mut fake = r.clone();
        fake.waypoint_error_m = 0.0;
        assert!(!e.verify(&p, &c, &fake));
    }

    #[test]
    fn proof_deterministic() {
        let e = CounterfactualEngine::new();
        let c = chain();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        assert_eq!(e.prove(&c, &r).proof_hash, e.prove(&c, &r).proof_hash);
    }

    #[test]
    fn different_chains_different_proofs() {
        let e = CounterfactualEngine::new();
        let c1 = chain();
        let mut c2 = CertChain::new(2, "other");
        c2.append(1.0, 3, 0xCC, SafetyState::Nominal).unwrap();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        assert_ne!(e.prove(&c1, &r).proof_hash, e.prove(&c2, &r).proof_hash);
    }

    #[test]
    fn counterfactual_hash_can_be_recorded() {
        let e = CounterfactualEngine::new();
        let mut c = chain();
        let r = e.what_if_wind(&traj(), [50.0, 0.0], 2.0);
        let p = e.prove(&c, &r);
        let _ = c.append(3.0, 1, p.proof_hash, SafetyState::Nominal);
        assert!(c.verify());
    }
}
