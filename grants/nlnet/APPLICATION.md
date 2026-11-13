# NLnet NGI Zero Application — RNFC

**Programa:** NGI Zero Core (ou NGI Zero Entrust se disponível)
**Deadline:** Verificar em https://nlnet.nl/propose/ (geralmente 1º de cada mês)
**Montante pedido:** €30,000
**Duração:** 9 meses

---

## 1. Informação Básica

| Campo | Valor |
|---|---|
| **Nome do projeto** | RNFC — Resilient Navigation & Flight Control |
| **Website** | https://github.com/Aspegi/handre.oliveira |
| **Licença** | MIT OR Apache-2.0 (dual) |
| **Contacto** | Andre Oliveira <handre.oliveira@gmail.com> |
| **País** | Portugal |
| **ORCID** | (preencher se tiveres) |

## 2. Abstract (máx 200 palavras)

RNFC is an open-source flight-control stack for multirotor UAVs, written
in Rust (`no_std` for MCU, `std` for companion). It provides layered FDIR
(fault detection, isolation, recovery), LNN-based actuator fault recovery,
MAVLink signing with anti-replay, a SHA-256 hash-chain safety ledger with
external anchoring, and a DO-326A security compliance package. The project
has 277 passing tests, 19 crates, and 2 research papers (SCC: safety case
compiler; CSR: counterfactual safety reasoning). It is not flight-tested
on hardware; the HAL for Pixhawk and Secure Boot for STM32F4 are skeletons.
The goal of this grant is to complete the hardware path (Pixhawk HAL,
Secure Boot), implement a geometric SE(3) controller for multi-waypoint
flight, commission an external security audit, and submit both papers to
peer-reviewed venues. The project fills a gap in open-source safety-critical
UAV software: no existing project combines FDIR, LNN recovery, cryptographic
certification, and DO-326A compliance in a single stack.

## 3. Descrição Completa do Projeto

### 3.1 Problema

Open-source UAV flight controllers (ArduPilot, PX4) are mature in flight
control but lack three things that certification (EASA, FAA) demands:

1. **Layered fault detection with learning-based recovery.** FDIR exists
   but is rule-based; motor loss recovery is not standard.
2. **Cryptographic safety certification.** Black-box logs are not
   tamper-evident; there is no proof that the mission was safe.
3. **Security compliance (DO-326A).** No open-source UAV project has a
   PSecAC draft, a threat model, or a requirements traceability matrix.

RNFC addresses all three.

### 3.2 Estado Atual (o que já existe)

| Componente | Estado |
|---|---|
| Flight control (Kalman, PID, guidance) | ✅ 277 testes |
| FDIR (5 camadas) | ✅ 17 testes |
| LNN-FTC (motor loss) | ✅ 6 testes, 804 params |
| MAVLink signing | ✅ 11 testes, 4/4 ataques |
| Hash-chain ledger + anchor | ✅ 38 testes, 3/3 ataques |
| DO-326A package | ✅ PSecAC draft, 12 ameaças, 13 requisitos |
| SBOM + cargo-deny | ✅ |
| 6-DOF physics (rotor-rs) | ✅ Single waypoint, 4cm erro |
| MagNav real (NV driver + mapa) | ✅ Crate + loader |
| Pixhawk HAL | ⚠️ Skeleton (4 testes) |
| Secure Boot | ⚠️ Integration guide only |
| Multi-waypoint 6-DOF | ❌ 9 tentativas falhadas (documentado) |
| Papers | ⚠️ 2 compilados, não submetidos |

### 3.3 O Que o Grant Financia

| Tarefa | Entregável | Meses | Custo |
|---|---|---|---|
| **T1: Pixhawk HAL completo** | HAL para Pixhawk 4 (STM32F765), IMU/GNSS/MAG drivers, PWM output, testado em hardware | 1-4 | €8,000 |
| **T2: Secure Boot** | Integração X-CUBE-SBSFU, assinatura de firmware Rust, teste de arranque seguro | 3-5 | €4,000 |
| **T3: Controlador SE(3)** | Controlador geométrico para multi-waypoint 6-DOF, substitui PID linear | 4-7 | €6,000 |
| **T4: Auditoria de segurança** | Auditoria externa (MAVLink, CertChain, DO-326A) | 6-8 | €8,000 |
| **T5: Papers** | Submissão ao IEEE Aerospace (CSR) e DASC (SCC) | 1-9 | €2,000 |
| **T6: Hardware** | Pixhawk 4, STM32F4-Discovery, ST-LINK | 1 | €2,000 |
| **Total** | | **9** | **€30,000** |

### 3.4 Porque a NLnet Deve Financiar

1. **Open-source safety-critical software.** Não existe alternativa
   open-source com FDIR + LNN + certificação criptográfica.
2. **Segurança (NGI Zero Entrust).** MAVLink signing, SBOM, DO-326A,
   hash-chain com ancoragem externa — tudo alinhado com a missão da NLnet.
3. **Rust `no_std`.** Avança o ecossistema Rust embarcado, que a NLnet
   tem financiado (ex: projetos Rust no NGI Zero).
4. **Contribuição científica.** 2 papers com contribuições genuinamente
   novas (SCC, CSR), que serão publicados em acesso aberto.
5. **Transparência.** O projecto documenta honestamente as limitações
   (9 tentativas de multi-waypoint, 2 gaps de segurança). Isto é raro
   e alinhado com os valores da NLnet.

### 3.5 Riscos e Mitigação

| Risco | Mitigação |
|---|---|
| HAL não funciona em hardware | Começar com Pixhawk 4 (bem documentado); fallback para STM32F4-Discovery |
| Secure Boot complexo | Usar X-CUBE-SBSFU oficial da ST; guia já escrito (SECURE_BOOT.md) |
| SE(3) controller não converge | Referência: Lee 2010; já temos rotor-rs com Se3Control |
| Papers rejeitados | Targets alternativos: DASC, ICUAS, SAFECOMP |

### 3.6 Equipa

| Nome | Papel | Experiência |
|---|---|---|
| Andre Oliveira | Lead developer | 277 testes, 19 crates, 2 papers (autor único) |
| (a contratar) | Auditor de segurança | Externo, orçamento T4 |

### 3.7 Orçamento Detalhado

| Item | Custo |
|---|---|
| Desenvolvimento (T1+T2+T3) | €18,000 |
| Auditoria externa (T4) | €8,000 |
| Hardware (T6) | €2,000 |
| Papers / viagens (T5) | €2,000 |
| **Total** | **€30,000** |

### 3.8 Cronograma

```
Mês 1: Hardware + setup + Pixhawk HAL (início)
Mês 2-4: Pixhawk HAL (completo, testado)
Mês 3-5: Secure Boot (STM32F4 + SBSFU)
Mês 4-7: Controlador SE(3) para multi-waypoint
Mês 6-8: Auditoria externa
Mês 1-9: Papers (submissão contínua)
Mês 9: Relatório final + release v7.0.0
```

### 3.9 Saídas (Deliverables)

1. `rnfc-hal-pixhawk` v1.0 (HAL completo, testado em hardware)
2. `rnfc-secure-boot` (integração SBSFU, firmware assinado)
3. `rnfc-se3` (controlador geométrico, multi-waypoint funcional)
4. Relatório de auditoria de segurança (externo)
5. 2 papers submetidos (IEEE Aerospace + DASC)
6. DO-326A compliance package v2 (com auditoria)
7. Release v7.0.0 (todas as melhorias integradas)

### 3.10 Como o Dinheiro Será Gasto

- **€18,000 desenvolvimento:** 9 meses × €2,000/mês (tempo parcial,
  compatível com emprego ou PhD)
- **€8,000 auditoria:** empresa externa (ex: Trail of Bits, NCC Group,
  ou auditor independente)
- **€2,000 hardware:** Pixhawk 4 (~€100), STM32F4-Discovery (~€15),
  ST-LINK (~€10), sensor NV (~$1,800 se disponível)
- **€2,000 papers:** taxas de submissão, viagens a conferências

## 4. Perguntas Frequentes da NLnet (respostas)

**Q: O projeto já recebeu financiamento?**
A: Não. É um projeto individual, auto-financiado até agora.

**Q: Qual é o impacto social?**
A: Drones BVLOS (entregas, SAR, inspeção) precisam de certificação.
   RNFC é a única stack open-source que combina FDIR, LNN, e certificação
   criptográfica. Isto reduz a barreira para fabricantes pequenos.

**Q: Como será mantido após o grant?**
A: O código é MIT/Apache-2.0. A comunidade Rust embarcado pode contribuir.
   Os papers aumentam a visibilidade e atraem colaboradores.

**Q: Qual é o aspeto mais inovador?**
A: SCC (compilador de safety case) e CSR (counterfactual proofs) não
   existem na literatura com esta combinação. Ambos serão publicados.

## 5. Links

- Repositório: https://github.com/Aspegi/handre.oliveira
- README: (link)
- SITL_6DOF.md (limitações): (link)
- SECURITY.md (medidas): (link)
- DO-326A_COMPLIANCE.md (PSecAC): (link)
- Papers: papers/scc/main.pdf, papers/csr/main.pdf

---

## Notas Para Ti (apagar antes de submeter)

- [ ] Verificar deadline em https://nlnet.nl/propose/
- [ ] Preencher ORCID (se tiveres)
- [ ] Ajustar orçamento se a NLnet pedir valores diferentes
- [ ] Adicionar links directos (não "(link)")
- [ ] Se tiveres afiliação institucional, adicionar
- [ ] Considerar NGI Zero Entrust (segurança) em vez de Core
- [ ] Prazo de submissão: 1º de cada mês (verificar)
