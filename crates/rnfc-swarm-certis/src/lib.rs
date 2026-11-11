#![allow(clippy::new_without_default)]
//! Swarm-CERTIS — Distributed Runtime Certification for UAV Swarms.
//!
//! Extensao do rnfc-certis para enxames. Cada drone mantem uma hash chain local
//! de eventos de seguranca coletivos. Periodicamente sincroniza com peers
//! (last-write-wins com verificacao de hash), construindo uma hash chain global
//! do enxame. No fim da missao, o SwarmSafetyCaseGenerator produz o safety case
//! coletivo (GSN + FMEA + FTA de enxame).

use std::collections::HashMap;
use std::vec::Vec;

// ============ EVENTOS DE ENXAME ============
#[derive(Clone, Debug, PartialEq)]
pub enum SwarmSafetyEventKind {
    /// Dois drones estavam em rota de colisao mas foi evitada.
    DroneCollisionAvoided { other_id: u8, min_dist_m: f32 },
    /// Um drone afastou-se demais do centro do enxame.
    SwarmCohesionLost { distance_m: f32 },
    /// A malha de comunicacao degradou-se (perda de pacotes).
    CommMeshDegraded { packet_loss_pct: f32 },
    /// Um membro reportou o seu estado (heartbeat).
    Heartbeat { battery_pct: u8 },
}

#[derive(Clone, Debug)]
pub struct SwarmSafetyEvent {
    pub drone_id: u8,
    pub timestamp_s: f32,
    pub kind: SwarmSafetyEventKind,
    pub hash: u64,
    pub prev_hash: u64,
}

// ============ HASH CHAIN (FNV-1a) ============
fn fnv1a_64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ============ RECORDER DISTRIBUIDO ============
pub struct DistributedCertisRecorder {
    pub drone_id: u8,
    pub local_events: Vec<SwarmSafetyEvent>,
    pub peers: HashMap<u8, u64>,     // peer_id -> last seen global hash
    pub mission_id: String,
}

impl DistributedCertisRecorder {
    pub fn new(drone_id: u8, mission_id: &str) -> Self {
        Self {
            drone_id,
            local_events: Vec::new(),
            peers: HashMap::new(),
            mission_id: mission_id.into(),
        }
    }

    /// Regista um evento de enxame. Hash chain: prev_hash = hash do evento anterior.
    pub fn record(&mut self, timestamp_s: f32, kind: SwarmSafetyEventKind) -> u64 {
        let prev_hash = self.local_events.last().map(|e| e.hash).unwrap_or(0);
        let mut buf = Vec::new();
        buf.extend_from_slice(&[self.drone_id]);
        buf.extend_from_slice(&timestamp_s.to_le_bytes());
        buf.extend_from_slice(format!("{:?}", kind).as_bytes());
        buf.extend_from_slice(&prev_hash.to_le_bytes());
        let hash = fnv1a_64(&buf);

        self.local_events.push(SwarmSafetyEvent {
            drone_id: self.drone_id,
            timestamp_s,
            kind,
            hash,
            prev_hash,
        });
        hash
    }

    /// Verifica integridade da hash chain local.
    pub fn verify_local_chain(&self) -> bool {
        let mut prev = 0u64;
        for e in &self.local_events {
            if e.prev_hash != prev { return false; }
            let mut buf = Vec::new();
            buf.extend_from_slice(&[e.drone_id]);
            buf.extend_from_slice(&e.timestamp_s.to_le_bytes());
            buf.extend_from_slice(format!("{:?}", e.kind).as_bytes());
            buf.extend_from_slice(&e.prev_hash.to_le_bytes());
            if fnv1a_64(&buf) != e.hash { return false; }
            prev = e.hash;
        }
        true
    }

    /// Sincroniza com um peer. Last-write-wins: guarda o hash do peer.
    /// Devolve true se o peer foi actualizado (novo hash).
    pub fn sync_with_peer(&mut self, peer_id: u8, peer_hash: u64) -> bool {
        match self.peers.get(&peer_id) {
            Some(&prev) if prev == peer_hash => false,
            _ => {
                self.peers.insert(peer_id, peer_hash);
                true
            }
        }
    }

    /// Hash de consenso global: FNV-1a sobre (drone_id, hash) de TODOS os membros
    /// (self + peers), ordenados por drone_id. Convergente: dois drones com a
    /// mesma visao do enxame produzem o mesmo hash.
    pub fn global_consensus(&self) -> u64 {
        // Construir lista de TODOS os (drone_id, hash) - incluir self + peers
        let mut all: Vec<(u8, u64)> = Vec::new();
        let local_hash = self.local_events.last().map(|e| e.hash).unwrap_or(0);
        all.push((self.drone_id, local_hash));
        for (id, h) in &self.peers {
            all.push((*id, *h));
        }
        // Ordenar TUDO por drone_id
        all.sort_by_key(|(id, _)| *id);

        // Serializar
        let mut buf = Vec::new();
        for (id, h) in &all {
            buf.extend_from_slice(&[*id]);
            buf.extend_from_slice(&h.to_le_bytes());
        }
        fnv1a_64(&buf)
    }

    /// Numero de peers conhecidos.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

// ============ TESTES ============
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_new_empty() {
        let r = DistributedCertisRecorder::new(1, "swarm_001");
        assert_eq!(r.local_events.len(), 0);
        assert_eq!(r.peer_count(), 0);
    }

    #[test]
    fn record_single_event() {
        let mut r = DistributedCertisRecorder::new(1, "m");
        let h = r.record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        assert_ne!(h, 0);
        assert_eq!(r.local_events.len(), 1);
        assert!(r.verify_local_chain());
    }

    #[test]
    fn record_multiple_events_chain_valid() {
        let mut r = DistributedCertisRecorder::new(2, "m");
        r.record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        r.record(2.0, SwarmSafetyEventKind::DroneCollisionAvoided { other_id: 1, min_dist_m: 3.5 });
        r.record(3.0, SwarmSafetyEventKind::SwarmCohesionLost { distance_m: 45.0 });
        assert!(r.verify_local_chain());
        assert_eq!(r.local_events.len(), 3);
    }

    #[test]
    fn tamper_detected() {
        let mut r = DistributedCertisRecorder::new(3, "m");
        r.record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        r.record(2.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 90 });
        // Modificar evento
        r.local_events[0].timestamp_s = 999.0;
        assert!(!r.verify_local_chain());
    }

    #[test]
    fn sync_with_peer_new_returns_true() {
        let mut r = DistributedCertisRecorder::new(1, "m");
        assert!(r.sync_with_peer(2, 0xABCD));
        assert_eq!(r.peer_count(), 1);
    }

    #[test]
    fn sync_with_peer_same_returns_false() {
        let mut r = DistributedCertisRecorder::new(1, "m");
        r.sync_with_peer(2, 0xABCD);
        assert!(!r.sync_with_peer(2, 0xABCD), "mesmo hash -> false");
    }

    #[test]
    fn sync_with_peer_new_hash_returns_true() {
        let mut r = DistributedCertisRecorder::new(1, "m");
        r.sync_with_peer(2, 0xABCD);
        assert!(r.sync_with_peer(2, 0x1234), "novo hash -> true");
    }

    #[test]
    fn global_consensus_deterministic() {
        let mut r1 = DistributedCertisRecorder::new(1, "m");
        r1.record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        r1.sync_with_peer(2, 0x2222);
        r1.sync_with_peer(3, 0x3333);

        let mut r2 = DistributedCertisRecorder::new(1, "m");
        r2.record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        r2.sync_with_peer(3, 0x3333);
        r2.sync_with_peer(2, 0x2222);

        // Ordem de sync nao importa - consenso deterministico
        assert_eq!(r1.global_consensus(), r2.global_consensus());
    }

    #[test]
    fn global_consensus_differs_with_different_peers() {
        let mut r1 = DistributedCertisRecorder::new(1, "m");
        r1.sync_with_peer(2, 0x2222);
        let c1 = r1.global_consensus();

        let mut r2 = DistributedCertisRecorder::new(1, "m");
        r2.sync_with_peer(2, 0x9999);
        let c2 = r2.global_consensus();

        assert_ne!(c1, c2, "peers diferentes -> consenso diferente");
    }

    #[test]
    fn two_drones_converge_to_same_consensus() {
        let mut r1 = DistributedCertisRecorder::new(1, "m");
        let mut r2 = DistributedCertisRecorder::new(2, "m");

        // Cada drone regista um evento
        r1.record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        r2.record(1.1, SwarmSafetyEventKind::Heartbeat { battery_pct: 90 });

        // Sincronizam mutuamente
        let h1 = r1.local_events[0].hash;
        let h2 = r2.local_events[0].hash;
        r1.sync_with_peer(2, h2);
        r2.sync_with_peer(1, h1);

        // Consenso global e o mesmo (porque os peers estao ordenados)
        assert_eq!(r1.global_consensus(), r2.global_consensus());
    }
}


// ============ SWARM SAFETY CASE ============
// Fase 2: gera GSN + FMEA + FTA de enxame a partir de multiplos DistributedCertisRecorder.

use rnfc_certis::{FaultNode, FmeaEntry, GsnNode, fault_tree_to_json, fmea_to_markdown, gsn_to_markdown};

/// Safety case distribuido do enxame.
pub struct SwarmSafetyCase {
    pub mission_id: String,
    pub drone_count: usize,
    pub consensus_hash: u64,
    pub all_drones_agree: bool,      // true se todos tem o mesmo consenso
    pub swarm_fmea: Vec<FmeaEntry>,
    pub swarm_gsn: Vec<GsnNode>,
    pub swarm_fault_tree: FaultNode,
}

/// Calcula o consenso global do enxame e verifica convergencia.
/// Devolve (hash_representativo, todos_de_acordo).
pub fn swarm_consensus(recorders: &[DistributedCertisRecorder]) -> (u64, bool) {
    if recorders.is_empty() {
        return (0, true);
    }
    let first = recorders[0].global_consensus();
    let all_agree = recorders.iter().all(|r| r.global_consensus() == first);
    (first, all_agree)
}

/// Gera FMEA de enxame: 4 modos especificos de swarm + evidencia agregada.
pub fn generate_swarm_fmea(recorders: &[DistributedCertisRecorder]) -> Vec<FmeaEntry> {
    let mut entries = vec![
        FmeaEntry::new(
            "Intra-swarm collision",
            "Two drones on converging trajectory",
            10, 3, 1,
            "Collision avoidance protocol (predict 2s ahead)",
        ),
        FmeaEntry::new(
            "Swarm cohesion loss",
            "Drone drifts away from formation",
            6, 4, 2,
            "Formation control + consensus",
        ),
        FmeaEntry::new(
            "Comm mesh degradation",
            "Packet loss in inter-drone radio link",
            7, 5, 2,
            "Fallback to autonomous navigation",
        ),
        FmeaEntry::new(
            "Consensus split-brain",
            "Distributed state diverges between drones",
            8, 2, 3,
            "Last-write-wins + global hash verification",
        ),
    ];

    // Contar eventos por tipo em todos os recorders
    let mut collision_count = 0u32;
    let mut cohesion_count = 0u32;
    let mut comm_count = 0u32;
    for r in recorders {
        for e in &r.local_events {
            match &e.kind {
                SwarmSafetyEventKind::DroneCollisionAvoided { .. } => collision_count += 1,
                SwarmSafetyEventKind::SwarmCohesionLost { .. } => cohesion_count += 1,
                SwarmSafetyEventKind::CommMeshDegraded { .. } => comm_count += 1,
                SwarmSafetyEventKind::Heartbeat { .. } => {}
            }
        }
    }
    entries[0].evidence_count = collision_count;
    entries[1].evidence_count = cohesion_count;
    entries[2].evidence_count = comm_count;

    // Split-brain: 1 se todos NAO concordam
    let (_, all_agree) = swarm_consensus(recorders);
    entries[3].evidence_count = if all_agree { 0 } else { 1 };

    entries
}

/// Gera GSN de enxame: goal principal + sub-goals por classe de risco coletivo.
pub fn generate_swarm_gsn(recorders: &[DistributedCertisRecorder]) -> Vec<GsnNode> {
    let total_events: usize = recorders.iter().map(|r| r.local_events.len()).sum();
    let (_, all_agree) = swarm_consensus(recorders);

    let mut gsn = vec![
        GsnNode::Goal {
            id: "SG1".into(),
            text: "Swarm mission is safe as a collective".into(),
        },
        GsnNode::Strategy {
            id: "SS1".into(),
            text: "Argument over individual + collective safety".into(),
        },
        GsnNode::Goal {
            id: "SG1.1".into(),
            text: "Each drone is individually safe (CERTIS per-drone)".into(),
        },
        GsnNode::Solution {
            id: "SSn1.1".into(),
            text: format!(
                "{} drones, {} safety events logged across swarm",
                recorders.len(), total_events
            ),
        },
        GsnNode::Goal {
            id: "SG1.2".into(),
            text: "Distributed consensus is achieved".into(),
        },
    ];

    if all_agree {
        gsn.push(GsnNode::Solution {
            id: "SSn1.2".into(),
            text: format!("All {} drones agree on consensus hash {:#018x}",
                recorders.len(), recorders.first().map(|r| r.global_consensus()).unwrap_or(0)),
        });
    } else {
        // Split-brain detectado - adicionar como falha na GSN
        gsn.push(GsnNode::Solution {
            id: "SSn1.2".into(),
            text: "SPLIT-BRAIN: drones disagree on consensus".into(),
        });
    }

    // Collision avoidance goal - so se ha eventos
    let has_collision = recorders.iter().any(|r| r.local_events.iter().any(|e| {
        matches!(e.kind, SwarmSafetyEventKind::DroneCollisionAvoided { .. })
    }));
    if has_collision {
        let count: usize = recorders.iter().flat_map(|r| &r.local_events).filter(|e| {
            matches!(e.kind, SwarmSafetyEventKind::DroneCollisionAvoided { .. })
        }).count();
        gsn.push(GsnNode::Goal {
            id: "SG1.3".into(),
            text: "Collision avoidance within swarm is functional".into(),
        });
        gsn.push(GsnNode::Solution {
            id: "SSn1.3".into(),
            text: format!("{} collisions avoided during mission", count),
        });
    }

    // Cohesion goal
    let has_cohesion = recorders.iter().any(|r| r.local_events.iter().any(|e| {
        matches!(e.kind, SwarmSafetyEventKind::SwarmCohesionLost { .. })
    }));
    if has_cohesion {
        let count: usize = recorders.iter().flat_map(|r| &r.local_events).filter(|e| {
            matches!(e.kind, SwarmSafetyEventKind::SwarmCohesionLost { .. })
        }).count();
        gsn.push(GsnNode::Goal {
            id: "SG1.4".into(),
            text: "Swarm cohesion is maintained".into(),
        });
        gsn.push(GsnNode::Solution {
            id: "SSn1.4".into(),
            text: format!("{} cohesion loss events (recovered)", count),
        });
    }

    gsn
}

/// Gera Fault Tree de enxame: TOP -> filhos (um por drone + coordenacao).
pub fn generate_swarm_fault_tree(recorders: &[DistributedCertisRecorder]) -> FaultNode {
    // Filhos: um por drone + um para coordenacao
    let mut children: Vec<FaultNode> = Vec::new();
    for r in recorders {
        let count = r.local_events.len() as u32;
        children.push(FaultNode {
            id: format!("drone_{}", r.drone_id),
            description: format!("Drone {} safety events", r.drone_id),
            event_count: count,
            mitigation: Some("Local CERTIS + FDIR".into()),
            children: vec![],
        });
    }

    // Coordenacao de enxame
    let coord_count: u32 = recorders.iter()
        .flat_map(|r| &r.local_events)
        .filter(|e| matches!(
            e.kind,
            SwarmSafetyEventKind::DroneCollisionAvoided { .. }
                | SwarmSafetyEventKind::SwarmCohesionLost { .. }
                | SwarmSafetyEventKind::CommMeshDegraded { .. }
        ))
        .count() as u32;

    children.push(FaultNode {
        id: "swarm_coordination".into(),
        description: "Swarm coordination events".into(),
        event_count: coord_count,
        mitigation: Some("Distributed consensus + collision avoidance".into()),
        children: vec![],
    });

    children.sort_by(|a, b| a.id.cmp(&b.id));

    let total: u32 = recorders.iter().map(|r| r.local_events.len() as u32).sum();
    FaultNode {
        id: "SWARM_TOP".into(),
        description: format!(
            "Swarm mission {} ({} drones)",
            recorders.first().map(|r| &r.mission_id).cloned().unwrap_or_default(),
            recorders.len()
        ),
        event_count: total,
        mitigation: None,
        children,
    }
}

impl SwarmSafetyCase {
    pub fn generate(recorders: &[DistributedCertisRecorder]) -> Self {
        let (consensus_hash, all_agree) = swarm_consensus(recorders);
        let mission_id = recorders.first()
            .map(|r| r.mission_id.clone())
            .unwrap_or_else(|| "swarm_empty".into());
        Self {
            mission_id,
            drone_count: recorders.len(),
            consensus_hash,
            all_drones_agree: all_agree,
            swarm_fmea: generate_swarm_fmea(recorders),
            swarm_gsn: generate_swarm_gsn(recorders),
            swarm_fault_tree: generate_swarm_fault_tree(recorders),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("# Swarm-CERTIS Safety Case — Mission {}\n\n", self.mission_id));
        s.push_str(&format!("**Drones:** {}\n\n", self.drone_count));
        s.push_str(&format!("**Consensus hash:** `{:#018x}`\n\n", self.consensus_hash));
        s.push_str(&format!(
            "**Consensus status:** {}\n\n",
            if self.all_drones_agree { "ALL DRONES AGREE" } else { "SPLIT-BRAIN DETECTED" }
        ));
        s.push_str("---\n\n## 1. GSN (Swarm Goal Structuring)\n\n");
        s.push_str(&gsn_to_markdown(&self.swarm_gsn, &self.mission_id));
        s.push_str("\n---\n\n## 2. FMEA (Swarm Failure Modes)\n\n");
        s.push_str(&fmea_to_markdown(&self.swarm_fmea, &self.mission_id));
        s.push_str("\n---\n\n## 3. Fault Tree Analysis (Swarm FTA)\n\n");
        s.push_str("```json\n");
        s.push_str(&fault_tree_to_json(&self.swarm_fault_tree, 0));
        s.push_str("\n```\n");
        s
    }
}

// ============ TESTES FASE 2 ============
#[cfg(test)]
mod tests_fase2 {
    use super::*;

    fn make_swarm(n: u8) -> Vec<DistributedCertisRecorder> {
        (1..=n).map(|id| DistributedCertisRecorder::new(id, "swarm_test")).collect()
    }

    fn sync_all(recorders: &mut [DistributedCertisRecorder]) {
        // Todos os drones trocam hash chains (full mesh)
        let hashes: Vec<(u8, u64)> = recorders.iter()
            .map(|r| (r.drone_id, r.local_events.last().map(|e| e.hash).unwrap_or(0)))
            .collect();
        for r in recorders.iter_mut() {
            for &(id, h) in &hashes {
                if id != r.drone_id {
                    r.sync_with_peer(id, h);
                }
            }
        }
    }

    #[test]
    fn swarm_fmea_has_4_swarm_modes() {
        let r = make_swarm(3);
        let fmea = generate_swarm_fmea(&r);
        assert_eq!(fmea.len(), 4);
        assert!(fmea.iter().any(|e| e.failure_mode.contains("collision")));
        assert!(fmea.iter().any(|e| e.failure_mode.contains("cohesion")));
        assert!(fmea.iter().any(|e| e.failure_mode.contains("Comm")));
        assert!(fmea.iter().any(|e| e.failure_mode.contains("Consensus")));
    }

    #[test]
    fn swarm_fmea_counts_events() {
        let mut r = make_swarm(2);
        r[0].record(1.0, SwarmSafetyEventKind::DroneCollisionAvoided { other_id: 2, min_dist_m: 3.0 });
        r[0].record(2.0, SwarmSafetyEventKind::DroneCollisionAvoided { other_id: 2, min_dist_m: 2.5 });
        r[1].record(1.5, SwarmSafetyEventKind::SwarmCohesionLost { distance_m: 50.0 });
        let fmea = generate_swarm_fmea(&r);
        let collision_entry = fmea.iter().find(|e| e.failure_mode.contains("collision")).unwrap();
        assert_eq!(collision_entry.evidence_count, 2);
        let cohesion_entry = fmea.iter().find(|e| e.failure_mode.contains("cohesion")).unwrap();
        assert_eq!(cohesion_entry.evidence_count, 1);
    }

    #[test]
    fn swarm_gsn_includes_consensus_goal() {
        let r = make_swarm(2);
        let gsn = generate_swarm_gsn(&r);
        assert!(gsn.iter().any(|n| matches!(n, GsnNode::Goal { id, .. } if id == "SG1.2")));
    }

    #[test]
    fn swarm_gsn_split_brain_detected() {
        let mut r = make_swarm(2);
        r[0].record(1.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 95 });
        r[1].record(2.0, SwarmSafetyEventKind::Heartbeat { battery_pct: 90 });
        // NAO sincronizam -> split brain
        let (_, all_agree) = swarm_consensus(&r);
        assert!(!all_agree, "deveria ser split-brain sem sync");
        let gsn = generate_swarm_gsn(&r);
        let consensus_solution = gsn.iter().find(|n| matches!(n, GsnNode::Solution { id, .. } if id == "SSn1.2")).unwrap();
        if let GsnNode::Solution { text, .. } = consensus_solution {
            assert!(text.contains("SPLIT-BRAIN"));
        }
    }

    #[test]
    fn swarm_gsn_coherent_after_sync() {
        let mut r = make_swarm(3);
        for (i, rec) in r.iter_mut().enumerate() {
            rec.record(i as f32, SwarmSafetyEventKind::Heartbeat { battery_pct: 90 });
        }
        sync_all(&mut r);
        let (_, all_agree) = swarm_consensus(&r);
        assert!(all_agree, "deveria estar coerente apos sync");
    }

    #[test]
    fn swarm_gsn_collision_goal_if_events() {
        let mut r = make_swarm(2);
        r[0].record(1.0, SwarmSafetyEventKind::DroneCollisionAvoided { other_id: 2, min_dist_m: 3.0 });
        let gsn = generate_swarm_gsn(&r);
        assert!(gsn.iter().any(|n| matches!(n, GsnNode::Goal { id, .. } if id == "SG1.3")));
    }

    #[test]
    fn swarm_gsn_no_collision_goal_if_no_events() {
        let r = make_swarm(2);
        let gsn = generate_swarm_gsn(&r);
        assert!(!gsn.iter().any(|n| matches!(n, GsnNode::Goal { id, .. } if id == "SG1.3")));
    }

    #[test]
    fn swarm_fault_tree_aggregates_drones() {
        let r = make_swarm(3);
        let tree = generate_swarm_fault_tree(&r);
        assert_eq!(tree.id, "SWARM_TOP");
        assert_eq!(tree.children.len(), 4);  // 3 drones + swarm_coordination
    }

    #[test]
    fn empty_swarm_case() {
        let r: Vec<DistributedCertisRecorder> = vec![];
        let case = SwarmSafetyCase::generate(&r);
        assert_eq!(case.drone_count, 0);
        assert_eq!(case.consensus_hash, 0);
        assert_eq!(case.swarm_fmea.len(), 4);  // still has 4 base modes
    }

    #[test]
    fn swarm_safety_case_aggregates() {
        let mut r = make_swarm(3);
        r[0].record(1.0, SwarmSafetyEventKind::DroneCollisionAvoided { other_id: 2, min_dist_m: 3.0 });
        r[1].record(2.0, SwarmSafetyEventKind::SwarmCohesionLost { distance_m: 45.0 });
        r[2].record(3.0, SwarmSafetyEventKind::CommMeshDegraded { packet_loss_pct: 15.0 });
        sync_all(&mut r);
        let case = SwarmSafetyCase::generate(&r);
        assert_eq!(case.drone_count, 3);
        assert!(case.all_drones_agree);
        assert_eq!(case.swarm_fmea.len(), 4);
        assert!(case.swarm_gsn.len() >= 5);  // SG1, SS1, SG1.1, SSn1.1, SG1.2, SSn1.2, SG1.3, ...
        assert_eq!(case.swarm_fault_tree.id, "SWARM_TOP");
    }

    #[test]
    fn swarm_safety_case_markdown() {
        let mut r = make_swarm(2);
        r[0].record(1.0, SwarmSafetyEventKind::DroneCollisionAvoided { other_id: 2, min_dist_m: 2.5 });
        sync_all(&mut r);
        let case = SwarmSafetyCase::generate(&r);
        let md = case.to_markdown();
        assert!(md.contains("# Swarm-CERTIS Safety Case"));
        assert!(md.contains("Consensus hash"));
        assert!(md.contains("GSN"));
        assert!(md.contains("FMEA"));
        assert!(md.contains("SWARM_TOP"));
    }
}

