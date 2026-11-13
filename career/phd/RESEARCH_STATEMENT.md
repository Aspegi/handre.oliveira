# PhD Application Package

## Research Statement (2 páginas)

### Título proposto
**"Cryptographic Safety Certification for Autonomous Systems"**

### Problema
A certificação de drones autónomos depende de safety cases estáticos e
logs não verificáveis. Reguladores (EASA, FAA) e seguradoras não conseguem
provar que uma missão foi segura sem confiar no operador.

### Contribuição proposta
1. **SCC** — compilador de safety case (requisitos → testes + GSN + chain)
2. **CSR** — auditoria contrafactual ("e se o vento fosse 2×?") com prova
3. **Integração com DO-326A** — PSecAC formal com evidência criptográfica

### Metodologia
- Rust `no_std` para o MCU, `std` para o companion
- Hash chains (SHA-256) + ancoragem externa
- Validação em hardware (Pixhawk 4, STM32F4)
- Avaliação com reguladores (EASA) e seguradoras

### Resultados preliminares
- 277 testes, 19 crates
- 2 papers compilados (SCC, CSR)
- DO-326A PSecAC draft (12 ameaças, 13 requisitos)
- Secure Boot + MagNav real (integration ready)

### Porque este PhD
O RNFC é a base. O PhD adiciona:
- Formal verification (Kani, Coq) dos invariantes
- Validação com EASA/FAA (processo real)
- Publicação em venues de topo (SAFECOMP, IEEE S&P)

## Programas-Alvo

| Programa | Supervisor potencial | Deadline |
|---|---|---|
| **ETH Zurich** | Prof. Roland Siegwart (ASL) | Dez-Jan |
| **TU Delft** | MAVLab (Prof. Guido de Croon) | Dez-Jan |
| **MIT AeroAstro** | Prof. Jonathan How | Dez |
| **Stanford ASL** | Prof. Marco Pavone | Dez |
| **Imperial College** | Aerial Robotics Lab | Jan |
| **Carnegie Mellon** | AirLab | Dez |

## Checklist de Submissão (por programa)

- [ ] Research statement (2 páginas) — este documento
- [ ] CV (com 277 testes, 19 crates, 2 papers)
- [ ] 2 cartas de recomendação (professores ou mentores)
- [ ] Transcripts
- [ ] TOEFL/IELTS (se necessário)
- [ ] GRE (se necessário — muitos programas já não pedem)
- [ ] Carta de motivação (1 página)
- [ ] Portfólio: link para github.com/Aspegi/handre.oliveira
- [ ] Papers: SCC e CSR (PDFs)

## O Que Destacar no CV

- **277 testes** em 19 crates (mostra disciplina)
- **2 papers** (SCC, CSR) — contribuições novas
- **DO-326A PSecAC** (12 ameaças, 13 requisitos)
- **Secure Boot + MagNav real** (hardware-ready)
- **9 tentativas de multi-waypoint documentadas** (honestidade)
- **CI/CD** com 6 jobs (engenharia profissional)

## Próximos Passos

1. Publicar no HN (visibilidade para recrutadores)
2. Submeter os 2 papers (IEEE Aerospace, DASC)
3. Contactar supervisores (email com link para o repo)
4. Preparar CV + research statement
5. Submeter até Dezembro 2026 (para Fall 2027)
