# RNFC — Security Risk Assessment (DO-326A)

**Projeto:** RNFC — Resilient Navigation & Flight Control
**Autor:** André Oliveira
**Copyright:** (c) 2027 André Oliveira — Todos os direitos reservados
**Versão:** 1.0
**Referência:** DO-326A / ED-202A

---

## 1. Metodologia

Cada **Threat Condition** (condição de ameaça) é classificada em duas dimensões:

- **Severidade:** efeito na segurança do voo (Catastrófico, Perigoso, Maior, Menor, Sem efeito)
- **Probabilidade:** probabilidade de ocorrência (Extremamente Remota, Remota, Rara, Provável, Frequente)

O **risco** é a combinação das duas. O **risco residual** é o risco após a mitigação existente.

**Escala de severidade (DO-178C):**

| Nível | Efeito |
| :--- | :--- |
| **Catastrófico** | Falha que impede o voo seguro e controlado |
| **Perigoso** | Falha que reduz a capacidade da aeronave ou da tripulação |
| **Maior** | Falha que reduz a margem de segurança |
| **Menor** | Falha que não afeta a segurança significativamente |
| **Sem efeito** | Nenhum efeito na segurança |

---

## 2. Avaliação de Risco por Cenário

### T-01 — Injeção de Comandos MAVLink

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Comando de voo não autorizado aceite pelo drone |
| **Severidade** | **Catastrófico** (pode causar terminação de voo ou desvio da missão) |
| **Probabilidade (sem mitigação)** | Frequente (qualquer dispositivo na rede pode enviar MAVLink) |
| **Mitigação** | MAVLink signing (SHA-256) |
| **Probabilidade (com mitigação)** | Extremamente Remota (requer compromisso da chave) |
| **Risco residual** | **Baixo** |

### T-02 — Replay de Comandos Legítimos

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Comando legítimo reenviado fora de contexto |
| **Severidade** | **Catastrófico** |
| **Probabilidade (sem mitigação)** | Provável (captura de tráfego é trivial) |
| **Mitigação** | Timestamp monotónico persistente |
| **Probabilidade (com mitigação)** | Extremamente Remota (timestamp nunca recomeça) |
| **Risco residual** | **Baixo** |

### T-03 — Spoofing de GNSS

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Posição falsa aceite pelo Kalman |
| **Severidade** | **Perigoso** (desvio da trajetória, mas o drone continua a voar) |
| **Probabilidade (sem mitigação)** | Rara (requer equipamento de spoofing) |
| **Mitigação** | Kalman 3D + IMU (GPS-denied) + HDOP < 5 |
| **Probabilidade (com mitigação)** | Remota (a transição para inercial funciona) |
| **Risco residual** | **Médio** (não há deteção explícita de spoofing) |

### T-04 — Compromisso da Chave de Signing

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Chave de 32 bytes extraída do hardware |
| **Severidade** | **Catastrófico** (permite todos os outros ataques) |
| **Probabilidade (sem mitigação)** | Rara (requer acesso físico) |
| **Mitigação** | Provisionamento via `SETUP_SIGNING` (não hardcoded) |
| **Probabilidade (com mitigação)** | Remota |
| **Risco residual** | **Alto** (no SITL, a chave é hardcoded — em produção, deve ser provisionada) |

### T-05 — Substituição de Firmware

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Binário malicioso executado no arranque |
| **Severidade** | **Catastrófico** |
| **Probabilidade (sem mitigação)** | Rara (requer acesso físico) |
| **Mitigação** | ❌ Nenhuma (sem secure boot) |
| **Probabilidade (com mitigação)** | Rara |
| **Risco residual** | **Alto** |

### T-06 — Flooding de Mensagens

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Link saturado, drone em hover indefinido |
| **Severidade** | **Maior** (reduz a capacidade, mas não causa perda) |
| **Probabilidade (sem mitigação)** | Provável |
| **Mitigação** | ⚠️ Parcial (signing autentica, mas não limita taxa) |
| **Probabilidade (com mitigação)** | Rara |
| **Risco residual** | **Médio** |

### T-07 — Leitura de Telemetria

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | Telemetria lida por terceiros |
| **Severidade** | **Menor** (não afeta a segurança do voo) |
| **Probabilidade (sem mitigação)** | Frequente |
| **Mitigação** | ❌ Nenhuma (MAVLink não encripta) |
| **Probabilidade (com mitigação)** | Frequente |
| **Risco residual** | **Médio** (confidencialidade em falta) |

### T-08 — Acesso Não Autorizado a Logs

| Campo | Valor |
| :--- | :--- |
| **Condição de ameaça** | `trajetoria.csv` lido por terceiros |
| **Severidade** | **Menor** |
| **Probabilidade (sem mitigação)** | Rara (requer acesso físico) |
| **Mitigação** | ❌ Nenhuma |
| **Risco residual** | **Baixo** |

---

## 3. Matriz de Risco Residual

| Cenário | Severidade | Risco Residual | Prioridade de Mitigação |
| :--- | :--- | :--- | :--- |
| T-01 (injeção) | Catastrófico | **Baixo** ✅ | Nenhuma (mitigado) |
| T-02 (replay) | Catastrófico | **Baixo** ✅ | Nenhuma (mitigado) |
| T-03 (spoofing GNSS) | Perigoso | **Médio** ⚠️ | Média (deteção de spoofing) |
| T-04 (chave) | Catastrófico | **Alto** ❌ | **Alta** (provisionamento seguro) |
| T-05 (firmware) | Catastrófico | **Alto** ❌ | **Alta** (secure boot) |
| T-06 (flooding) | Maior | **Médio** ⚠️ | Média (rate limiting) |
| T-07 (leitura) | Menor | **Médio** ⚠️ | Baixa (encriptação) |
| T-08 (logs) | Menor | **Baixo** ✅ | Baixa (encriptação em repouso) |

---

## 4. Conclusão

**Riscos mitigados (Baixo):** T-01, T-02, T-08 — o signing e o anti-replay resolvem os ataques mais críticos.

**Riscos a mitigar (Alto):** T-04, T-05 — o provisionamento da chave e o secure boot são os próximos passos prioritários.

**Riscos a monitorizar (Médio):** T-03, T-06, T-07 — a deteção de spoofing, o rate limiting e a encriptação são melhorias incrementais.

---

**Documento propriedade de André Oliveira — (c) 2027. Todos os direitos reservados.**