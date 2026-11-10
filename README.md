# RNFC — Resilient Navigation & Flight Control

Piloto automatico em Rust no_std com navegacao resiliente, DAA dinamico e visao MAVLink.

## Estado: v0.8.0-sitl-complete

- 21 testes verdes (20 unit + 1 integration)
- Precisao final: 0.48m (sem vento) / 2.00m (com vento + ataque)
- Missao: A(100,50) -> B(150,-30) -> C(0,0)
- Resiliência: vento, gyro bias, mag spoofing, obstaculos desconhecidos

## Como correr

cargo run -p rnfc-backend-sitl --release

## Testes

cargo test -p rnfc-core --release
cargo test -p rnfc-backend-sitl --test mission --release -- --nocapture

## Tags SITL

v0.4.0 a v0.8.0-sitl-complete
