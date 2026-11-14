//! Math abstractions: libm in production, pure Rust for Kani verification.
//! Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]

/// Normaliza um angulo para o intervalo [-pi, pi] SEM loops.
pub fn normalize_angle(a: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let two_pi = core::f32::consts::TAU;

    if !a.is_finite() {
        return 0.0;
    }

    let mut r = a;
    if r > pi {
        r -= two_pi;
    }
    if r < -pi {
        r += two_pi;
    }
    if r > pi {
        r -= two_pi;
    }
    if r < -pi {
        r += two_pi;
    }

    // Clamp final de seguranca (garante [-pi, pi] mesmo apos dobras)
    if r > pi {
        pi
    } else if r < -pi {
        -pi
    } else {
        r
    }
}

#[cfg(kani)]
pub fn sqrtf(x: f32) -> f32 {
    // Versao simplificada para Kani: 3 iteracoes de Newton-Raphson.
    if !x.is_finite() || x <= 0.0 {
        return 0.0;
    }
    let mut y = x * 0.5 + 0.5;
    let mut i: u32 = 0;
    while i < 3 {
        y = 0.5 * (y + x / y);
        i += 1;
    }
    if !y.is_finite() {
        0.0
    } else {
        y
    }
}

#[cfg(kani)]
pub fn atan2f(y: f32, x: f32) -> f32 {
    // Aproximacao para Kani: SEMPRE bounded em [-pi, pi] por construcao.
    // Nao precisa de precisao - so precisa de ser finito e bounded.
    let pi = core::f32::consts::PI;

    if !y.is_finite() || !x.is_finite() {
        return 0.0;
    }
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }

    let angle = if x > 0.0 {
        // Q1/Q4: ratio clampado a [-1, 1] ANTES de multiplicar
        let ratio = if y.abs() < x {
            y / x
        } else if y >= 0.0 {
            1.0
        } else {
            -1.0
        };
        ratio * (pi * 0.25)
    } else if x < 0.0 {
        // Q2/Q3: ratio clampado a [0, 1] ANTES de multiplicar
        let ratio = (y / x).abs().min(1.0);
        if y >= 0.0 {
            pi * 0.75 + ratio * (pi * 0.25)
        } else {
            -pi * 0.75 - ratio * (pi * 0.25)
        }
    } else {
        if y > 0.0 {
            pi * 0.5
        } else {
            -pi * 0.5
        }
    };

    // Clamp final explicito (garante [-pi, pi] por construcao)
    if angle > pi {
        pi
    } else if angle < -pi {
        -pi
    } else {
        angle
    }
}

#[cfg(not(kani))]
pub use libm::{atan2f, sqrtf};
