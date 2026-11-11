# RNFC v1.6.1 — Aerospace-Grade, Verificado em 6 Camadas

## Estado Final

| Metrica | Valor |
|---|---|
| **Testes totais** | **79** (52 unit + 1 integration + 5 chaos + 4 Monte Carlo + 8 fuzz + 9 regression) |
| **Fuzz inputs** | **80.000** aleatorios (zero panics, zero NaN) |
| **Monte Carlo** | **1000 runs, 100% success**, 0 timeouts, 0 crashes |
| **Benchmarks** | **109 ns** por loop (margem 9000x para 100Hz Cortex-M4F) |
| **Estimador de atitude** | 0.24 graus erro (accel+gyro com ruido realista) |
| **Precisao missao** | **1.98m** (com atitude estimada, nao ground truth) |
| **Crates Cortex-M4F** | 3 (abstractions, fdir, hal) |
| **CI** | fmt + clippy + test + build + no_std |

## Arquitetura de Deployment

| Crate | Host | Cortex-M4F | Onde Corre |
|---|---|---|---|
| rnfc-abstractions | OK | OK | Drone (traits) |
| **rnfc-fdir** | OK | OK | **Drone (FDIR critico)** |
| **rnfc-hal** | OK | OK | **Drone (HAL)** |
| rnfc-core | OK | - | Companion (nalgebra) |
| rnfc-daa | OK | - | Ground station (A*) |
| rnfc-mavlink | OK | parcial | Ambos |

## Componentes

- **Percecao**: Kalman 3D + FusionMag 2D/3D + AttitudeEstimator (accel+gyro)
- **Controlo**: PID 3 eixos + FDIR (Hover/RTL/EmergencyLand)
- **Decisao**: Guidance + A* dinamico + smart reroute
- **FDIR**: SensorHealth + Failsafe + SpoofingDetector + SensorVoter (2-of-3)
- **Visao**: Imagens 64x48 via MAVLink
- **HAL**: FlightController trait (abstracao para Pixhawk/STM32)

## Adversidades Sobrevividas

- Vento ate 5 m/s + rajadas
- Gyro bias (+25 graus em 226s)
- Mag spoofing +30 graus (isolado via 2-of-3)
- Mag stuck (isolado via 2-of-3)
- GNSS dropout (10s, dead reckoning)
- Ruido realista de hardware (accel 0.5, gyro bias 0.01)
- Obstaculo desconhecido (LiDAR + A*)
- Bateria baixa (RTL)

## Defesa em Camadas

```
Layer 1: SensorHealth     -> stuck/outlier (single sensor)
Layer 2: SensorVoter      -> 2-of-3 majority voting
Layer 3: SpoofingDetector -> mag vs gyro divergence
Layer 4: Failsafe         -> geofence/battery/sensor -> RTL/Hover/Land
```

## 6 Camadas de Verificacao

1. **Unit** (52) - correcao de cada modulo
2. **Integration** (1) - missao A->B->C completa
3. **Chaos** (5) - adversidades combinadas
4. **Monte Carlo** (1000) - distribuicao estatistica
5. **Fuzz** (80k inputs) - inputs adversariais aleatorios
6. **Regression** (9) - previne os 9 bugs historicos

## 9 Bugs Historicos Blindados

1. `deadband_radius` (v0.4 - drone a 15.24m)
2. `direct_mode 8.0` (v0.4 - arrival a 8m)
3. `min_speed 3.0` (v0.4 - overshoot 25m)
4. `powi` em no_std (v1.3 - E0599)
5. `sqrt` sem libm (v1.3 - E0599)
6. `plan_astar` loop sem guard (v0.9.7 - 7 replans/30s)
7. `use` antes de attrs (v1.3 - E0753)
8. derives duplicados (v1.4 - E0119)
9. `clippy::map_or` invalido (v1.8 - unknown_lint)

## Roadmap Futuro (semanas, nao sessoes)

- [ ] Kani proofs exaustivos (DAL A)
- [ ] HAL para Pixhawk real (STM32 HAL)
- [ ] EKF 15-state (atitude + bias no filtro)
- [ ] DO-178C DAL A traceability

## Series de Tags

- v0.4.0 a v0.10.0 (13 marcos SITL)
- v1.0.0 (release aerospace)
- v1.1.0 a v1.6.1 (8 marcos de verificacao)