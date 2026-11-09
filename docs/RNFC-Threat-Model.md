# RNFC — Threat Model (DO-326A)

**Projeto:** RNFC — Resilient Navigation & Flight Control
**Autor:** André Oliveira
**Copyright:** (c) 2027 André Oliveira — Todos os direitos reservados
**Versão:** 1.0
**Data:** 2027
**Referência:** DO-326A / ED-202A — Airworthiness Security Process Specification

---

## 1. Âmbito e Objetivo

Este documento identifica os **ativos** do RNFC, as **fronteiras de segurança** e os **cenários de ataque** relevantes para a aeronavegabilidade. É o primeiro artefacto do processo DO-326A e serve de base para:

- Avaliação de risco de segurança (documento 2)
- Requisitos de segurança (documento 3)
- Evidência de mitigação (documento 4)
- Monitorização em operação (documento 5)

**Pressuposto:** o RNFC é um componente de navegação e controlo que comunica com uma estação de terra (GCS) via MAVLink. A aeronave é não tripulada.

---

## 2. Ativos (Assets)

| ID | Ativo | Descrição | Criticidade |
| :--- | :--- | :--- | :--- |
| A-01 | **Comandos de voo** | Mensagens MAVLink que alteram a trajetória (waypoints, atitude, terminação) | Catastrófico |
| A-02 | **Telemetria de estado** | Posição, atitude, velocidade enviadas para a GCS | Maior |
| A-03 | **Chave de signing MAVLink** | 32 bytes partilhados entre drone e GCS | Catastrófico |
| A-04 | **Timestamp de signing** | Contador anti-replay persistido em disco | Maior |
| A-05 | **Parâmetros de controlo** | Ganhos do PID, limites de velocidade, `max_pitch` | Maior |
| A-06 | **Waypoints da missão** | Lista de destinos A, B, C | Maior |
| A-07 | **Firmware / binário** | Código executável do RNFC | Catastrófico |
| A-08 | **Logs de voo** | `trajetoria.csv`, telemetria histórica | Menor |
| A-09 | **Estado do Kalman** | Covariância, posição estimada | Maior |

---

## 3. Fronteiras de Segurança (Trust Boundaries)

| ID | Fronteira | Descrição | Confiança |
| :--- | :--- | :--- | :--- |
| TB-01 | **Link de rádio MAVLink** | Comunicação drone ↔ GCS (UDP, rádio RF) | **Não confiável** |
| TB-02 | **Interface de manutenção** | Acesso físico ao hardware (USB, JTAG) | **Confiável** (com controlo de acesso) |
| TB-03 | **Sensores (IMU, GNSS, LiDAR)** | Entradas físicas do ambiente | **Não confiável** (suscetível a spoofing) |
| TB-04 | **Armazenamento persistente** | Ficheiros (`mavlink_signing_ts.txt`, `trajetoria.csv`) | **Confiável** (se o hardware não for comprometido) |
| TB-05 | **Processo de build** | `cargo build` → binário | **Confiável** (mas sem secure boot) |

---

## 4. Cenários de Ataque (Threat Scenarios)

### T-01 — Injeção de Comandos MAVLink

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-01 (comandos de voo) |
| **Fronteira** | TB-01 (link de rádio) |
| **Descrição** | Um atacante na mesma rede/rádio envia um comando `MAV_CMD_DO_FLIGHTTERMINATION` ou altera waypoints |
| **Pré-condições** | Acesso ao link MAVLink (sem autenticação) |
| **Mitigação existente** | ✅ **MAVLink signing** (SHA-256) — o drone rejeita mensagens sem assinatura válida |
| **Risco residual** | Baixo (se a chave não for comprometida) |

### T-02 — Replay de Comandos Legítimos

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-01 (comandos de voo) |
| **Fronteira** | TB-01 (link de rádio) |
| **Descrição** | Um atacante grava e reenvia comandos legítimos de uma sessão anterior |
| **Pré-condições** | Capacidade de capturar tráfego MAVLink |
| **Mitigação existente** | ✅ **Timestamp monotónico persistente** (anti-replay) — mensagens com timestamp anterior são rejeitadas |
| **Risco residual** | Baixo (timestamp nunca recomeça entre sessões) |

### T-03 — Spoofing de GNSS

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-09 (estado do Kalman), A-02 (telemetria) |
| **Fronteira** | TB-03 (sensores) |
| **Descrição** | Um atacante emite sinais GNSS falsos para desviar o drone |
| **Pré-condições** | Equipamento de spoofing GNSS |
| **Mitigação existente** | ✅ **Navegação GPS-denied** (Kalman 3D + IMU + atitude) — o drone continua a navegar sem GNSS; ✅ **Verificação de consistência** (HDOP < 5) |
| **Risco residual** | Médio (a transição para inercial funciona, mas não há deteção explícita de spoofing) |

### T-04 — Compromisso da Chave de Signing

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-03 (chave de signing) |
| **Fronteira** | TB-02 (interface de manutenção) |
| **Descrição** | Um atacante com acesso físico extrai a chave de 32 bytes |
| **Pré-condições** | Acesso físico ao hardware |
| **Mitigação existente** | ⚠️ **Parcial** — a chave é hardcoded no SITL (chave de teste); em produção, deve ser provisionada via `SETUP_SIGNING` |
| **Risco residual** | Alto (se o hardware for comprometido) |

### T-05 — Substituição de Firmware

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-07 (firmware) |
| **Fronteira** | TB-05 (processo de build) |
| **Descrição** | Um atacante substitui o binário por um malicioso |
| **Pré-condições** | Acesso físico ao armazenamento |
| **Mitigação existente** | ❌ **Nenhuma** — sem secure boot |
| **Risco residual** | **Alto** (mitigação recomendada: bootloader com chave pública) |

### T-06 — Flooding de Mensagens

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-01, A-02 |
| **Fronteira** | TB-01 |
| **Descrição** | Um atacante satura o link com mensagens para forçar o drone a hover indefinidamente |
| **Pré-condições** | Acesso ao link |
| **Mitigação existente** | ⚠️ **Parcial** — o signing autentica, mas não limita a taxa |
| **Risco residual** | Médio (mitigação recomendada: rate limiting no `MavlinkSender`) |

### T-07 — Leitura de Telemetria (Confidencialidade)

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-02 (telemetria), A-08 (logs) |
| **Fronteira** | TB-01, TB-04 |
| **Descrição** | Um atacante lê a telemetria (posição, atitude) sem a alterar |
| **Pré-condições** | Acesso ao link |
| **Mitigação existente** | ❌ **Nenhuma** — MAVLink não encripta |
| **Risco residual** | Médio (mitigação recomendada: VPN/IPsec ou AES-256) |

### T-08 — Acesso Não Autorizado a Logs

| Campo | Detalhe |
| :--- | :--- |
| **Ativo afetado** | A-08 (logs) |
| **Fronteira** | TB-04 |
| **Descrição** | Um atacante com acesso ao hardware lê `trajetoria.csv` |
| **Pré-condições** | Acesso físico |
| **Mitigação existente** | ❌ **Nenhuma** — logs em claro |
| **Risco residual** | Baixo (mitigação recomendada: AES-256 em repouso) |

---

## 5. Resumo das Mitigações

| Cenário | Mitigação | Estado |
| :--- | :--- | :--- |
| T-01 (injeção) | MAVLink signing (SHA-256) | ✅ Implementado |
| T-02 (replay) | Timestamp monotónico persistente | ✅ Implementado |
| T-03 (spoofing GNSS) | Kalman 3D + IMU (GPS-denied) | ✅ Implementado |
| T-04 (chave) | Provisionamento via `SETUP_SIGNING` | ⚠️ Parcial (SITL usa chave de teste) |
| T-05 (firmware) | Secure boot | ❌ Não implementado |
| T-06 (flooding) | Rate limiting | ⚠️ Parcial |
| T-07 (leitura) | Encriptação (VPN/AES) | ❌ Não implementado |
| T-08 (logs) | Encriptação em repouso | ❌ Não implementado |

---

## 6. Rastreabilidade

Este threat model alimenta:

- **`RNFC-Security-Risk-Assessment.md`** — severidade e risco residual
- **`RNFC-Security-Requirements.md`** — requisitos derivados
- **`RNFC-Security-Evidence.md`** — evidência de mitigação

---

**Documento propriedade de André Oliveira — (c) 2027. Todos os direitos reservados.**