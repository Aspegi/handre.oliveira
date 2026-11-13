# Hacker News Post — Pronto a Colar

## Título (72 caracteres, máx 80)

```
Show HN: A no_std Rust flight controller with 277 tests and DO-326A
```

## URL

```
https://github.com/Aspegi/handre.oliveira
```

## Texto (primeiro comentário, cola após submeter)

```
Hi HN — I've been building this as a portfolio/research project for the
last few months. RNFC is a simulated flight-control stack for multirotor
UAVs in Rust.

What's actually there:

- 277 tests across 19 crates (unit, integration, chaos, fuzz, regression,
  research)
- Layered FDIR: 5 layers for sensor faults (health, 2-of-3 voting,
  spoofing detection, zombie detector, failsafe)
- LNN-FTC for motor-loss recovery: 804 parameters, runs on Cortex-M4F,
  PID baseline crashes in 1.5s, LNN lands at 17.73s with roll error < 1°
- 6-DOF physics (rotor-rs, quaternion dynamics), single waypoint at 4cm
  error with a cascaded PID
- MAVLink signing (4/4 attacks rejected: tampered, wrong key, replay,
  future timestamp)
- Hash-chain safety ledger with external anchoring (3/3 attacks rejected:
  rewrite, truncation, wrong key)
- DO-326A compliance package: PSecAC draft, 12-threat model, 13-requirement
  traceability matrix (12 complete, 1 partial)
- 2 papers in progress: SCC (safety case compiler) and CSR (counterfactual
  safety reasoning)

What it is NOT (the README says this honestly):

- Not flight-tested on real hardware
- Multi-waypoint 6-DOF failed 9 times — documented in SITL_6DOF.md
- "Quantum navigation" is a synthetic 2D anomaly map, not real SERF
- CertChain uses FNV-1a, not full SHA-256 (proof construction uses SHA-256)
- Secure Boot and Pixhawk HAL are skeletons awaiting hardware

I'm not claiming production-ready. I'm looking for feedback on:

1. The DO-326A approach — is the 13-requirement matrix useful, or
   over-engineered for a simulation?
2. The multi-waypoint 6-DOF failure — anyone solved this with a linear PID,
   or is geometric SE(3) the only way?
3. The papers — are SCC and CSR novel enough for SAFECOMP/DASC, or
   incremental?

Happy to answer questions about any of the layers.
```

## Quando submeter

| Dia | Hora Portugal | Hora Pacific |
|---|---|---|
| Terça-Quinta | 16h-18h | 8h-10h |
| Evitar | Fins de semana | — |

## O Que Esperar (realisticamente)

- HN front page: 200-500 estrelas, 20-50 comentários
- HN sem front page: 20-50 estrelas, 2-10 comentários
- Feedback técnico quase sempre

## Se receberes comentários negativos

- "It's all simulation" → Concorda. O README diz isso.
- "Quantum is fake" → Concorda. É um mapa sintético.
- "Blockchain isn't blockchain" → Concorda. É FNV-1a. O paper CSR diz isso.
- "Multi-waypoint doesn't work" → Concorda. 9 tentativas documentadas.

A honestidade é a tua defesa. Não exageres.
