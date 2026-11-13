# Submission Checklist

## Como compilar (precisas de MiKTeX ou TeX Live)

```powershell
cd C:\dev\rnfc\papers\scc
pdflatex main.tex
bibtex main
pdflatex main.tex
pdflatex main.tex

cd C:\dev\rnfc\papers\csr
pdflatex main.tex
bibtex main
pdflatex main.tex
pdflatex main.tex
```

## Antes de submeter

### Verificação de conteúdo
- [ ] Ler o PDF inteiro do SCC — corrigir typos
- [ ] Ler o PDF inteiro do CSR — corrigir typos
- [ ] Verificar as 2 figuras (TikZ) — estão legíveis?
- [ ] Verificar o limite de páginas (IEEE Aerospace: 6-8; DASC: 6-8)

### Verificação de citações (Google Scholar)
- [ ] kelly1998 — Arguing Safety (PhD thesis)
- [ ] kelly2004 — GSN notation
- [ ] easa_sc_vtol — Special Condition VTOL
- [ ] do178c — DO-178C
- [ ] do326a — DO-326A
- [ ] bartocci2018 — Runtime Verification
- [ ] crosby2009 — Tamper-Evident Logging
- [ ] merkle1988 — Digital Signature
- [ ] pearl2009 — Causality
- [ ] lewis1973 — Counterfactuals
- [ ] saltelli2008 — Sensitivity Analysis
- [ ] wachter2017 — Counterfactual Explanations
- [ ] amodei2016 — Concrete Problems in AI Safety
- [ ] ardupilot, px4, rustlang — websites
- [ ] gnadt2022 — MagNav (opcional)

### Citações em falta (adiciona tu)
- [ ] SafeCert / BusbySim (procura "safety case pattern autonomous systems")
- [ ] Counterfactual reasoning for autonomous systems (procura "counterfactual safety UAV")
- [ ] EASA AI Roadmap (já está no .bib, adiciona o \cite no texto)

## Submissão

### CSR — IEEE Aerospace 2027
- URL: https://aeroastro.org/
- Deadline: ~Novembro 2026
- Formato: PDF (6-8 páginas)
- Requer: conta IEEE, cover letter, autores

### SCC — DASC 2027
- URL: https://dasc2027.org/
- Deadline: ~Junho 2027
- Formato: PDF (6-8 páginas)
- Requer: conta DASC, cover letter, autores

## O Que NÃO Podes Fazer Sem Mim

- Substituir-me como autor (tens de ser tu)
- Criar conta IEEE/DASC por ti
- Escrever a cover letter sem o teu contexto institucional

## O Que Está Pronto

- ✅ Abstract completo
- ✅ Introduction completa
- ✅ 7 secções de body (Background, Design, Implementation, Evaluation, Discussion, Conclusion)
- ✅ 15+ citações BibTeX
- ✅ 2 figuras TikZ por paper
- ✅ main.tex integrado (input dos ficheiros)
- ✅ Limitações documentadas honestamente
