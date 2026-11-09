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

## Seguranca

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

### O que NAO esta protegido

| Ameaca | Estado | Mitigacao recomendada |
| :--- | :--- | :--- |
| **Confidencialidade** | ❌ MAVLink nao encripta | VPN/IPsec ou radio com AES-256 |
| **Secure boot** | ❌ Firmware nao verificado | Bootloader com chave publica |
| **Deteccao de intrusao** | ❌ Sem monitorizacao de trafego | IDS com regras MAVLink |
| **Encriptacao de logs** | ❌ `trajetoria.csv` em claro | AES-256 em repouso |
| **Rotacao de chaves** | ❌ Manual | Protocolo de rotacao periodica |
| **Provisionamento** | ❌ Chave de teste hardcoded | `SETUP_SIGNING` em ligacao de confianca |

### Caminho para certificacao de seguranca (DO-326A)

O RNFC esta pronto para o **DO-178C** (seguranca funcional: `no_std`, Kani, sem `unsafe`), mas ainda nao tem os artefactos do **DO-326A** (seguranca de aeronavegabilidade):

| Requisito DO-326A | Estado |
| :--- | :--- |
| Analise de ameacas (threat model) | ⬜ |
| Avaliacao de risco de seguranca | ⬜ |
| Requisitos de seguranca | ⬜ |
| Evidencia de mitigacao | ⬜ (signing implementado) |
| Monitorizacao em operacao | ⬜ |

**Nota:** para operacoes militares ou civis certificadas, o hardening da comunicacao (encriptacao, secure boot) e a documentacao DO-326A sao pre-requisitos.

### Teste do signing

O signing foi validado com um verificador independente (PowerShell, `System.Security.Cryptography.SHA256`):

- **662 mensagens por sessao** (601 ATTITUDE + 61 HEARTBEAT)
- **Todas assinadas** (bit `incompat_flags & 0x01` ativo)
- **Todas validas** (SHA-256 calculado em Rust == SHA-256 calculado em PowerShell)
- **Anti-replay:** timestamp 1986 -> 2648 entre sessoes (nunca recomeca)