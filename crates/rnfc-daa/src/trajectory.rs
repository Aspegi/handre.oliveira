// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
//! Trajetoria suave (Catmull-Rom spline) com calculo de curvatura.

pub struct Trajectory {
    pub points: Vec<[f32; 2]>,
    pub cumulative_dist: Vec<f32>,
    pub total_length: f32,
}

impl Trajectory {
    pub fn from_waypoints(waypoints: &[[f32; 2]], step: f32) -> Self {
        let mut points = Vec::new();
        if waypoints.len() < 2 {
            return Self {
                points: waypoints.to_vec(),
                cumulative_dist: vec![0.0],
                total_length: 0.0,
            };
        }

        for i in 0..waypoints.len() - 1 {
            let p0 = if i == 0 {
                waypoints[0]
            } else {
                waypoints[i - 1]
            };
            let p1 = waypoints[i];
            let p2 = waypoints[i + 1];
            let p3 = if i + 2 < waypoints.len() {
                waypoints[i + 2]
            } else {
                waypoints[i + 1]
            };

            let seg_len = ((p2[0] - p1[0]).powi(2) + (p2[1] - p1[1]).powi(2)).sqrt();
            let steps = ((seg_len / step).ceil() as usize).max(1);

            for k in 0..steps {
                let t = k as f32 / steps as f32;
                points.push(catmull_rom(p0, p1, p2, p3, t));
            }
        }
        points.push(*waypoints.last().unwrap());

        let mut cum = vec![0.0];
        for i in 1..points.len() {
            let d = ((points[i][0] - points[i - 1][0]).powi(2)
                + (points[i][1] - points[i - 1][1]).powi(2))
            .sqrt();
            cum.push(cum[i - 1] + d);
        }
        let total = *cum.last().unwrap();

        Self {
            points,
            cumulative_dist: cum,
            total_length: total,
        }
    }

    pub fn point_at(&self, distance: f32) -> [f32; 2] {
        if distance <= 0.0 {
            return self.points[0];
        }
        if distance >= self.total_length {
            return *self.points.last().unwrap();
        }
        let idx = self.cumulative_dist.partition_point(|&d| d < distance);
        if idx == 0 {
            return self.points[0];
        }
        let i = idx - 1;
        let d0 = self.cumulative_dist[i];
        let d1 = self.cumulative_dist[idx];
        let t = if d1 > d0 {
            (distance - d0) / (d1 - d0)
        } else {
            0.0
        };
        [
            self.points[i][0] + (self.points[idx][0] - self.points[i][0]) * t,
            self.points[i][1] + (self.points[idx][1] - self.points[i][1]) * t,
        ]
    }

    /// Curvatura via Menger: k = 4A / (abc) usando 3 pontos em torno de `distance`.
    pub fn curvature_at(&self, distance: f32) -> f32 {
        let d = 2.0;
        let p0 = self.point_at((distance - d).max(0.0));
        let p1 = self.point_at(distance);
        let p2 = self.point_at((distance + d).min(self.total_length));
        let a = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
        let b = ((p2[0] - p1[0]).powi(2) + (p2[1] - p1[1]).powi(2)).sqrt();
        let c = ((p2[0] - p0[0]).powi(2) + (p2[1] - p0[1]).powi(2)).sqrt();
        let area =
            0.5 * ((p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1])).abs();
        if a * b * c < 1e-6 {
            0.0
        } else {
            4.0 * area / (a * b * c)
        }
    }

    pub fn find_closest(&self, pos: [f32; 2]) -> (usize, f32) {
        let mut best_i = 0;
        let mut best_d = f32::INFINITY;
        for (i, p) in self.points.iter().enumerate() {
            let d = (p[0] - pos[0]).powi(2) + (p[1] - pos[1]).powi(2);
            if d < best_d {
                best_d = d;
                best_i = i;
            }
        }
        (best_i, self.cumulative_dist[best_i])
    }
}

fn catmull_rom(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let t2 = t * t;
    let t3 = t2 * t;
    let a = -0.5 * t3 + t2 - 0.5 * t;
    let b = 1.5 * t3 - 2.5 * t2 + 1.0;
    let c = -1.5 * t3 + 2.0 * t2 + 0.5 * t;
    let d = 0.5 * t3 - 0.5 * t2;
    [
        a * p0[0] + b * p1[0] + c * p2[0] + d * p3[0],
        a * p0[1] + b * p1[1] + c * p2[1] + d * p3[1],
    ]
}
