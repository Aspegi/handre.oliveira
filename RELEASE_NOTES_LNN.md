# RNFC v3.0.0-lnn-ftc — LNN Fault-Tolerant Control

## BREAKTHROUGH: Motor Loss Recovery via Two-Phase LNN

### O Problema
Quando um motor de um quadrotor falha, o PID baseline crasha em <2s.
A fisica e clara: 3 motores nao conseguem produzir roll torque simetrico.
Nenhum tuning de PID resolve - e limitacao fisica.

### A Solucao (Two-Phase LNN)

Fase 1 - LNN_healthy (16 hidden):
- Treinada com motor OK
- Aprende hover estavel (fitness 982)
- Roll: -2.2 graus, altitude: 9.66m

Fase 2 - LNN_recovery (16 hidden):
- Treinada com motor 1 morto em t=0
- Aprende descida controlada (fitness 972)
- Landed em 17.73s, roll 0.7 graus

Switch: por m1_alive (>0.5 = healthy, <0.5 = recovery)

### Comparacao

| Config | Resultado |
|---|---|
| **LNN Two-Phase** | **Hover + Recovery (ambos)** |
| LNN single 8h | Recovery so |
| LNN single 16h | Hover so |
| LNN single 32h | Nenhum (evolucao falhou) |
| Dual-LNN sum | Hover so (cheat) |
| Dual-LNN min | Nenhum (gradiente morto) |
| **PID baseline** | **Crash 1.5s** |

### Implementacao

- Crate: rnfc-lnn (~180 linhas, no_std)
- LiquidNetwork<IN, HID, OUT> via const generics
- ODE Euler solver + tau aprendidas
- 804 params total (2 x 402)
- Compativel com Cortex-M4F

### Reproducibility

cargo test -p rnfc-backend-sitl --test motor_loss_2phase --release -- --nocapture

### Roadmap v3.1

- [ ] Adversarial MC com LNN-FTC (motor loss como ataque)
- [ ] Compare LNN vs PID em 1000 cenarios
- [ ] Deploy em Cortex-M4F real
- [ ] Paper submission (IEEE Aerospace 2027)