//! SCC — Safety Case Compiler.
//!
//! Takes safety requirements in a small DSL and compiles them into:
//! 1. Rust test stubs (specifications)
//! 2. GSN safety case nodes (markdown)
//! 3. Cryptographic chain blocks (via rnfc-certchain)
//!
//! One source of truth for safety. Change a requirement, regenerate
//! all artifacts, re-validate the chain.

use rnfc_certchain::{CertChain, SafetyState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReqType { SpeedMax, AltitudeRange, ObstacleClearance }

#[derive(Clone, Debug)]
pub struct Requirement {
    pub name: String,
    pub req_type: ReqType,
    pub values: Vec<f32>,
    pub severity: String,
}

pub fn parse_requirements(input: &str) -> Result<Vec<Requirement>, String> {
    let mut reqs = Vec::new();
    for (line_no, raw) in input.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let (key, rest) = line.split_once(':')
            .ok_or_else(|| format!("line {}: missing ':'", line_no + 1))?;
        let key = key.trim();
        let parts: Vec<&str> = rest.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!("line {}: need <value(s)> <severity>", line_no + 1));
        }
        let severity = parts[parts.len() - 1].to_string();
        if severity != "HIGH" && severity != "MEDIUM" && severity != "LOW" {
            return Err(format!("line {}: severity must be HIGH/MEDIUM/LOW", line_no + 1));
        }
        let mut values = Vec::new();
        for s in &parts[..parts.len() - 1] {
            values.push(s.parse::<f32>().map_err(|_|
                format!("line {}: '{}' is not a number", line_no + 1, s))?);
        }
        let req_type = match key {
            "speed_max" => ReqType::SpeedMax,
            "altitude_range" => ReqType::AltitudeRange,
            "obstacle_clearance" => ReqType::ObstacleClearance,
            _ => return Err(format!("line {}: unknown type '{}'", line_no + 1, key)),
        };
        reqs.push(Requirement { name: key.to_string(), req_type, values, severity });
    }
    Ok(reqs)
}

pub fn generate_test(req: &Requirement) -> String {
    let check = match req.req_type {
        ReqType::SpeedMax => format!("assert!(velocity <= {:.1}, \"speed_max violated: {{}}\", velocity);", req.values[0]),
        ReqType::AltitudeRange => format!("assert!(altitude >= {:.1} && altitude <= {:.1}, \"altitude_range violated: {{}}\", altitude);", req.values[0], req.values[1]),
        ReqType::ObstacleClearance => format!("assert!(distance >= {:.1}, \"obstacle_clearance violated: {{}}\", distance);", req.values[0]),
    };
    format!("#[test]\nfn test_{}_invariant() {{\n    // Requirement: {}\n    // Severity: {}\n    // {}\n    let _ = ();\n}}\n", req.name, req.name, req.severity, check)
}

pub fn generate_gsn(req: &Requirement, block_idx: usize) -> String {
    let desc = match req.req_type {
        ReqType::SpeedMax => format!("velocity <= {:.1} m/s", req.values[0]),
        ReqType::AltitudeRange => format!("{:.1} m <= altitude <= {:.1} m", req.values[0], req.values[1]),
        ReqType::ObstacleClearance => format!("distance_to_obstacle >= {:.1} m", req.values[0]),
    };
    format!("### G_{}: {}\n- **Type**: Invariant\n- **Severity**: {}\n- **Evidence**: `test_{}_invariant`\n- **Chain block**: {}\n\n", req.name, desc, req.severity, req.name, block_idx)
}

pub fn compile_requirements(reqs: &[Requirement]) -> Result<CertChain, String> {
    let mut chain = CertChain::new(1, "scc_compiled");
    for (i, r) in reqs.iter().enumerate() {
        let mut buf = Vec::new();
        buf.extend_from_slice(r.name.as_bytes());
        for v in &r.values { buf.extend_from_slice(&v.to_le_bytes()); }
        buf.extend_from_slice(r.severity.as_bytes());
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in &buf { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        chain.append(i as f32, 1, h, SafetyState::Nominal)
            .map_err(|e| format!("chain error at block {}: {:?}", i, e))?;
    }
    Ok(chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single() {
        let r = parse_requirements("speed_max: 10.0 HIGH").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].req_type, ReqType::SpeedMax);
        assert_eq!(r[0].severity, "HIGH");
    }

    #[test]
    fn parse_multiple() {
        let input = "# c\nspeed_max: 10.0 HIGH\naltitude_range: 5.0 120.0 HIGH\nobstacle_clearance: 2.0 MEDIUM";
        assert_eq!(parse_requirements(input).unwrap().len(), 3);
    }

    #[test]
    fn parse_invalid_no_colon() {
        assert!(parse_requirements("speed_max 10.0 HIGH").is_err());
    }

    #[test]
    fn parse_invalid_unknown_type() {
        assert!(parse_requirements("foo: 1.0 HIGH").is_err());
    }

    #[test]
    fn parse_invalid_float() {
        assert!(parse_requirements("speed_max: abc HIGH").is_err());
    }

    #[test]
    fn parse_altitude_two_values() {
        let r = parse_requirements("altitude_range: 5.0 120.0 HIGH").unwrap();
        assert_eq!(r[0].values, vec![5.0, 120.0]);
    }

    #[test]
    fn test_stub_contains_name_and_check() {
        let r = parse_requirements("speed_max: 10.0 HIGH").unwrap();
        let s = generate_test(&r[0]);
        assert!(s.contains("test_speed_max_invariant"));
        assert!(s.contains("velocity <= 10.0"));
    }

    #[test]
    fn gsn_contains_name_and_block() {
        let r = parse_requirements("altitude_range: 5.0 120.0 HIGH").unwrap();
        let s = generate_gsn(&r[0], 0);
        assert!(s.contains("G_altitude_range"));
        assert!(s.contains("Chain block**: 0"));
    }

    #[test]
    fn chain_grows_with_requirements() {
        let input = "speed_max: 10.0 HIGH\naltitude_range: 5.0 120.0 HIGH";
        let reqs = parse_requirements(input).unwrap();
        let chain = compile_requirements(&reqs).unwrap();
        assert_eq!(chain.block_count(), 2);
        assert!(chain.verify());
    }

    #[test]
    fn chain_deterministic() {
        let input = "speed_max: 10.0 HIGH";
        let c1 = compile_requirements(&parse_requirements(input).unwrap()).unwrap();
        let c2 = compile_requirements(&parse_requirements(input).unwrap()).unwrap();
        assert_eq!(c1.root_hash(), c2.root_hash());
    }
}
