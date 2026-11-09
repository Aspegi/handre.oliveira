# RNFC — Security Requirements (DO-326A)

**Projeto:** RNFC — Resilient Navigation & Flight Control
**Autor:** André Oliveira
**Copyright:** (c) 2027 André Oliveira — Todos os direitos reservados
**Versão:** 1.0
**Referência:** DO-326A / ED-202A — Artefacto 3

---

## 1. Metodologia

Cada requisito deriva de uma **mitigação** identificada no Threat Model e tem:

- **ID:** `SEC-REQ-NNN` (rastreável)
- **Descrição:** o que o sistema deve fazer
- **Rationale:** porque existe (liga ao cenário T-NN)
- **Método de verificação:** como se prova que está implementado
- **Estado:** Implementado / Parcial / Pendente

---

## 2. Requisitos de Autenticação (T-01)

### SEC-REQ-001 — Assinatura MAVLink

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve assinar cada frame MAVLink v2 com SHA-256 sobre `(secret_key + header + payload + checksum + link_id + timestamp)`, anexando os primeiros 6 bytes |
| **Rationale** | Mitiga T-01 (injeção de comandos) — sem assinatura, o drone aceita qualquer comando |
| **Verificação** | Verificador independente em PowerShell (`System.Security.Cryptography.SHA256`); 662 mensagens/sessão, 0 inválidas |
| **Estado** | ✅ Implementado |

### SEC-REQ-002 — Rejeição de Mensagens Não Assinadas

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve rejeitar mensagens MAVLink sem o bit `incompat_flags & 0x01` ativo quando o signing está habilitado |
| **Rationale** | Mitiga T-01 — impede que mensagens em claro sejam aceites |
| **Verificação** | Listener PowerShell verifica `incompat_flags` em cada pacote |
| **Estado** | ✅ Implementado |

### SEC-REQ-003 — Chave de 32 Bytes

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | A chave de signing deve ter exatamente 32 bytes (256 bits) |
| **Rationale** | Consistência com MAVLink v2 spec e robustez criptográfica |
| **Verificação** | Tipo `[u8; 32]` em Rust (garantido pelo compilador) |
| **Estado** | ✅ Implementado |

---

## 3. Requisitos Anti-Replay (T-02)

### SEC-REQ-004 — Timestamp Monotónico

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | Cada frame assinado deve incrementar o `signing_timestamp` em 1 unidade (10 µs) |
| **Rationale** | Mitiga T-02 (replay) — sem monotonicidade, mensagens antigas são aceites |
| **Verificação** | O verificador observa timestamps crescentes (1, 2, 3, ...) |
| **Estado** | ✅ Implementado |

### SEC-REQ-005 — Persistência do Timestamp

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O `signing_timestamp` deve ser persistido em disco e recarregado no arranque |
| **Rationale** | Mitiga T-02 — impede replay entre sessões (o timestamp nunca recomeça em 0) |
| **Verificação** | `mavlink_signing_ts.txt` mostra 1986 → 2648 entre sessões |
| **Estado** | ✅ Implementado |

### SEC-REQ-006 — Rejeição de Timestamps Antigos

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve rejeitar mensagens com timestamp anterior ao último aceite para o mesmo `(sysid, compid, link_id)` |
| **Rationale** | Mitiga T-02 — a verificação no lado do recetor é o complemento do emissor |
| **Verificação** | Teste com replay de pacotes gravados (a implementar no `MavlinkReceiver`) |
| **Estado** | ⚠️ Parcial (emissor implementado, recetor pendente) |

---

## 4. Requisitos de Resiliência GNSS (T-03)

### SEC-REQ-007 — Navegação GPS-Denied

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve continuar a navegar quando o GNSS é perdido, usando Kalman 3D + IMU + atitude |
| **Rationale** | Mitiga T-03 (spoofing/jamming GNSS) |
| **Verificação** | SITL demonstra transição `GnssAided → Degraded → Inertial` sem divergência |
| **Estado** | ✅ Implementado |

### SEC-REQ-008 — Verificação de HDOP

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve rejeitar fixes GNSS com `HDOP ≥ 5.0` |
| **Rationale** | Mitiga T-03 — fixes de baixa qualidade são descartados |
| **Verificação** | Constante `HDOP_MAX = 5.0` no `NavCore` |
| **Estado** | ✅ Implementado |

---

## 5. Requisitos de Provisionamento (T-04)

### SEC-REQ-009 — Provisionamento Fora de Banda

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | A chave de signing deve ser provisionada via `SETUP_SIGNING` numa ligação de confiança (USB), nunca hardcoded no binário |
| **Rationale** | Mitiga T-04 (compromisso de chave) — hardcoding expõe a chave a qualquer extração |
| **Verificação** | Inspeção do binário não contém a chave em claro |
| **Estado** | ❌ Pendente (SITL usa chave de teste hardcoded) |

### SEC-REQ-010 — Rotação de Chaves

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve suportar rotação periódica da chave de signing sem reinício |
| **Rationale** | Mitiga T-04 — limita a janela de exposição |
| **Verificação** | Teste de rotação com duas chaves (a implementar) |
| **Estado** | ❌ Pendente |

---

## 6. Requisitos de Integridade de Firmware (T-05)

### SEC-REQ-011 — Secure Boot

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O bootloader deve verificar a assinatura do binário antes de o executar |
| **Rationale** | Mitiga T-05 (substituição de firmware) |
| **Verificação** | Teste de binário modificado é rejeitado no arranque |
| **Estado** | ❌ Pendente |

---

## 7. Requisitos de Rate Limiting (T-06)

### SEC-REQ-012 — Limite de Taxa MAVLink

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O RNFC deve rejeitar mensagens que excedam uma taxa configurável (ex: 50 msg/s por `sysid`) |
| **Rationale** | Mitiga T-06 (flooding) |
| **Verificação** | Teste com flood (a implementar no `MavlinkReceiver`) |
| **Estado** | ⚠️ Parcial (contador existe, sem enforcement) |

---

## 8. Requisitos de Confidencialidade (T-07, T-08)

### SEC-REQ-013 — Encriptação do Link

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | O link MAVLink deve ser encriptado (VPN/IPsec ou AES-256) |
| **Rationale** | Mitiga T-07 (leitura de telemetria) |
| **Verificação** | Análise de tráfego não revela conteúdo em claro |
| **Estado** | ❌ Pendente (fora do âmbito do software) |

### SEC-REQ-014 — Encriptação de Logs

| Campo | Detalhe |
| :--- | :--- |
| **Descrição** | `trajetoria.csv` e logs devem ser cifrados em repouso (AES-256-GCM) |
| **Rationale** | Mitiga T-08 (acesso a logs) |
| **Verificação** | Ficheiro em disco não contém texto em claro |
| **Estado** | ❌ Pendente |

---

## 9. Rastreabilidade

| Cenário | Requisitos | Estado |
| :--- | :--- | :--- |
| T-01 (injeção) | SEC-REQ-001, 002, 003 | ✅ Implementado |
| T-02 (replay) | SEC-REQ-004, 005, 006 | ⚠️ Parcial |
| T-03 (spoofing GNSS) | SEC-REQ-007, 008 | ✅ Implementado |
| T-04 (chave) | SEC-REQ-009, 010 | ❌ Pendente |
| T-05 (firmware) | SEC-REQ-011 | ❌ Pendente |
| T-06 (flooding) | SEC-REQ-012 | ⚠️ Parcial |
| T-07 (leitura) | SEC-REQ-013 | ❌ Pendente |
| T-08 (logs) | SEC-REQ-014 | ❌ Pendente |

**Resumo:** 6 requisitos implementados, 2 parciais, 6 pendentes.

---

**Documento propriedade de André Oliveira — (c) 2027. Todos os direitos reservados.**