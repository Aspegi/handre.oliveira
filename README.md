# RNFC — Resilient Navigation & Flight Control

Piloto automatico em Rust no_std com navegacao resiliente, DAA dinamico e visao MAVLink.

## Estado: v1.7.3-stable — COMPLETO

| Metrica | Valor |
|---|---|
| Testes | **87** (7 camadas de verificacao) |
| Precisao normal | 1.98m |
| Precisao adversarial (5000 evolved) | **2.00m** |
| Crashes em 6000 MC | **0** |
| Zombie orbit | **Eliminado** |
| Crates Cortex-M4F | 3 |
| CI | fmt + clippy + test + build + no_std |

## Como correr

cargo run -p rnfc-backend-sitl --release

## Testes

cargo test -p rnfc-core --release
cargo test -p rnfc-backend-sitl --test mission --release -- --nocapture

## Tags SITL

v0.4.0 a v0.8.0-sitl-complete


