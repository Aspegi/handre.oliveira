# Startup Package — RNFC Certification-as-a-Service

## Pitch Deck (10 slides)

### Slide 1: Título
**RNFC — Certificação Criptográfica para Drones BVLOS**
Andre Oliveira, Founder & CTO

### Slide 2: Problema
- Reguladores (EASA, FAA) exigem safety cases, mas são estáticos
- Seguradoras não confiam em logs não verificáveis
- Fabricantes pequenos não têm equipa de certificação
- Mercado BVLOS: $500M+ em 2027

### Slide 3: Solução
- **RNFC CertChain**: hash chain de eventos de segurança
- **RNFC SCC**: compilador de safety case (requisitos → prova)
- **RNFC CSR**: auditoria contrafactual ("e se o vento fosse 2×?")
- **RNFC MagNav**: navegação GPS-denied certificável

### Slide 4: Produto
- **CaaS (Certification-as-a-Service)**: €50k-€200k/ano por fabricante
- **API de verificação**: regulador verifica em segundos
- **Dashboard**: seguradora vê histórico de missões

### Slide 5: Mercado
- TAM: $500M+ (certificação BVLOS, 2027)
- SAM: $50M+ (fabricantes de drones BVLOS)
- SOM: $5M+ (10 fabricantes × €100k + 3 seguradoras × €200k)

### Slide 6: Tração
- 277 testes, 19 crates (open-source)
- 2 papers compilados (SCC, CSR)
- DO-326A PSecAC draft completo
- 95 tags, CI/CD com 6 jobs

### Slide 7: Modelo de Negócio
- €50k-€200k/ano por fabricante (CaaS)
- 5-10% do prémio de seguro (seguradoras)
- Piloto EASA/FAA (reguladores)

### Slide 8: Competição
| Competidor | O Que Faz | O Que Falta |
|---|---|---|
| SafeCert / BusbySim | Safety case estático | Prova contínua |
| ArduPilot / PX4 | Flight controller | Certificação |
| Skydio | Hardware + percepção | Open-source, certificação |
| **RNFC** | **Stack completa + certificação** | **Hardware** |

### Slide 9: Equipa
- **Andre Oliveira (CTO)**: 277 testes, 19 crates, 2 papers
- **(a contratar) CEO**: experiência em vendas B2B/defesa
- **(a contratar) Auditor**: segurança de sistemas

### Slide 10: Ask
- **€500k seed** (18 meses runway)
- **Uso**: HAL Pixhawk (€100k), auditoria (€50k), vendas (€150k),
  hardware (€50k), papers (€50k), operações (€100k)
- **Milestones**: v7.0.0 com HAL (6 meses), 3 clientes piloto (12 meses)

## Roadmap de Hardware (8-12 semanas)

| Semana | Tarefa |
|---|---|
| 1 | Comprar Pixhawk 4 + ST-LINK (~€110) |
| 2-4 | Implementar rnfc-hal-pixhawk para STM32F765 (SPI IMU, I2C MAG, UART GNSS, PWM) |
| 5-6 | Testar em bancada (PWM com osciloscópio) |
| 7-8 | Testar em voo (hover, waypoint único) |
| 9-10 | Secure Boot (STM32F4 + SBSFU) |
| 11-12 | Integração final + release v7.0.0 |

## Co-Founder Search

### Perfil Procurado
- **CEO**: experiência em vendas B2B (defesa, aviação, seguros)
- **Rede**: contactos em EASA, FAA, seguradoras, fabricantes de drones
- **Localização**: Europa (Portugal, Reino Unido, Alemanha)

### Onde Procurar
- **HN post**: "Looking for co-founder" (após o show HN)
- **LinkedIn**: grupos de drones, defesa, aviação
- **Y Combinator co-founder matching**: https://www.startupschool.org/cofounder-matching
- **Reddit r/drones, r/startups**: posts honestos

### Pitch Para Co-Founder
"Tenho uma stack open-source de certificação de drones com 277 testes,
2 papers, e DO-326A compliance. Preciso de um CEO que saiba vender para
defesa/seguradoras. O código está feito; o mercado está por abrir."

## Próximos Passos

1. Comprar Pixhawk (€110) — 1 dia
2. Completar HAL (8-12 semanas)
3. Publicar no HN (visibilidade para co-founder)
4. Procurar co-founder (LinkedIn, YC, Reddit)
5. Primeiros clientes piloto (fabricantes pequenos)
6. Pitch a investidores (após 3 clientes)
