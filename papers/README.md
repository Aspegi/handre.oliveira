# Como Preencher os Papers — Guia Prático

## O Que Já Tens

| Elemento | Onde | Estado |
|---|---|---|
| Abstract | `main.tex` | ✅ Escrito |
| Estrutura (8 secções) | `main.tex` | ✅ Esboçada |
| Resultados experimentais | `*_demo` outputs | ✅ Reais |
| Limitações honestas | `SITL_6DOF.md`, `SECURITY_REQUIREMENTS.md` | ✅ Documentadas |

## O Que Falta (2-4 semanas cada)

### 1. Literature Review (1 semana)

**SCC:**
- GSN: Kelly (1998), Kelly & Weaver (2004)
- Safety cases: EASA SC-VTOL, DO-178C
- Runtime verification: Bartocci et al. (2018)
- Existing tools: SafeCert, BusbySim

**CSR:**
- Counterfactual reasoning: Pearl (2009), Lewis (1973)
- Sensitivity analysis: Saltelli et al. (2008)
- Black-box logs: ArduPilot, PX4
- Cryptographic audit: Crosby & Wallach (2009)

### 2. Escrita (1-2 semanas)

Cada secção tem um comentário `%` no `main.tex` com o que escrever.
Segue a ordem: Intro → Background → Design → Implementation → Evaluation → Discussion → Conclusion.

### 3. Figuras (2-3 dias)

**SCC:**
- Figura 1: Pipeline (DSL → parse → 3 artefactos)
- Figura 2: Exemplo de GSN gerado

**CSR:**
- Figura 1: Arquitectura (trajectory → replay → proof)
- Figura 2: Tabela de cenários (4 cenários, 4 provas)

### 4. Revisão (3-5 dias)

- Verificar citações (Google Scholar)
- Verificar formatação (IEEEtran)
- Submeter ao target

## Targets

| Paper | Target | Deadline | Aceitação |
|---|---|---|---|
| SCC | SAFECOMP 2027 | ~Março 2027 | ~30% |
| SCC (alt) | DASC 2027 | ~Junho 2027 | ~50% |
| CSR | IEEE Aerospace 2027 | ~Nov 2026 | ~60% |
| CSR (alt) | DASC 2027 | ~Junho 2027 | ~50% |

**Recomendação:** submete o CSR ao IEEE Aerospace (deadline mais próximo,
aceitação mais alta). O SCC ao DASC (mais acessível que SAFECOMP).

## Compilação

```bash
cd papers/scc && pdflatex main.tex && bibtex main && pdflatex main.tex && pdflatex main.tex
cd papers/csr && pdflatex main.tex && bibtex main && pdflatex main.tex && pdflatex main.tex
```

Requer: MiKTeX (Windows) ou TeX Live (Linux/Mac).
