//! CERTIS — Certifiable Evidence from Real-Time Integrated Safety.
//!
//! Compilador de safety cases a partir de eventos de runtime do RNFC.
//!
//! Objectivo comercial: gerar automaticamente artefactos de certificacao
//! (FTA, FMEA, GSN) que a EASA/FAA exigem para drones BVLOS. O RNFC ja tem
//! o FDIR e o LNN-FTC; o CERTIS transforma-os em evidencia certificavel.
//!
//! Fase 1 (este crate): SafetyEvent + CertisRecorder + FTA generator.

use std::collections::HashMap;
use std::vec::Vec;

// ============ EVENTOS ============
/// Tipo de evento de safety registado durante a missao.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SafetyEventType {
    // FDIR — sensores
    SensorStuck,
    SensorSpoof,
    SensorOutlier,
    VoterIsolated { sensor: u8 },
    ZombieDetected,
    FailsafeTriggered { action: FailsafeActionKind },
    // LNN-FTC — atuadores
    MotorLossDetected { motor: u8 },
    LnnRecoveryStarted,
    LnnRecoverySucceeded,
    LnnRecoveryFailed,
    // Pilot authority
    PilotOverride { command: String },
    PilotResisted { reason: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailsafeActionKind {
    ReturnToLaunch,
    Hover,
    EmergencyLand,
}

/// Um evento de safety com timestamp e hash do evento anterior.
#[derive(Clone, Debug)]
pub struct SafetyEvent {
    pub timestamp_s: f32,
    pub kind: SafetyEventType,
    pub hash: u64, // hash chain: sha-like do evento + hash prev
    pub prev_hash: u64,
}

// ============ HASH CHAIN (FNV-1a 64-bit, no_std-friendly) ============
fn fnv1a_64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ============ RECORDER ============
/// Regista eventos de safety com hash chain tamper-evident.
pub struct CertisRecorder {
    pub events: Vec<SafetyEvent>,
    pub mission_id: String,
}

impl CertisRecorder {
    pub fn new(mission_id: &str) -> Self {
        Self {
            events: Vec::new(),
            mission_id: mission_id.into(),
        }
    }

    /// Regista um evento. Hash chain: novo hash depende do hash anterior.
    pub fn record(&mut self, timestamp_s: f32, kind: SafetyEventType) -> u64 {
        let prev_hash = self.events.last().map(|e| e.hash).unwrap_or(0);
        let mut buf = Vec::new();
        buf.extend_from_slice(&timestamp_s.to_le_bytes());
        buf.extend_from_slice(format!("{:?}", kind).as_bytes());
        buf.extend_from_slice(&prev_hash.to_le_bytes());
        let hash = fnv1a_64(&buf);

        self.events.push(SafetyEvent {
            timestamp_s,
            kind,
            hash,
            prev_hash,
        });
        hash
    }

    /// Verifica integridade da hash chain (tamper detection).
    pub fn verify_chain(&self) -> bool {
        let mut prev_hash = 0u64;
        for e in &self.events {
            if e.prev_hash != prev_hash {
                return false;
            }
            let mut buf = Vec::new();
            buf.extend_from_slice(&e.timestamp_s.to_le_bytes());
            buf.extend_from_slice(format!("{:?}", e.kind).as_bytes());
            buf.extend_from_slice(&e.prev_hash.to_le_bytes());
            if fnv1a_64(&buf) != e.hash {
                return false;
            }
            prev_hash = e.hash;
        }
        true
    }

    /// Hash final da cadeia (para assinar o safety case).
    pub fn root_hash(&self) -> u64 {
        self.events.last().map(|e| e.hash).unwrap_or(0)
    }
}

// ============ FAULT TREE ============
/// No de uma Fault Tree (FTA).
#[derive(Clone, Debug)]
pub struct FaultNode {
    pub id: String,
    pub description: String,
    pub event_count: u32,
    pub mitigation: Option<String>,
    pub children: Vec<FaultNode>,
}

/// Gera uma Fault Tree a partir dos eventos registados.
pub fn generate_fault_tree(recorder: &CertisRecorder) -> FaultNode {
    // Contagem por tipo de falha
    let mut counts: HashMap<String, (u32, String, Option<String>)> = HashMap::new();

    for e in &recorder.events {
        let (key, desc, mitigation) = match &e.kind {
            SafetyEventType::SensorStuck => (
                "sensor_stuck".into(),
                "Sensor stuck detected".into(),
                Some("ZombieDetector -> RTL".into()),
            ),
            SafetyEventType::SensorSpoof => (
                "sensor_spoof".into(),
                "Sensor spoofing detected".into(),
                Some("SpoofingDetector -> RTL".into()),
            ),
            SafetyEventType::SensorOutlier => (
                "sensor_outlier".into(),
                "Sensor outlier detected".into(),
                Some("SensorHealth warning".into()),
            ),
            SafetyEventType::VoterIsolated { sensor } => (
                format!("voter_isolated_{}", sensor),
                format!("Sensor {} isolated by voter", sensor),
                Some("SensorVoter 2-of-3".into()),
            ),
            SafetyEventType::ZombieDetected => (
                "zombie_orbit".into(),
                "Zombie orbit detected".into(),
                Some("ZombieDetector -> RTL latch".into()),
            ),
            SafetyEventType::FailsafeTriggered { action } => (
                format!("failsafe_{:?}", action).to_lowercase(),
                format!("Failsafe {:?} triggered", action),
                Some("Failsafe layer".into()),
            ),
            SafetyEventType::MotorLossDetected { motor } => (
                format!("motor_loss_{}", motor),
                format!("Motor {} loss detected", motor),
                Some("LNN-FTC recovery".into()),
            ),
            SafetyEventType::LnnRecoveryStarted => (
                "lnn_recovery_started".into(),
                "LNN recovery initiated".into(),
                Some("LNN-FTC active".into()),
            ),
            SafetyEventType::LnnRecoverySucceeded => (
                "lnn_recovery_success".into(),
                "LNN recovery succeeded".into(),
                Some("Motor loss survived".into()),
            ),
            SafetyEventType::LnnRecoveryFailed => (
                "lnn_recovery_failed".into(),
                "LNN recovery failed".into(),
                Some("EmergencyLand".into()),
            ),
            SafetyEventType::PilotOverride { .. } => (
                "pilot_override".into(),
                "Pilot override".into(),
                Some("Pilot authority preserved".into()),
            ),
            SafetyEventType::PilotResisted { .. } => (
                "pilot_resisted".into(),
                "Pilot resisted by safety envelope".into(),
                Some("HASE filter".into()),
            ),
        };

        let entry = counts.entry(key).or_insert((0, desc, mitigation));
        entry.0 += 1;
    }

    // Construir filhos
    let mut children: Vec<FaultNode> = counts
        .into_iter()
        .map(|(id, (count, desc, mit))| FaultNode {
            id,
            description: desc,
            event_count: count,
            mitigation: mit,
            children: vec![],
        })
        .collect();
    children.sort_by(|a, b| a.id.cmp(&b.id));

    FaultNode {
        id: "TOP".into(),
        description: format!("Mission {} safety events", recorder.mission_id),
        event_count: recorder.events.len() as u32,
        mitigation: None,
        children,
    }
}

/// Exporta a Fault Tree como JSON simples.
pub fn fault_tree_to_json(node: &FaultNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let pad2 = "  ".repeat(indent + 1);
    let mut s = String::new();
    s.push_str(&format!("{}{{\n", pad));
    s.push_str(&format!("{}\"id\": \"{}\",\n", pad2, node.id));
    s.push_str(&format!(
        "{}\"description\": \"{}\",\n",
        pad2, node.description
    ));
    s.push_str(&format!("{}\"event_count\": {},\n", pad2, node.event_count));
    if let Some(m) = &node.mitigation {
        s.push_str(&format!("{}\"mitigation\": \"{}\",\n", pad2, m));
    }
    if !node.children.is_empty() {
        s.push_str(&format!("{}\"children\": [\n", pad2));
        for (i, c) in node.children.iter().enumerate() {
            s.push_str(&fault_tree_to_json(c, indent + 2));
            if i < node.children.len() - 1 {
                s.push_str(",\n");
            } else {
                s.push_str("\n");
            }
        }
        s.push_str(&format!("{}]\n", pad2));
    }
    s.push_str(&format!("{}}}", pad));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_new_empty() {
        let r = CertisRecorder::new("test_001");
        assert_eq!(r.events.len(), 0);
        assert_eq!(r.root_hash(), 0);
    }

    #[test]
    fn recorder_hash_chain_valid() {
        let mut r = CertisRecorder::new("test_002");
        r.record(1.0, SafetyEventType::SensorStuck);
        r.record(2.0, SafetyEventType::ZombieDetected);
        r.record(3.0, SafetyEventType::MotorLossDetected { motor: 1 });
        assert!(r.verify_chain());
        assert_ne!(r.root_hash(), 0);
    }

    #[test]
    fn recorder_tamper_detected() {
        let mut r = CertisRecorder::new("test_003");
        r.record(1.0, SafetyEventType::SensorStuck);
        r.record(2.0, SafetyEventType::ZombieDetected);
        // Modificar um evento -> chain quebra
        r.events[0].timestamp_s = 999.0;
        assert!(!r.verify_chain(), "tamper nao detectado!");
    }

    #[test]
    fn fault_tree_counts_events() {
        let mut r = CertisRecorder::new("test_004");
        r.record(1.0, SafetyEventType::SensorStuck);
        r.record(2.0, SafetyEventType::SensorStuck);
        r.record(3.0, SafetyEventType::MotorLossDetected { motor: 1 });
        let tree = generate_fault_tree(&r);
        assert_eq!(tree.event_count, 3);
        assert_eq!(tree.children.len(), 2);
        let stuck_node = tree
            .children
            .iter()
            .find(|c| c.id == "sensor_stuck")
            .unwrap();
        assert_eq!(stuck_node.event_count, 2);
        assert!(stuck_node.mitigation.is_some());
    }

    #[test]
    fn fault_tree_has_mitigations() {
        let mut r = CertisRecorder::new("test_005");
        r.record(1.0, SafetyEventType::ZombieDetected);
        r.record(2.0, SafetyEventType::LnnRecoverySucceeded);
        let tree = generate_fault_tree(&r);
        for child in &tree.children {
            assert!(child.mitigation.is_some(), "no {} sem mitigacao", child.id);
        }
    }

    #[test]
    fn fault_tree_json_export() {
        let mut r = CertisRecorder::new("test_006");
        r.record(1.0, SafetyEventType::SensorSpoof);
        let tree = generate_fault_tree(&r);
        let json = fault_tree_to_json(&tree, 0);
        assert!(json.contains("\"id\": \"TOP\""));
        assert!(json.contains("sensor_spoof"));
        assert!(json.contains("event_count"));
    }

    #[test]
    fn empty_fault_tree() {
        let r = CertisRecorder::new("test_007");
        let tree = generate_fault_tree(&r);
        assert_eq!(tree.event_count, 0);
        assert_eq!(tree.children.len(), 0);
    }

    #[test]
    fn pilot_events_tracked() {
        let mut r = CertisRecorder::new("test_008");
        r.record(
            1.0,
            SafetyEventType::PilotOverride {
                command: "roll_right".into(),
            },
        );
        r.record(
            2.0,
            SafetyEventType::PilotResisted {
                reason: "geofence".into(),
            },
        );
        let tree = generate_fault_tree(&r);
        assert!(tree.children.iter().any(|c| c.id == "pilot_override"));
        assert!(tree.children.iter().any(|c| c.id == "pilot_resisted"));
    }
}

// ============ FMEA ============
/// Entrada FMEA: modo de falha, severidade, ocorrencias, deteccao, mitigacao.
#[derive(Clone, Debug)]
pub struct FmeaEntry {
    pub failure_mode: String,
    pub cause: String,
    pub severity: u8,   // 1-10 (10 = catastrófico)
    pub occurrence: u8, // 1-10 (10 = frequente)
    pub detection: u8,  // 1-10 (10 = indetectável)
    pub rpn: u16,       // Risk Priority Number = sev * occ * det
    pub mitigation: String,
    pub evidence_count: u32, // Nº de vezes que ocorreu (dos eventos)
}

impl FmeaEntry {
    pub fn new(
        failure_mode: &str,
        cause: &str,
        severity: u8,
        occurrence: u8,
        detection: u8,
        mitigation: &str,
    ) -> Self {
        Self {
            failure_mode: failure_mode.into(),
            cause: cause.into(),
            severity,
            occurrence,
            detection,
            rpn: (severity as u16) * (occurrence as u16) * (detection as u16),
            mitigation: mitigation.into(),
            evidence_count: 0,
        }
    }

    pub fn risk_level(&self) -> &'static str {
        if self.rpn >= 200 {
            "CRITICAL"
        } else if self.rpn >= 100 {
            "HIGH"
        } else if self.rpn >= 50 {
            "MEDIUM"
        } else {
            "LOW"
        }
    }
}

/// Gera FMEA a partir dos eventos registados.
pub fn generate_fmea(recorder: &CertisRecorder) -> Vec<FmeaEntry> {
    let mut entries = vec![
        FmeaEntry::new(
            "Single sensor stuck",
            "Hardware failure in IMU/Mag",
            7,
            2,
            1,
            "SensorHealth detects stuck, ZombieDetector -> RTL",
        ),
        FmeaEntry::new(
            "Sensor spoofing",
            "External magnetic interference or adversarial attack",
            8,
            3,
            1,
            "SpoofingDetector (mag vs gyro divergence)",
        ),
        FmeaEntry::new(
            "Sensor outlier",
            "EMI, vibration, or calibration drift",
            5,
            5,
            2,
            "SensorVoter 2-of-3 majority voting",
        ),
        FmeaEntry::new(
            "Motor loss (actuator failure)",
            "Motor burn-out, ESC failure, or prop loss",
            10,
            1,
            3,
            "LNN-FTC two-phase recovery (hover + landing)",
        ),
        FmeaEntry::new(
            "Zombie orbit",
            "Adversarial wind + sensor noise causes stuck",
            6,
            4,
            2,
            "ZombieDetector (>60s stuck -> RTL latch)",
        ),
        FmeaEntry::new(
            "GNSS dropout",
            "Jamming, urban canyon, or tunnel",
            7,
            4,
            1,
            "Kalman dead-reckoning 10s",
        ),
        FmeaEntry::new(
            "Battery depletion",
            "Long mission or unexpected drain",
            7,
            3,
            2,
            "Failsafe RTL at 10.5V",
        ),
        FmeaEntry::new(
            "Geofence breach",
            "Pilot error or wind drift",
            6,
            2,
            1,
            "Failsafe ReturnToLaunch",
        ),
        FmeaEntry::new(
            "Pilot override unsafe command",
            "Human error",
            6,
            3,
            2,
            "HASE safety filter + haptic feedback",
        ),
    ];

    // Contar ocorrencias nos eventos
    for e in &recorder.events {
        let idx = match &e.kind {
            SafetyEventType::SensorStuck => Some(0),
            SafetyEventType::SensorSpoof => Some(1),
            SafetyEventType::SensorOutlier => Some(2),
            SafetyEventType::MotorLossDetected { .. } => Some(3),
            SafetyEventType::ZombieDetected => Some(4),
            SafetyEventType::PilotOverride { .. } => Some(8),
            _ => None,
        };
        if let Some(i) = idx {
            entries[i].evidence_count += 1;
        }
    }

    entries
}

/// Exporta FMEA como tabela Markdown.
pub fn fmea_to_markdown(entries: &[FmeaEntry], mission_id: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("# FMEA — Mission {}\n\n", mission_id));
    s.push_str(
        "| Failure Mode | Cause | Sev | Occ | Det | RPN | Level | Mitigation | Evidence |\n",
    );
    s.push_str("|---|---|---|---|---|---|---|---|---|\n");
    for e in entries {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            e.failure_mode,
            e.cause,
            e.severity,
            e.occurrence,
            e.detection,
            e.rpn,
            e.risk_level(),
            e.mitigation,
            e.evidence_count
        ));
    }
    s
}

// ============ GSN ============
/// Goal Structuring Notation (simplificado).
#[derive(Clone, Debug)]
pub enum GsnNode {
    Goal { id: String, text: String },
    Strategy { id: String, text: String },
    Solution { id: String, text: String },
}

/// Gera GSN a partir dos eventos registados.
pub fn generate_gsn(recorder: &CertisRecorder) -> Vec<GsnNode> {
    let mut gsn = vec![
        GsnNode::Goal {
            id: "G1".into(),
            text: "RNFC flight controller is safe during mission".into(),
        },
        GsnNode::Strategy {
            id: "S1".into(),
            text: "Argument over all fault classes: sensor, actuator, pilot".into(),
        },
    ];

    let has_sensor = recorder.events.iter().any(|e| {
        matches!(
            e.kind,
            SafetyEventType::SensorStuck
                | SafetyEventType::SensorSpoof
                | SafetyEventType::SensorOutlier
        )
    });
    if has_sensor {
        gsn.push(GsnNode::Goal {
            id: "G1.1".into(),
            text: "Sensor faults are mitigated by FDIR".into(),
        });
        gsn.push(GsnNode::Solution {
            id: "Sn1.1".into(),
            text: format!(
                "SensorHealth + SensorVoter detected {} events in runtime",
                recorder.events.len()
            ),
        });
    }

    let has_motor = recorder
        .events
        .iter()
        .any(|e| matches!(e.kind, SafetyEventType::MotorLossDetected { .. }));
    if has_motor {
        gsn.push(GsnNode::Goal {
            id: "G1.2".into(),
            text: "Actuator faults are recovered by LNN-FTC".into(),
        });
        let recovery_ok = recorder
            .events
            .iter()
            .any(|e| matches!(e.kind, SafetyEventType::LnnRecoverySucceeded));
        gsn.push(GsnNode::Solution {
            id: "Sn1.2".into(),
            text: if recovery_ok {
                "LNN-FTC recovery Succeeded".into()
            } else {
                "LNN-FTC recovery in progress/failed".into()
            },
        });
    }

    let has_pilot = recorder.events.iter().any(|e| {
        matches!(
            e.kind,
            SafetyEventType::PilotOverride { .. } | SafetyEventType::PilotResisted { .. }
        )
    });
    if has_pilot {
        gsn.push(GsnNode::Goal {
            id: "G1.3".into(),
            text: "Pilot authority is preserved with safety envelope".into(),
        });
        gsn.push(GsnNode::Solution {
            id: "Sn1.3".into(),
            text: "HASE safety filter + haptic feedback logged".into(),
        });
    }

    gsn
}

/// Exporta GSN como Markdown.
pub fn gsn_to_markdown(nodes: &[GsnNode], mission_id: &str) -> String {
    let mut s = format!("# GSN — Mission {}\n\n", mission_id);
    for n in nodes {
        match n {
            GsnNode::Goal { id, text } => s.push_str(&format!("- **Goal {}**: {}\n", id, text)),
            GsnNode::Strategy { id, text } => {
                s.push_str(&format!("  - *Strategy {}*: {}\n", id, text))
            }
            GsnNode::Solution { id, text } => {
                s.push_str(&format!("    - Evidence {}: {}\n", id, text))
            }
        }
    }
    s
}

// ============ SAFETY CASE AGREGADO ============
pub struct SafetyCase {
    pub mission_id: String,
    pub root_hash: u64,
    pub fault_tree: FaultNode,
    pub fmea: Vec<FmeaEntry>,
    pub gsn: Vec<GsnNode>,
}

impl SafetyCase {
    pub fn generate(recorder: &CertisRecorder) -> Self {
        Self {
            mission_id: recorder.mission_id.clone(),
            root_hash: recorder.root_hash(),
            fault_tree: generate_fault_tree(recorder),
            fmea: generate_fmea(recorder),
            gsn: generate_gsn(recorder),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "# CERTIS Safety Case — Mission {}\n\n",
            self.mission_id
        ));
        s.push_str(&format!(
            "**Root hash (tamper-evident):** `{:#018x}`\n\n",
            self.root_hash
        ));
        s.push_str("---\n\n## 1. GSN (Goal Structuring Notation)\n\n");
        s.push_str(&gsn_to_markdown(&self.gsn, &self.mission_id));
        s.push_str("\n---\n\n## 2. FMEA (Failure Mode and Effects Analysis)\n\n");
        s.push_str(&fmea_to_markdown(&self.fmea, &self.mission_id));
        s.push_str("\n---\n\n## 3. Fault Tree Analysis (FTA)\n\n");
        s.push_str("```json\n");
        s.push_str(&fault_tree_to_json(&self.fault_tree, 0));
        s.push_str("\n```\n");
        s
    }
}

// ============ TESTES FASE 2 ============
#[cfg(test)]
mod tests_fase2 {
    use super::*;

    #[test]
    fn fmea_has_9_entries() {
        let r = CertisRecorder::new("test_fmea");
        let fmea = generate_fmea(&r);
        assert_eq!(fmea.len(), 9);
    }

    #[test]
    fn fmea_rpn_calculated() {
        let e = FmeaEntry::new("test", "cause", 10, 2, 3, "mit");
        assert_eq!(e.rpn, 60);
        assert_eq!(e.risk_level(), "MEDIUM");
    }

    #[test]
    fn fmea_risk_critical() {
        let e = FmeaEntry::new("test", "cause", 10, 5, 5, "mit");
        assert!(e.rpn >= 200);
        assert_eq!(e.risk_level(), "CRITICAL");
    }

    #[test]
    fn fmea_counts_motor_loss_events() {
        let mut r = CertisRecorder::new("test");
        r.record(1.0, SafetyEventType::MotorLossDetected { motor: 1 });
        r.record(2.0, SafetyEventType::MotorLossDetected { motor: 2 });
        let fmea = generate_fmea(&r);
        let motor_entry = fmea
            .iter()
            .find(|e| e.failure_mode.contains("Motor"))
            .unwrap();
        assert_eq!(motor_entry.evidence_count, 2);
    }

    #[test]
    fn fmea_markdown_export() {
        let r = CertisRecorder::new("m1");
        let fmea = generate_fmea(&r);
        let md = fmea_to_markdown(&fmea, "m1");
        assert!(md.contains("# FMEA"));
        assert!(md.contains("Motor loss"));
        assert!(md.contains("|"));
    }

    #[test]
    fn gsn_includes_pilot_if_events() {
        let mut r = CertisRecorder::new("test");
        r.record(
            1.0,
            SafetyEventType::PilotOverride {
                command: "x".into(),
            },
        );
        let gsn = generate_gsn(&r);
        assert!(gsn
            .iter()
            .any(|n| matches!(n, GsnNode::Goal { id, .. } if id == "G1.3")));
    }

    #[test]
    fn gsn_excludes_motor_if_no_events() {
        let r = CertisRecorder::new("test");
        let gsn = generate_gsn(&r);
        assert!(!gsn
            .iter()
            .any(|n| matches!(n, GsnNode::Goal { id, .. } if id == "G1.2")));
    }

    #[test]
    fn safety_case_aggregates() {
        let mut r = CertisRecorder::new("mission_x");
        r.record(1.0, SafetyEventType::SensorStuck);
        r.record(2.0, SafetyEventType::MotorLossDetected { motor: 1 });
        r.record(3.0, SafetyEventType::LnnRecoverySucceeded);
        let sc = SafetyCase::generate(&r);
        assert_eq!(sc.mission_id, "mission_x");
        assert_eq!(sc.fmea.len(), 9);
        assert_ne!(sc.root_hash, 0);
        assert!(sc.gsn.len() >= 4); // G1, S1, G1.1, Sn1.1, G1.2, Sn1.2
    }

    #[test]
    fn safety_case_markdown_complete() {
        let mut r = CertisRecorder::new("m");
        r.record(1.0, SafetyEventType::SensorStuck);
        let sc = SafetyCase::generate(&r);
        let md = sc.to_markdown();
        assert!(md.contains("# CERTIS Safety Case"));
        assert!(md.contains("GSN"));
        assert!(md.contains("FMEA"));
        assert!(md.contains("Fault Tree"));
        assert!(md.contains("Root hash"));
    }
}
