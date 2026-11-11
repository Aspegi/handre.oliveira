# RNFC Safety Analysis

## Invariantes de Seguranca (o que NUNCA acontece)

| ID | Invariante | Como e garantido | Teste |
|---|---|---|---|
| S1 | `FusionMag::yaw` esta sempre em [-pi, pi] | Wrap apos cada update | `yaw_always_bounded_after_update` |
| S2 | `FusionMag::yaw` nunca e NaN | Guard `is_finite` nas entradas | `yaw_always_bounded_after_update` |
| S3 | `Guidance::arrived` so dispara no waypoint final | `current_index == len-1` check | `arrived_only_at_final_waypoint` |
| S4 | `Guidance::pitch` nunca excede `max_pitch` | `.min(max_pitch)` | `pitch_bounded_by_max_pitch` |
| S5 | `PID::update` retorna output finito para inputs finitos | Clamp + sem divisao por zero | `no_nan_extreme_inputs` |
| S6 | Kalman nao diverge em 1000 iteracoes | P matrix stable | `no_nan_after_many_steps` |
| S7 | Failsafe geofence dispara RTL | Prioridade P1 | `failsafe_geofence_triggers_rtl` |
| S8 | Failsafe bateria critica -> emergency land | Prioridade P2 | `failsafe_critical_battery_emergency_land` |
| S9 | Sensor stuck -> Hover (nao aborta) | SensorHealth stuck_count >= threshold | `stuck_sensor_detected` |
| S10 | Spoofing abrupto -> RTL | SpoofingDetector err persistente | `abrupt_spoofing_detected` |

## Limites Conhecidos (documentados)

| ID | Limitacao | Impacto | Mitigacao |
|---|---|---|---|
| L1 | **2-of-3 pode ser enganado se 2 sensores spoofed para MESMO valor** | Voter escolhe maioria errada | SpoofingDetector (gyro-based) deteta a divergencia e dispara RTL |
| L2 | SpoofingDetector usa `yaw` true no SITL (ground truth) | Nao reproduzivel em hardware | Em HW real, usar EKF yaw (bias-free) |
| L3 | Battery drain 0.0084 V/s fixo no SITL | Modelo simplificado | Testado com MC 0.005-0.012 V/s |
| L4 | Threshold SpoofingDetector 0.35 rad (20 graus) | Ataques < 20 graus nao detetados | Trade-off: falsos positivos vs sensibilidade |
| L5 | 2-of-3 nao faz temporal analysis | Spoofing coordenado de 2 sensores | Adicionar correlacao temporal em v2.0 |

## Defesa em Camadas (defense in depth)

```
Layer 1: SensorHealth  -> stuck/outlier (single sensor)
Layer 2: SensorVoter   -> 2-of-3 majority voting (isolates 1 bad)
Layer 3: SpoofingDetector -> mag vs gyro divergence (catches 2 bad)
Layer 4: Failsafe      -> geofence/battery/sensor -> RTL/Hover/Land
```

Se qualquer camada falha, a proxima deteta. Prova: teste `layered_defense_2_of_3_compromised` demonstra que Layer 2 falha + Layer 3 salva.

## Cobertura de Testes

| Categoria | Testes | Ficheiro |
|---|---|---|
| Unit core | 23 | rnfc-core |
| FDIR (single sensor + layered) | 19 | rnfc-fdir |
| Chaos (SITL) | 5 | chaos.rs |
| Integration (missao) | 1 | mission.rs |
| Monte Carlo | 4 (100+1000) | mc_suite.rs |
| **Total** | **52** | — |

## Metricas de Robustez

- **Monte Carlo (1000 runs)**: 100% success rate, 0 timeouts, 0 crashes
- **Dist max (1000 runs)**: 2.00m
- **Dist avg (1000 runs)**: 1.97m
- **Precisao ideal (sem adversidade)**: 0.48m

## Certificacao — Proximos Passos

- [ ] Kani proofs exaustivos (todos os inputs)
- [ ] HAL para hardware real (Pixhawk/STM32)
- [ ] EKF 15-state (atitude + bias no filtro)
- [ ] DO-178C DAL A traceability
## Deployment Architecture (Decisao de Design)

| Componente | Onde corre | Target | Razao |
|---|---|---|---|
| **rnfc-fdir** | **Drone (Pixhawk/STM32)** | **thumbv7em-none-eabihf** | Deteta falhas mesmo se host falha |
| rnfc-abstractions | Drone | thumbv7em-none-eabihf | Traits sem std |
| rnfc-core (Kalman/PID/Guidance) | **Companion computer** | x86_64 / ARM64 | nalgebra nao e totalmente no_std (matrixmultiply) |
| rnfc-daa (A*) | **Ground station** | x86_64 | Planeamento pesado, envia waypoints via MAVLink |
| rnfc-mavlink | Ambos | — | Interface de comunicacao |

### Porque esta arquitetura e valida

**Padrao da industria:** drones reais (ArduPilot, PX4) separam:
- **Flight controller** (MCU Cortex-M) — controlo basico, FDIR
- **Companion computer** (Linux ARM) — Kalman pesado, visao, A*

O RNFC segue este padrao: o FDIR corre no MCU (seguranca critica), o Kalman pode correr num companion ou ser substituido por um INS dedicado no hardware real.

### Limitacao documentada

**L6: rnfc-core nao compila para Cortex-M4F** por dependencia em `nalgebra` (matrixmultiply nao e no_std para matrizes >4x4).

**Mitigacao em hardware real:**
- Opcao A: correr o Kalman num companion computer (padrao ArduPilot)
- Opcao B: substituir por INS dedicado (hardware sensor fusion)
- Opcao C: reimplementar Kalman com arrays fixos (sem nalgebra) — trabalho de v2.0

## Limites Adicionais (actualizado v1.6.1)

| ID | Limitacao | Mitigacao |
|---|---|---|
| L7 | Accel convention: `az=-9.81` em nivel (specific force NED). Roll usa `atan2(-ay,-az)`. | Testes `rolled_right_detects_roll`, `pitched_forward_detects_pitch` blindam a convencao |
| L8 | AttitudeEstimator testado com ruido sintetico (accel 0.5, gyro 0.01+0.02 sin). Sensores reais variam. | Calibrar alpha (0.98) ao hardware. Property tests + fuzz blindam a matematica |

## Regressao - Bugs Historicos Blindados

| Bug | Versao | Teste |
|---|---|---|
| deadband_radius (drone a 15.24m) | v0.4.0 | `bug_01_no_deadband_radius` |
| direct_mode 8.0 (arrival prematuro) | v0.4.0 | `bug_02_no_direct_mode_8m` |
| min_speed 3.0 (overshoot 25m) | v0.4.0 | `bug_03_no_min_speed_3_perto_goal` |
| powi em no_std | v1.3.0 | `bug_04_no_powi_in_fdir` |
| sqrt sem libm | v1.3.0 | `bug_05_no_raw_sqrt_in_fdir` |
| plan_astar loop sem guard | v0.9.7 | `bug_06_no_unbounded_reroute_loop` |
| use antes de inner attrs | v1.3.0 | `bug_07_use_apos_inner_attrs` |
| derives duplicados | v1.4.0 | `bug_08_no_duplicate_derives` |
| clippy::map_or invalido | v1.8 | `bug_09_no_invalid_clippy_lint` |

## L9 — Zombie Orbit (Adversarial MC)

**Descoberto:** Adversarial Monte Carlo (5000 cenarios evoluidos, 50 geracoes).

**Cenario pior (gen 15):**
- wind (5.16, 0.0) m/s
- attack +16.3 graus @ t=193.8s
- GNSS dropout 10s @ t=136.3s
- noise 2.12x
- OUTCOME: Timeout a 505m

**Modo de falha:** drone orbita a ~500m da base, sem crashar nem fugir. FDIR previne crash, mas nao deteta "estagnacao".

**Fitness plateau:** ~605 (100 + 505m). Nao crasha -> nao atinge 10000. Nao foge -> nao atinge 2000+.

**Mitigacao proposta (v1.7.1):** ZombieDetector - se dist > 50m AND tempo sem progresso > 60s -> RTL.

## L9 — Zombie Orbit (RESOLVIDO em v1.7.1)

**Descoberto:** Adversarial MC v1.7.0 (worst dist = 505m, fitness = 605).

**Resolucao:** ZombieDetector com latch permanente:
- Trigger: dist > 50m AND no progress > 60s
- Acao: latch RTL permanente (nao oscila)
- Integrado no mc_adversarial e no main.rs

**Prova (Adversarial MC v1.7.1):**
- Worst dist: 505.39m -> 2.00m (-99.6%)
- Fitness: 605.39 -> 2.00 (-99.7%)
- OUTCOME: Timeout -> RtlSuccess
- 5000 cenarios evoluidos, zero crashes, zero timeouts

**Estado:** L9 eliminado. Envelope final:
  - Normal: 1.98m precisao, 254s
  - Adversarial (5000 evolved): pior caso 2.00m
  - Zombie orbit: impossivel (ZombieDetector previne)

## L1 — Cross-Layer Byzantine Detection (IMPLEMENTADO, NAO INTEGRADO)

**Crate:** rnfc-fdir (CrossLayerDetector + 6 testes)

**Funcionalidade:** Deteta ataques coordenados (IMU vs GNSS vs Mag)
- `Coherent`: tudo consistente
- `Inconsistent`: 1 camada diverge (recuperavel)
- `Attacked`: 2+ camadas divergem (ataque confirmado)

**Bloqueio de integracao:** O modelo SITL atual usa `true_pos = kalman_pos` (mesma variavel).
Quando GNSS e spoofed, Kalman pos acumula +spoof por iteracao -> runaway (16M km observado).
Para testar adversarialmente, requer refactor: separar `true_pos` (fisica) de `kalman_pos` (estimativa).

**Estado:** Codigo do detector em producao (rnfc-fdir), testado em isolamento.
Integracao no SITL pendente de refactor do modelo de simulacao (v2.0).

## L1 — Cross-Layer Byzantine Detection (REMOVIDO em v1.7.3)

**Tentado em v1.7.2:** implementado em rnfc-fdir (6 testes, compila Cortex-M4F).

**Resultado:** causou regressao no ZombieDetector (mc_adversarial passou de 2.00m a 16M km).
Causa exacta nao identificada (possivel bug no script de append ou interaccao libm).

**Decisao:** CrossLayerDetector removido de rnfc-fdir. A tentativa fica no historico git (953687d).

**Estado actual:** L9 (ZombieDetector) fechado, L1 nao implementado.
Envelope de performance: pior caso 2.00m em 5000 cenarios adversarialmente evoluidos.
