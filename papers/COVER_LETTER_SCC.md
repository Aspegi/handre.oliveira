# Cover Letter — SCC (DASC 2027)

**Para:** DASC 2027 Program Committee
**De:** Andre Oliveira <handre.oliveira@gmail.com>
**Assunto:** Submission — SCC: A Safety Case Compiler for Autonomous Drones

---

Dear Program Committee,

I am submitting the paper "SCC: A Safety Case Compiler for Autonomous
Drones" for consideration at DASC 2027.

The paper addresses a practical problem in UAV certification: safety cases
are static documents, disconnected from the runtime behaviour they are
meant to assure. When requirements change, the safety case is manually
updated — or, more commonly, not updated at all.

SCC compiles safety requirements into three artefacts per requirement:
a Rust test stub, a GSN node, and a hash chain block. One source of truth
(the requirements file) generates all three, so there is no possibility
of the test checking a different value than the GSN node. A single-line
change regenerates all artefacts and re-validates the chain.

The paper is honest about its limitations: only 3 requirement types, no
quantifiers, synthetic physics. We see SCC as a first step toward
"compiled safety cases", not a complete solution.

The code is open-source at https://github.com/Aspegi/handre.oliveira
(19 crates, 277 tests, DO-326A compliance package).

I believe the paper fits DASC's scope on aviation software, certification,
and safety-critical systems. I am happy to address any questions.

Sincerely,
Andre Oliveira
RNFC Project
handre.oliveira@gmail.com

---

## Notas para ti (apagar antes de submeter)

- [ ] Adicionar afiliação institucional
- [ ] Adicionar ORCID
- [ ] Confirmar email
- [ ] Verificar PDF
- [ ] Verificar limite de páginas
- [ ] Verificar citações
