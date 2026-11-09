\# 🚁 RNFC — Resilient Navigation \& Flight Control



Sistema de navegação e controlo de voo para aeronaves não tripuladas, escrito \*\*100% em Rust\*\*, projetado para operar em ambientes de \*\*GNSS negado\*\* e com \*\*verificação formal\*\* (Kani) do núcleo de navegação.



\## Visão Geral



O RNFC é um piloto automático completo, com:

\- \*\*Navegação\*\* por Kalman 3D (NED) + atitude (roll/pitch/yaw) + fusão gyro/acelerómetro

\- \*\*Controlo\*\* PID de atitude

\- \*\*Planeamento\*\* A\* com LiDAR + occupancy grid

\- \*\*Seguimento\*\* Pure Pursuit + cross-track + spline Catmull-Rom

\- \*\*Missão\*\* multi-waypoint com DAA dinâmico (replan em tempo real)

\- \*\*Verificação formal\*\* com Kani (406 checks SUCCESSFUL)

\- \*\*Comunicação\*\* MAVLink v2 (padrão da indústria)

\- \*\*Visualização\*\* HTML interativa com Plotly



\## Objetivo



Fornecer um \*\*componente certificável\*\* (caminho DO-178C DAL A) para navegação resiliente — a capacidade de continuar a missão quando o GNSS é negado, bloqueado ou falsificado.



\## Arquitetura





PLANEJADOR (A\* sobre occupancy grid) -> rnfc-daa::plan\_astar

|

TRAJETÓRIA (spline + cross-track) -> rnfc-daa::trajectory

|

GUIAMENTO (Pure Pursuit + look-ahead) -> rnfc-core::guidance

|

CONTROLO (PID roll/pitch/yaw) -> rnfc-core::attitude\_controller

|

NAVEGAÇÃO (Kalman 3D + atitude) -> rnfc-core::{kalman, attitude, rotation}

|

COMUNICAÇÃO (MAVLink v2 via UDP) -> rnfc-mavlink::MavlinkSender



text



\## Crates



| Crate | Responsabilidade |

| :--- | :--- |

| rnfc-abstractions | Traits Imu, Gnss, Lidar |

| rnfc-core | Núcleo no\_std: Kalman, atitude, PID, guiamento |

| rnfc-daa | Detect and Avoid: LiDAR sim, occupancy grid, A\*, spline |

| rnfc-mavlink | Encoder MAVLink v2 (HEARTBEAT + ATTITUDE) |

| rnfc-backend-sitl | Software-in-the-Loop (física simulada) |



\## Como Executar



\### Pré-requisitos

\- Rust estável (via rustup)

\- Windows 11 + MSVC Build Tools

\- WSL2/Ubuntu (apenas para Kani)



\### Correr a missão



```powershell

cd C:\\dev\\rnfc

cargo run -p rnfc-backend-sitl --release


---

## Autor e Propriedade

**André Oliveira** — autor e proprietário do projeto

- **Email:** handre.oliveira@gmail.com
- **Copyright:** (c) 2027 André Oliveira — Todos os direitos reservados
- **Licença:** Proprietária (ver ficheiro [LICENSE](./LICENSE))

Este software e o seu código-fonte são propriedade exclusiva de André Oliveira. Nenhuma parte pode ser reproduzida, distribuída ou modificada sem autorização prévia e por escrito.


---

## Segurança

### O que esta protegido (v1.9)

| Camada | Defesa | Implementacao |
| :--- | :--- | :--- |
| **Codigo** | Seguranca de memoria | Rust `no_std`, sem alocacao, `#![deny(unsafe_code)]` |
| **Codigo** | Ausencia de panic | Kani: 406 checks SUCCESSFUL no Kalman3D |
| **Codigo** | Invariantes matematicas | Covariancia positiva e finita provada |
| **Navegacao** | GNSS negado | Kalman 3D + IMU + atitude (continua sem GPS) |
| **Comunicacao** | Autenticacao | MAVLink v2 signing (SHA-256, 13 bytes/frame) |
| **Comunicacao** | Anti-replay | Timestamp monotonico persistente entre sessoes |
| **Comunicacao** | Integridade | Checksum CRC-16/MCRF4XX + assinatura SHA-256 |
| **Comunicacao** | Deteccao de intrusao | `MavlinkReceiver` com 6 regras IDS (flooding, sysid desconhecido, replay, gap, assinatura invalida, bruteforce) |
| **Robustez** | Fisica realista | Vento + arrasto + rajada no SITL |
| **Robustez** | Consciencia de limites | Logica de abortar quando `stuck > 10s` |

### O que ainda NAO esta protegido

| Ameaca | Estado | Mitigacao recomendada |
| :--- | :--- | :--- |
| **Confidencialidade** | Pendente | VPN/IPsec ou radio com AES-256 (MAVLink nao encripta) |
| **Secure boot** | Pendente | Bootloader com chave publica (verifica assinatura do binario) |
| **Encriptacao de logs** | Pendente | AES-256-GCM em repouso (`trajetoria.csv`) |
| **Rotacao de chaves** | Pendente | Protocolo de rotacao periodica (manual via `SETUP_SIGNING`) |
| **Provisionamento** | Parcial | SITL usa chave de teste; producao deve usar `SETUP_SIGNING` via USB |

### Caminho para certificacao de seguranca (DO-326A)

O RNFC tem os **5 artefactos DO-326A completos** em `docs/`:

| Requisito DO-326A | Ficheiro | Estado |
| :--- | :--- | :--- |
| Analise de ameacas (threat model) | `docs/RNFC-Threat-Model.md` | Concluido |
| Avaliacao de risco de seguranca | `docs/RNFC-Security-Risk-Assessment.md` | Concluido |
| Requisitos de seguranca | `docs/RNFC-Security-Requirements.md` | Concluido (14 requisitos) |
| Evidencia de mitigacao | `docs/RNFC-Security-Evidence.md` | Concluido (6 implementados) |
| Monitorizacao em operacao | `docs/RNFC-IDS-Design.md` + `MavlinkReceiver` | Concluido (design + codigo) |

**Riscos mitigados (Baixo):** T-01 (injecao), T-02 (replay), T-06 (flooding via IDS), T-08 (logs).

**Riscos a mitigar (Alto):** T-04 (provisionamento de chave), T-05 (secure boot).

**Riscos a monitorizar (Medio):** T-03 (spoofing GNSS), T-07 (confidencialidade).

### Testes de robustez

| Teste | Resultado |
| :--- | :--- |
| Vento normal (3 m/s) | Missao completa em ~338s, 1.23m de precisao |
| Vento extremo (8 m/s) | **ABORTAR** dispara em 10s, drone para a 18.55m |
| Rajada (5 m/s) | PID compensa, missao continua |
| MAVLink signing | 662 msg/sessao, 0 erros, anti-replay (3310 -> 3972) |
| IDS | 4 ataques testados (chave errada, sysid desconhecido, replay, flooding) |

### O que esta protegido

| Camada | Defesa | Implementacao |
| :--- | :--- | :--- |
| **Codigo** | Seguranca de memoria | Rust `no_std`, sem alocacao, `#![deny(unsafe_code)]` |
| **Codigo** | Ausencia de panic | Kani: 406 checks SUCCESSFUL no Kalman3D |
| **Codigo** | Invariantes matematicas | Covariancia positiva e finita provada |
| **Navegacao** | GNSS negado | Kalman 3D + IMU + atitude (continua sem GPS) |
| **Comunicacao** | Autenticacao | MAVLink v2 signing (SHA-256, 13 bytes/frame) |
| **Comunicacao** | Anti-replay | Timestamp monotónico persistente entre sessoes |
| **Comunicacao** | Integridade | Checksum CRC-16/MCRF4XX + assinatura SHA-256 |

### Caminho para certificacao de seguranca (DO-326A)

O RNFC esta pronto para o **DO-178C** (seguranca funcional: `no_std`, Kani, sem `unsafe`), mas ainda nao tem os artefactos do **DO-326A** (seguranca de aeronavegabilidade):

| Requisito DO-326A | Estado |
| :--- | :--- |
| Analise de ameacas (threat model) | Pendente |
| Avaliacao de risco de seguranca | Pendente |
| Requisitos de seguranca | Pendente |
| Evidencia de mitigacao | Parcial (signing implementado) |
| Monitorizacao em operacao | Pendente |

**Nota:** para operacoes militares ou civis certificadas, o hardening da comunicacao (encriptacao, secure boot) e a documentacao DO-326A sao pre-requisitos.

### Teste do signing

O signing foi validado com um verificador independente (PowerShell, `System.Security.Cryptography.SHA256`):

- **662 mensagens por sessao** (601 ATTITUDE + 61 HEARTBEAT)
- **Todas assinadas** (bit `incompat_flags & 0x01` ativo)
- **Todas validas** (SHA-256 calculado em Rust == SHA-256 calculado em PowerShell)
- **Anti-replay:** timestamp 1986 -> 2648 entre sessoes (nunca recomeca)


### Testes do IDS (6/6 regras validadas)

O `MavlinkReceiver` foi testado com **2 ficheiros de teste** que simulam 7 ataques:

| Ficheiro | Teste | Regra IDS | Resultado |
| :--- | :--- | :--- | :--- |
| `ids_test.rs` | 5 pacotes legítimos | — | 5 válidos, 0 alertas |
| `ids_test.rs` | 3 com chave errada | IDS-RULE-002, 005 | 3 inválidos, 6 alertas (INVALID_SIGNATURE + UNKNOWN_SYSID) |
| `ids_test.rs` | 2 legítimos (ts continua) | IDS-RULE-003 | 7 válidos (sem replay) |
| `ids_test.rs` | 1 sysid desconhecido (42) | IDS-RULE-002 | 1 alerta (UNKNOWN_SYSID) |
| `ids_test2.rs` | 60 pacotes rápidos | IDS-RULE-001 | 10 alertas (FLOODING) |
| `ids_test2.rs` | Timestamp +200 | IDS-RULE-004 | 1 alerta (TIMESTAMP_GAP) |
| `ids_test2.rs` | 12 chaves erradas | IDS-RULE-006 | 3 alertas (BRUTEFORCE) |

**Resultado final:** 6/6 regras IDS validadas. Os testes correm com:

```powershell
cargo run -p rnfc-mavlink --bin ids_test
cargo run -p rnfc-mavlink --bin ids_test2


### Testes de robustez a vento (matriz de 6 cenários)

O RNFC foi testado com **6 valores de vento** (0 a 12 m/s) para validar a robustez do controlo:

| Vento (m/s) | Tempo | Precisão | Comportamento |
| :--- | :--- | :--- | :--- |
| **0** (sem vento) | 82.71s | **3.98m** | Missão completa |
| **2** (leve) | 91.14s | **3.99m** | Missão completa |
| **4** (moderado) | 93.82s | **4.00m** | Missão completa |
| **6** (forte) | 253.03s | **3.99m** | Missão completa (lento) |
| **8** (muito forte) | ~180s | ~152m | **HOVER em B** (não deriva) |
| **12** (extremo) | ~180s | ~164m | **HOVER em B** (não deriva) |

**Precisão de 4m em todos os cenários alcançáveis** (0-6 m/s).

### Capacidades de adaptação ao vento

O sistema implementa **3 estratégias de adaptação**:

| Estratégia | Quando ativa | O que faz |
| :--- | :--- | :--- |
| **REROUTE** | Drone stuck >8s | Muda para o waypoint mais próximo |
| **DIRECT MODE** | Trajetória A* termina longe | Vai em linha reta com compensação de vento |
| **HOVER** | Após 5 reroutes ou vento > capacidade | Paira no ponto atual (não deriva) |

### Bugs corrigidos durante o stress testing

| Bug | Causa | Correção |
| :--- | :--- | :--- |
| **Abortar sempre** | `stuck_timer` acumulava por iteração | Verificação a cada 1s |
| **Loop infinito de REROUTE** | Contador resetado ao chegar | Contador global (máx 5) |
| **Overshoot de 42m** | `max_speed` demasiado alto (50 m/s) | `max_speed` realista (20 m/s) |
| **Precisão de 12m** | `arrival_radius` folgado | `arrival_radius = 4m` |
| **HOVER indevido** | Voltava ao mesmo waypoint | Contador de reroutes |

### Ficheiros de teste

| Ficheiro | O que faz |
| :--- | :--- |
| `test_wind_matrix.ps1` | Corre o SITL com 6 ventos e produz a tabela |
| `crates/rnfc-mavlink/src/bin/ids_test.rs` | Testa 4 ataques (legítimos, chave errada, replay, sysid) |
| `crates/rnfc-mavlink/src/bin/ids_test2.rs` | Testa 3 regras IDS (flooding, gap, bruteforce) |

**O drone nunca aborta, nunca deriva, nunca entra em loop.** Adapta-se ao vento dentro dos limites físicos.