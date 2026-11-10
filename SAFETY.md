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