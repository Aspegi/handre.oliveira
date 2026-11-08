#![deny(unsafe_code)]

/// Filtro de Kalman 3-eixos (NED): posição e velocidade por eixo.
pub struct Kalman3D {
    pub x: [[f32; 2]; 3],
    pub p: [[[f32; 2]; 2]; 3],
    q_pos: f32,
    q_vel: f32,
    r_pos: f32,
}

impl Kalman3D {
    pub fn new(pos: [f32; 3], vel: [f32; 3]) -> Self {
        let mut x = [[0.0_f32; 2]; 3];
        for i in 0..3 {
            x[i][0] = pos[i];
            x[i][1] = vel[i];
        }

        let mut p = [[[0.0_f32; 2]; 2]; 3];
        for i in 0..3 {
            p[i] = [[1.0, 0.0], [0.0, 1.0]];
        }

        Self {
            x,
            p,
            q_pos: 0.01,
            q_vel: 0.01,
            r_pos: 1.0,
        }
    }

    pub fn predict(&mut self, accel: [f32; 3], dt: f32) {
        for i in 0..3 {
            self.x[i][0] += self.x[i][1] * dt + 0.5 * accel[i] * dt * dt;
            self.x[i][1] += accel[i] * dt;

            let p = self.p[i];
            let dt2 = dt * dt;
            self.p[i][0][0] = p[0][0] + 2.0 * dt * p[0][1] + dt2 * p[1][1] + self.q_pos;
            self.p[i][0][1] = p[0][1] + dt * p[1][1];
            self.p[i][1][0] = self.p[i][0][1];
            self.p[i][1][1] = p[1][1] + self.q_vel;
        }
    }

    pub fn update_position(&mut self, z: [f32; 3]) {
        for i in 0..3 {
            let s = self.p[i][0][0] + self.r_pos;
            let k0 = self.p[i][0][0] / s;
            let k1 = self.p[i][1][0] / s;
            let y = z[i] - self.x[i][0];

            self.x[i][0] += k0 * y;
            self.x[i][1] += k1 * y;

            let p = self.p[i];
            self.p[i][0][0] = (1.0 - k0) * p[0][0];
            self.p[i][0][1] = (1.0 - k0) * p[0][1];
            self.p[i][1][0] = self.p[i][0][1];
            self.p[i][1][1] = p[1][1] - k1 * p[0][1];
        }
    }
}
