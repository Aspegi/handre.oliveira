//! Geodetic <-> NED conversion.
//! Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]

use nav_types::{NED, WGS84};

/// Converte um waypoint geodesico (lat, lon, alt) para NED,
/// usando `origin` como referencia (posicao atual do drone).
pub fn geodetic_to_ned(origin: WGS84<f32>, target: WGS84<f32>) -> NED<f32> {
    // A subtracao de duas posicoes WGS84 produz um vetor ENU.
    let vec_enu = target - origin;

    // ENU -> NED: trocar North<->East e inverter o eixo Up para Down.
    NED::new(vec_enu.north(), vec_enu.east(), -vec_enu.up())
}

/// Cria uma posicao WGS84 a partir de graus e metros.
pub fn wgs84_from_degrees(lat: f32, lon: f32, alt: f32) -> WGS84<f32> {
    WGS84::from_degrees_and_meters(lat, lon, alt)
}
