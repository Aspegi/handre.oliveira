# RNFC v6.7.0 — Checklist Final

## O Que Está Feito (Código)

- [x] 19 crates, 277 testes
- [x] 12 demos end-to-end
- [x] 13/13 requisitos de segurança documentados (12 completos, 1 parcial)
- [x] DO-326A PSecAC draft
- [x] Threat model (12 ameaças)
- [x] Secure Boot integration package
- [x] SBOM + cargo-deny
- [x] MagNav real (NV driver + mapa real)
- [x] Pixhawk HAL skeleton
- [x] CI/CD com 6 jobs

## O Que Está Feito (Papers)

- [x] SCC: abstract + introduction + 7 secções + figura TikZ
- [x] CSR: abstract + introduction + 7 secções + figura TikZ
- [x] references.bib (18+ entradas)
- [x] main.tex integrados (input dos ficheiros)
- [x] Cover letters (CSR + SCC)
- [x] Script de compilação (compile.ps1)

## O Que Falta (Trabalho Teu)

### Curto prazo (1-2 dias)
- [ ] Instalar MiKTeX (https://miktex.org/download)
- [ ] Compilar os 2 papers: `.\papers\compile.ps1`
- [ ] Ler os PDFs, corrigir typos
- [ ] Verificar cada citação no Google Scholar

### Médio prazo (1-2 semanas)
- [ ] Adicionar 2-3 citações que só tu podes procurar
- [ ] Preencher cover letters (afiliação, ORCID)
- [ ] Submeter CSR ao IEEE Aerospace (deadline ~Nov 2026)
- [ ] Submeter SCC ao DASC (deadline ~Jun 2027)

### Longo prazo (hardware)
- [ ] Comprar STM32F4 (~€25) para Secure Boot
- [ ] Comprar Pixhawk (~€100) para voo real
- [ ] Comprar sensor NV (~$10k) para MagNav real

### Visibilidade (20 min)
- [ ] Postar no Hacker News (texto em HN_POST.md)
- [ ] Postar no LinkedIn

## Comandos Úteis

```powershell
# Compilar papers
cd C:\dev\rnfc\papers\scc
pdflatex main.tex && bibtex main && pdflatex main.tex && pdflatex main.tex

# Correr testes
cd C:\dev\rnfc
cargo test --workspace --no-fail-fast

# Ver estado
git log --oneline -5
git tag --list "v6*"
```

## Ficheiros Importantes

| Ficheiro | O Que Contém |
|---|---|
| `README.md` | Visão geral do projecto |
| `SITL_6DOF.md` | 9 tentativas de multi-waypoint |
| `SECURITY.md` | Todas as medidas de segurança |
| `DO-326A_COMPLIANCE.md` | PSecAC draft |
| `security/THREAT_MODEL.md` | 12 ameaças |
| `security/SECURITY_REQUIREMENTS.md` | Matriz de rastreabilidade |
| `SECURE_BOOT.md` | Guia STM32F4 + SBSFU |
| `MAGNAV_REAL.md` | Driver NV + mapa real |
| `papers/` | 2 papers completos |
| `HN_POST.md` | Post para Hacker News |

## O Que NÃO Fazer

- ❌ Crate #20 (não há mais código que valha a pena)
- ❌ Mais demos (12 cobrem tudo)
- ❌ Mais testes (277 são suficientes)
- ❌ Exagerar no HN post (a honestidade é a defesa)

## O Que Fazer

- ✅ Compilar os papers
- ✅ Verificar citações
- ✅ Submeter
- ✅ Publicar no HN
- ✅ Comprar hardware (se quiseres)

---

*O RNFC está entregue em v6.7.0. O próximo passo é teu.*
