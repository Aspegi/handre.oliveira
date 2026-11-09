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

