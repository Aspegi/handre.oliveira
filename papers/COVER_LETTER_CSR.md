# Cover Letter — CSR (IEEE Aerospace 2027)

**Para:** IEEE Aerospace Conference 2027 Program Committee
**De:** Andre Oliveira <handre.oliveira@gmail.com>
**Assunto:** Submission — Counterfactual Safety Reasoning: Post-Mission What-If Proofs for Autonomous Drones

---

Dear Program Committee,

I am submitting the paper "Counterfactual Safety Reasoning: Post-Mission
What-If Proofs for Autonomous Drones" for consideration at IEEE Aerospace
2027.

The paper addresses a gap in the certification of autonomous drones:
existing safety cases and black-box logs record what happened, but not
what would have happened under different conditions. Regulators and
insurers need to answer counterfactual questions ("would the drone have
failed if the wind had been 2x?") with verifiable evidence, not operator
trust.

CSR enables a drone to answer these questions with cryptographic proofs
bound to the original mission chain. The key contribution is the
combination of trajectory replay, counterfactual classification, and
SHA-256 proof construction in a single system. On a 4-scenario case study,
4/4 proofs verified and 3/3 attacks (tampered chain, tampered result,
wrong key) were rejected.

The paper is honest about its limitations: the physics is synthetic (1D/2D),
the chain uses FNV-1a (not full SHA-256), and there are no formal
guarantees. We see CSR as a proof of concept, not a certified tool.

The code is open-source at https://github.com/Aspegi/handre.oliveira
(19 crates, 277 tests, DO-326A compliance package).

I believe the paper fits IEEE Aerospace's scope on autonomous systems,
safety certification, and post-mission analysis. I am happy to address
any questions from the committee.

Sincerely,
Andre Oliveira
RNFC Project
handre.oliveira@gmail.com

---

## Notas para ti (apagar antes de submeter)

- [ ] Adicionar afiliação institucional (se tiveres)
- [ ] Adicionar ORCID (se tiveres)
- [ ] Confirmar que o email está correcto
- [ ] Verificar que o PDF compila sem erros
- [ ] Verificar o limite de páginas (6-8)
- [ ] Verificar que todas as citações estão no .bib
- [ ] Se tiveres co-autores, adicioná-los no portal
