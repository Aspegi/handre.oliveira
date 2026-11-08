#![deny(unsafe_code)]

use libm::{cosf, sinf};

/// Roda um vetor do referencial do corpo (body) para o referencial NED.
/// Convencao ZYX: R = Rz(yaw) * Ry(pitch) * Rx(roll)
pub fn body_to_ned(v: [f32; 3], roll: f32, pitch: f32, yaw: f32) -> [f32; 3] {
    let sr = sinf(roll);
    let cr = cosf(roll);
    let sp = sinf(pitch);
    let cp = cosf(pitch);
    let sy = sinf(yaw);
    let cy = cosf(yaw);

    [
        cy * cp * v[0] + (cy * sp * sr - sy * cr) * v[1] + (cy * sp * cr + sy * sr) * v[2],
        sy * cp * v[0] + (sy * sp * sr + cy * cr) * v[1] + (sy * sp * cr - cy * sr) * v[2],
        -sp * v[0] + cp * sr * v[1] + cp * cr * v[2],
    ]
}
