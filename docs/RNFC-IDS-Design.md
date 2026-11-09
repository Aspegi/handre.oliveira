\# RNFC — Intrusion Detection System (IDS) Design (DO-326A)



\*\*Projeto:\*\* RNFC — Resilient Navigation \& Flight Control

\*\*Autor:\*\* André Oliveira

\*\*Copyright:\*\* (c) 2027 André Oliveira — Todos os direitos reservados

\*\*Versão:\*\* 1.0

\*\*Referência:\*\* DO-326A / ED-202A — Artefacto 5



\---



\## 1. Objetivo



Este documento descreve o design da monitorização em operação do RNFC — o sistema que deteta, em tempo real, tráfego MAVLink anómalo ou não autorizado. É o último artefacto do DO-326A e fecha o ciclo: os artefactos 1-4 previnem; este deteta o que escapa à prevenção.



\*\*Princípio:\*\* o signing e o anti-replay bloqueiam a maioria dos ataques. O IDS cobre o risco residual — ataques que comprometeram a chave, flooding, ou comportamento anómalo de um peer legítimo.



\---



\## 2. Arquitetura



