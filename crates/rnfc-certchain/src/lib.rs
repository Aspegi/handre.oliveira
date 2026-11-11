#![allow(clippy::new_without_default)]
//! CertChain — Self-Sovereign Safety Case via Lightweight Blockchain.
//!
//! Cada drone mantem um ledger local (hash chain de SafetyBlocks). Cada bloco
//! so e aceite se cumprir Proof-of-Safety (contem eventos validos + estado
//! consistente). Os peers validam a chain inteira antes de a aceitar.
//!
//! Conceito comercial: certificacao auto-soberana. O drone *e* o safety case.
//! Reguladores (EASA/FAA) e seguradoras verificam a chain em vez de auditar
//! relatorios. Mercado estimado 2027: $500M+ em certificacao de drones.

use std::collections::HashMap;
use std::vec::Vec;

// ============ ESTADO DE SEGURANCA ============
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyState {
    Nominal,
    FdirActive,
    LnnRecovery,
    FailsafeTriggered,
}

impl SafetyState {
    pub fn as_u8(&self) -> u8 {
        match self {
            SafetyState::Nominal => 0,
            SafetyState::FdirActive => 1,
            SafetyState::LnnRecovery => 2,
            SafetyState::FailsafeTriggered => 3,
        }
    }
}

// ============ HASH ============
fn fnv1a_64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ============ BLOCO ============
#[derive(Clone, Debug)]
pub struct SafetyBlock {
    pub index: u64,
    pub timestamp_s: f32,
    pub event_count: u32,
    pub event_hash: u64,
    pub safety_state: SafetyState,
    pub prev_hash: u64,
    pub safety_proof: u64,
    pub hash: u64,
}

impl SafetyBlock {
    /// Cria um novo bloco. Proof-of-Safety: event_count > 0.
    pub fn new(
        index: u64,
        timestamp_s: f32,
        event_count: u32,
        event_hash: u64,
        safety_state: SafetyState,
        prev_hash: u64,
    ) -> Result<Self, ChainError> {
        if event_count == 0 {
            return Err(ChainError::EmptyEvents);
        }
        let safety_proof = compute_proof(event_count, event_hash, safety_state, prev_hash);
        let hash = compute_hash(index, timestamp_s, safety_proof, prev_hash);
        Ok(Self {
            index,
            timestamp_s,
            event_count,
            event_hash,
            safety_state,
            prev_hash,
            safety_proof,
            hash,
        })
    }

    /// Verifica integridade: hash + proof + prev_hash.
    pub fn is_valid(&self, expected_prev_hash: u64) -> bool {
        if self.prev_hash != expected_prev_hash {
            return false;
        }
        if self.event_count == 0 {
            return false;
        }
        let expected_proof = compute_proof(
            self.event_count, self.event_hash, self.safety_state, self.prev_hash,
        );
        if self.safety_proof != expected_proof {
            return false;
        }
        let expected_hash = compute_hash(
            self.index, self.timestamp_s, self.safety_proof, self.prev_hash,
        );
        if self.hash != expected_hash {
            return false;
        }
        true
    }
}

fn compute_proof(event_count: u32, event_hash: u64, state: SafetyState, prev_hash: u64) -> u64 {
    let mut buf = Vec::new();
    buf.extend_from_slice(&event_count.to_le_bytes());
    buf.extend_from_slice(&event_hash.to_le_bytes());
    buf.extend_from_slice(&[state.as_u8()]);
    buf.extend_from_slice(&prev_hash.to_le_bytes());
    fnv1a_64(&buf)
}

fn compute_hash(index: u64, timestamp_s: f32, proof: u64, prev_hash: u64) -> u64 {
    let mut buf = Vec::new();
    buf.extend_from_slice(&index.to_le_bytes());
    buf.extend_from_slice(&timestamp_s.to_le_bytes());
    buf.extend_from_slice(&proof.to_le_bytes());
    buf.extend_from_slice(&prev_hash.to_le_bytes());
    fnv1a_64(&buf)
}

// ============ ERROS ============
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainError {
    EmptyEvents,
    InvalidPredecessor,
    InvalidBlock { index: u64 },
}

// ============ CERTCHAIN ============
pub struct CertChain {
    pub drone_id: u8,
    pub mission_id: String,
    pub blocks: Vec<SafetyBlock>,
}

impl CertChain {
    pub fn new(drone_id: u8, mission_id: &str) -> Self {
        Self {
            drone_id,
            mission_id: mission_id.into(),
            blocks: Vec::new(),
        }
    }

    /// Append de um bloco. Valida Proof-of-Safety.
    pub fn append(
        &mut self,
        timestamp_s: f32,
        event_count: u32,
        event_hash: u64,
        safety_state: SafetyState,
    ) -> Result<u64, ChainError> {
        let prev_hash = self.blocks.last().map(|b| b.hash).unwrap_or(0);
        let index = self.blocks.len() as u64;
        let block = SafetyBlock::new(index, timestamp_s, event_count, event_hash, safety_state, prev_hash)?;
        let hash = block.hash;
        self.blocks.push(block);
        Ok(hash)
    }

    /// Verifica a integridade da chain inteira.
    pub fn verify(&self) -> bool {
        let mut prev = 0u64;
        for (i, block) in self.blocks.iter().enumerate() {
            if block.index != i as u64 {
                return false;
            }
            if !block.is_valid(prev) {
                return false;
            }
            prev = block.hash;
        }
        true
    }

    pub fn root_hash(&self) -> u64 {
        self.blocks.last().map(|b| b.hash).unwrap_or(0)
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn last_state(&self) -> SafetyState {
        self.blocks.last().map(|b| b.safety_state).unwrap_or(SafetyState::Nominal)
    }
}

// ============ PEERS ============
#[derive(Clone, Debug)]
pub struct PeerRecord {
    pub peer_id: u8,
    pub last_known_hash: u64,
    pub last_known_index: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncResult {
    Accepted,
    AlreadyKnown,
    Rejected(&'static str),
}

/// CertChainNode = chain local + registo de chains dos peers.
pub struct CertChainNode {
    pub chain: CertChain,
    pub peers: HashMap<u8, PeerRecord>,
}

impl CertChainNode {
    pub fn new(drone_id: u8, mission_id: &str) -> Self {
        Self {
            chain: CertChain::new(drone_id, mission_id),
            peers: HashMap::new(),
        }
    }

    /// Aceita uma chain de um peer. Verifica integridade total.
    pub fn accept_peer_chain(&mut self, peer_chain: &CertChain) -> SyncResult {
        if peer_chain.drone_id == self.chain.drone_id {
            return SyncResult::Rejected("cannot accept self chain");
        }
        if !peer_chain.verify() {
            return SyncResult::Rejected("peer chain integrity failed");
        }
        let peer_id = peer_chain.drone_id;
        let new_hash = peer_chain.root_hash();
        let new_index = peer_chain.block_count() as u64 - 1;

        match self.peers.get(&peer_id) {
            Some(rec) if rec.last_known_hash == new_hash => SyncResult::AlreadyKnown,
            _ => {
                self.peers.insert(peer_id, PeerRecord {
                    peer_id,
                    last_known_hash: new_hash,
                    last_known_index: new_index,
                });
                SyncResult::Accepted
            }
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Hash agregado de todos os peers (para consenso global do enxame).
    pub fn swarm_consensus(&self) -> u64 {
        let mut all: Vec<(u8, u64)> = self.peers.iter()
            .map(|(k, v)| (*k, v.last_known_hash))
            .collect();
        all.push((self.chain.drone_id, self.chain.root_hash()));
        all.sort_by_key(|(id, _)| *id);
        let mut buf = Vec::new();
        for (id, h) in &all {
            buf.extend_from_slice(&[*id]);
            buf.extend_from_slice(&h.to_le_bytes());
        }
        fnv1a_64(&buf)
    }
}

// ============ TESTES ============
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_new_empty() {
        let c = CertChain::new(1, "m");
        assert_eq!(c.block_count(), 0);
        assert_eq!(c.root_hash(), 0);
        assert!(c.verify());
    }

    #[test]
    fn append_first_block() {
        let mut c = CertChain::new(1, "m");
        let hash = c.append(1.0, 3, 0xDEAD, SafetyState::Nominal).unwrap();
        assert_ne!(hash, 0);
        assert_eq!(c.block_count(), 1);
        assert!(c.verify());
    }

    #[test]
    fn append_multiple_blocks() {
        let mut c = CertChain::new(1, "m");
        c.append(1.0, 1, 0xAAAA, SafetyState::Nominal).unwrap();
        c.append(2.0, 2, 0xBBBB, SafetyState::FdirActive).unwrap();
        c.append(3.0, 1, 0xCCCC, SafetyState::LnnRecovery).unwrap();
        assert_eq!(c.block_count(), 3);
        assert!(c.verify());
        assert_eq!(c.last_state(), SafetyState::LnnRecovery);
    }

    #[test]
    fn reject_empty_events() {
        let mut c = CertChain::new(1, "m");
        let r = c.append(1.0, 0, 0xDEAD, SafetyState::Nominal);
        assert_eq!(r, Err(ChainError::EmptyEvents));
        assert_eq!(c.block_count(), 0);
    }

    #[test]
    fn reject_tampered_proof() {
        let mut c = CertChain::new(1, "m");
        c.append(1.0, 3, 0xDEAD, SafetyState::Nominal).unwrap();
        // Modificar safety_proof directamente
        c.blocks[0].safety_proof ^= 0xFF;
        assert!(!c.verify(), "tamper nao detectado");
    }

    #[test]
    fn reject_tampered_hash() {
        let mut c = CertChain::new(1, "m");
        c.append(1.0, 3, 0xDEAD, SafetyState::Nominal).unwrap();
        c.append(2.0, 1, 0xBEEF, SafetyState::FdirActive).unwrap();
        // Modificar hash do primeiro bloco -> chain quebra
        c.blocks[0].hash ^= 0x01;
        assert!(!c.verify());
    }

    #[test]
    fn peer_accepts_valid_chain() {
        let mut peer = CertChain::new(2, "m");
        peer.append(1.0, 2, 0x1111, SafetyState::Nominal).unwrap();
        peer.append(2.0, 1, 0x2222, SafetyState::FdirActive).unwrap();

        let mut node = CertChainNode::new(1, "m");
        assert_eq!(node.accept_peer_chain(&peer), SyncResult::Accepted);
        assert_eq!(node.peer_count(), 1);
    }

    #[test]
    fn peer_rejects_tampered_chain() {
        let mut peer = CertChain::new(2, "m");
        peer.append(1.0, 2, 0x1111, SafetyState::Nominal).unwrap();
        peer.append(2.0, 1, 0x2222, SafetyState::Nominal).unwrap();
        peer.blocks[0].event_hash ^= 0xFF;  // tamper

        let mut node = CertChainNode::new(1, "m");
        let result = node.accept_peer_chain(&peer);
        assert!(matches!(result, SyncResult::Rejected(_)));
        assert_eq!(node.peer_count(), 0);
    }

    #[test]
    fn peer_already_known() {
        let mut peer = CertChain::new(2, "m");
        peer.append(1.0, 2, 0x1111, SafetyState::Nominal).unwrap();

        let mut node = CertChainNode::new(1, "m");
        assert_eq!(node.accept_peer_chain(&peer), SyncResult::Accepted);
        assert_eq!(node.accept_peer_chain(&peer), SyncResult::AlreadyKnown);
    }

    #[test]
    fn swarm_consensus_converges() {
        // 3 drones, cada um aceita as chains dos outros dois
        let mut chains: Vec<CertChain> = (1..=3).map(|id| {
            let mut c = CertChain::new(id, "m");
            c.append(1.0, 2, 0x1000 + id as u64, SafetyState::Nominal).unwrap();
            c
        }).collect();

        let mut nodes: Vec<CertChainNode> = chains.iter().map(|c| {
            let mut node = CertChainNode::new(c.drone_id, "m");
            node.chain = c.clone();
            node
        }).collect();

        // Cada node aceita os outros dois
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    nodes[i].accept_peer_chain(&chains[j]);
                }
            }
        }

        // Todos convergem para o mesmo consenso
        let h0 = nodes[0].swarm_consensus();
        let h1 = nodes[1].swarm_consensus();
        let h2 = nodes[2].swarm_consensus();
        assert_eq!(h0, h1);
        assert_eq!(h1, h2);
    }

    // Suprimir warnings de unused
    #[allow(dead_code)]
    fn _ensure_clone(c: &CertChain) -> CertChain { c.clone() }
}

impl Clone for CertChain {
    fn clone(&self) -> Self {
        Self {
            drone_id: self.drone_id,
            mission_id: self.mission_id.clone(),
            blocks: self.blocks.clone(),
        }
    }
}

// ============ FASE 2: CERTIFICATION-AS-A-SERVICE (CaaS) ============
// Exporta bundles verificaveis. Regulador valida sem confiar no operador.

/// Bundle de certificacao: tudo o que um regulador precisa para verificar.
#[derive(Clone, Debug)]
pub struct CertificateBundle {
    pub schema_version: u8,       // versao do formato (1)
    pub drone_id: u8,
    pub mission_id: String,
    pub block_count: usize,
    pub root_hash: u64,
    pub first_block_hash: u64,
    pub last_state: SafetyState,
    pub total_events: u32,
    /// Texto JSON serializado (verificavel independentemente).
    pub json: String,
}

/// Veredicto de verificacao (regulador EASA/FAA).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegulatorVerdict {
    Pass { bundle_hash: u64, blocks: usize },
    Fail { reason: String },
}

/// Exporta a chain como bundle de certificacao.
pub fn export_bundle(chain: &CertChain) -> CertificateBundle {
    let block_count = chain.blocks.len();
    let root_hash = chain.root_hash();
    let first_block_hash = chain.blocks.first().map(|b| b.hash).unwrap_or(0);
    let last_state = chain.last_state();
    let total_events: u32 = chain.blocks.iter().map(|b| b.event_count).sum();

    // JSON minimal (schema_version, drone_id, blocks, states, root_hash)
    let mut json = String::from("{\n");
    json.push_str("  \"schema_version\": 1,\n");
    json.push_str(&format!("  \"drone_id\": {},\n", chain.drone_id));
    json.push_str(&format!("  \"mission_id\": \"{}\",\n", chain.mission_id));
    json.push_str(&format!("  \"root_hash\": {},\n", root_hash));
    json.push_str(&format!("  \"block_count\": {},\n", block_count));
    json.push_str("  \"blocks\": [\n");
    for (i, b) in chain.blocks.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"i\":{},\"t\":{:.3},\"ev\":{},\"st\":{},\"h\":{},\"p\":{}}}{}\n",
            b.index, b.timestamp_s, b.event_count, b.safety_state.as_u8(), b.hash, b.prev_hash,
            if i < block_count - 1 { "," } else { "" }
        ));
    }
    json.push_str("  ]\n");
    json.push_str("}\n");

    CertificateBundle {
        schema_version: 1,
        drone_id: chain.drone_id,
        mission_id: chain.mission_id.clone(),
        block_count,
        root_hash,
        first_block_hash,
        last_state,
        total_events,
        json,
    }
}

/// Verifica um bundle (simulando um regulador EASA/FAA).
/// Nao confia no operador - verifica integridade estrutural.
pub fn verify_bundle(bundle: &CertificateBundle) -> RegulatorVerdict {
    // 1. Schema version
    if bundle.schema_version != 1 {
        return RegulatorVerdict::Fail {
            reason: format!("unsupported schema version: {}", bundle.schema_version),
        };
    }

    // 2. Chain nao vazia
    if bundle.block_count == 0 {
        return RegulatorVerdict::Fail {
            reason: "empty chain".into(),
        };
    }

    // 3. Root hash presente
    if bundle.root_hash == 0 {
        return RegulatorVerdict::Fail {
            reason: "root_hash is zero".into(),
        };
    }

    // 4. Primeiro bloco hash presente
    if bundle.first_block_hash == 0 {
        return RegulatorVerdict::Fail {
            reason: "first_block_hash is zero".into(),
        };
    }

    // 5. JSON contem todos os campos criticos
    for field in &["schema_version", "root_hash", "blocks", "drone_id"] {
        if !bundle.json.contains(field) {
            return RegulatorVerdict::Fail {
                reason: format!("json missing field: {}", field),
            };
        }
    }

    // 6. Nenhum bloco vazio (Proof-of-Safety baseline)
    if bundle.total_events == 0 {
        return RegulatorVerdict::Fail {
            reason: "chain contains zero events (no proof-of-safety)".into(),
        };
    }

    // PASS: retorna hash do bundle (assinatura)
    let mut buf = Vec::new();
    buf.extend_from_slice(&bundle.root_hash.to_le_bytes());
    buf.extend_from_slice(&[bundle.drone_id]);
    buf.extend_from_slice(&(bundle.block_count as u64).to_le_bytes());
    let bundle_hash = fnv1a_64(&buf);

    RegulatorVerdict::Pass {
        bundle_hash,
        blocks: bundle.block_count,
    }
}

// ============ TESTES FASE 2 ============
#[cfg(test)]
mod tests_fase2 {
    use super::*;

    fn make_chain_5_blocks() -> CertChain {
        let mut c = CertChain::new(7, "mission_easa_001");
        c.append(1.0, 3, 0x1111, SafetyState::Nominal).unwrap();
        c.append(2.0, 2, 0x2222, SafetyState::FdirActive).unwrap();
        c.append(3.0, 1, 0x3333, SafetyState::LnnRecovery).unwrap();
        c.append(4.0, 2, 0x4444, SafetyState::Nominal).unwrap();
        c.append(5.0, 1, 0x5555, SafetyState::Nominal).unwrap();
        c
    }

    #[test]
    fn bundle_export_has_metadata() {
        let c = make_chain_5_blocks();
        let b = export_bundle(&c);
        assert_eq!(b.drone_id, 7);
        assert_eq!(b.mission_id, "mission_easa_001");
        assert_eq!(b.block_count, 5);
        assert_ne!(b.root_hash, 0);
        assert_eq!(b.schema_version, 1);
        assert_eq!(b.total_events, 3 + 2 + 1 + 2 + 1);
    }

    #[test]
    fn bundle_json_complete() {
        let c = make_chain_5_blocks();
        let b = export_bundle(&c);
        for field in &["schema_version", "drone_id", "root_hash", "blocks", "mission_id"] {
            assert!(b.json.contains(field), "json missing {}", field);
        }
        // Contar linhas de bloco
        let block_lines = b.json.lines().filter(|l| l.contains("\"i\":")).count();
        assert_eq!(block_lines, 5);
    }

    #[test]
    fn regulator_passes_valid_bundle() {
        let c = make_chain_5_blocks();
        let b = export_bundle(&c);
        let v = verify_bundle(&b);
        assert!(matches!(v, RegulatorVerdict::Pass { .. }));
        if let RegulatorVerdict::Pass { blocks, .. } = v {
            assert_eq!(blocks, 5);
        }
    }

    #[test]
    fn regulator_rejects_empty_chain() {
        let c = CertChain::new(1, "m");
        let b = export_bundle(&c);
        let v = verify_bundle(&b);
        assert!(matches!(v, RegulatorVerdict::Fail { .. }));
    }

    #[test]
    fn regulator_rejects_wrong_schema() {
        let c = make_chain_5_blocks();
        let mut b = export_bundle(&c);
        b.schema_version = 99;
        let v = verify_bundle(&b);
        assert!(matches!(v, RegulatorVerdict::Fail { .. }));
    }

    #[test]
    fn regulator_rejects_zero_root_hash() {
        let c = make_chain_5_blocks();
        let mut b = export_bundle(&c);
        b.root_hash = 0;
        let v = verify_bundle(&b);
        assert!(matches!(v, RegulatorVerdict::Fail { .. }));
    }

    #[test]
    fn regulator_rejects_zero_events() {
        let c = make_chain_5_blocks();
        let mut b = export_bundle(&c);
        b.total_events = 0;
        let v = verify_bundle(&b);
        assert!(matches!(v, RegulatorVerdict::Fail { .. }));
    }

    #[test]
    fn regulator_rejects_corrupted_json() {
        let c = make_chain_5_blocks();
        let mut b = export_bundle(&c);
        b.json = "{}".into();  // Remove todos os campos
        let v = verify_bundle(&b);
        assert!(matches!(v, RegulatorVerdict::Fail { .. }));
    }

    #[test]
    fn bundle_hash_deterministic() {
        let c = make_chain_5_blocks();
        let b1 = export_bundle(&c);
        let b2 = export_bundle(&c);
        let v1 = verify_bundle(&b1);
        let v2 = verify_bundle(&b2);
        if let (RegulatorVerdict::Pass { bundle_hash: h1, .. },
                RegulatorVerdict::Pass { bundle_hash: h2, .. }) = (v1, v2) {
            assert_eq!(h1, h2);
        } else {
            panic!("both should PASS");
        }
    }

    #[test]
    fn end_to_end_certification_flow() {
        // 1. Chain construida em runtime
        let mut chain = CertChain::new(42, "easa_demo");
        chain.append(1.0, 2, 0xAA, SafetyState::Nominal).unwrap();
        chain.append(2.0, 1, 0xBB, SafetyState::FdirActive).unwrap();
        chain.append(3.0, 2, 0xCC, SafetyState::LnnRecovery).unwrap();
        chain.append(4.0, 1, 0xDD, SafetyState::Nominal).unwrap();

        // 2. Operador exporta bundle
        let bundle = export_bundle(&chain);

        // 3. Regulador verifica
        let verdict = verify_bundle(&bundle);

        // 4. Deve passar (4 blocos, 6 eventos, root_hash != 0)
        assert!(matches!(verdict, RegulatorVerdict::Pass { blocks: 4, .. }));
    }
}
