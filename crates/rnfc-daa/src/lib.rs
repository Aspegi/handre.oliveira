// Copyright (c) 2027 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
pub mod trajectory;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Obstaculo representado por um segmento de reta (parede).
#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub n_start: f32,
    pub e_start: f32,
    pub n_end: f32,
    pub e_end: f32,
}

/// Simulacao de LiDAR: dispara raios e calcula distancia ao obstaculo mais proximo.
pub struct LidarSim {
    pub obstacles: Vec<Obstacle>,
    pub max_range: f32,
    pub n_rays: usize,
}

impl LidarSim {
    pub fn new(obstacles: Vec<Obstacle>, max_range: f32) -> Self {
        Self {
            obstacles,
            max_range,
            n_rays: 36,
        }
    }

    pub fn scan(&self, pos_n: f32, pos_e: f32) -> Vec<[f32; 2]> {
        let mut hits = Vec::new();
        for k in 0..self.n_rays {
            let angle = (k as f32) * (2.0 * std::f32::consts::PI / self.n_rays as f32);
            let dir_n = angle.cos();
            let dir_e = angle.sin();

            let mut min_dist = self.max_range;
            for obs in &self.obstacles {
                if let Some(t) = ray_segment_intersection(
                    pos_n,
                    pos_e,
                    dir_n,
                    dir_e,
                    obs.n_start,
                    obs.e_start,
                    obs.n_end,
                    obs.e_end,
                ) {
                    if t < min_dist && t > 0.1 {
                        min_dist = t;
                    }
                }
            }

            if min_dist < self.max_range {
                hits.push([pos_n + dir_n * min_dist, pos_e + dir_e * min_dist]);
            }
        }
        hits
    }
}

fn ray_segment_intersection(
    pn: f32,
    pe: f32,
    dn: f32,
    de: f32,
    an: f32,
    ae: f32,
    bn: f32,
    be: f32,
) -> Option<f32> {
    let seg_n = bn - an;
    let seg_e = be - ae;
    let det = dn * seg_e - de * seg_n;
    if det.abs() < 1e-6 {
        return None;
    }
    let t = ((an - pn) * seg_e - (ae - pe) * seg_n) / det;
    let s = ((an - pn) * de - (ae - pe) * dn) / det;
    if t > 0.0 && s >= 0.0 && s <= 1.0 {
        Some(t)
    } else {
        None
    }
}

/// Occupancy grid 2D em coordenadas NED.
pub struct OccupancyGrid {
    pub origin_n: f32,
    pub origin_e: f32,
    pub resolution: f32,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<bool>,
}

impl OccupancyGrid {
    pub fn new(origin_n: f32, origin_e: f32, width: usize, height: usize, resolution: f32) -> Self {
        Self {
            origin_n,
            origin_e,
            resolution,
            width,
            height,
            cells: vec![false; width * height],
        }
    }

    pub fn to_grid(&self, n: f32, e: f32) -> Option<(usize, usize)> {
        let i = ((n - self.origin_n) / self.resolution) as i32;
        let j = ((e - self.origin_e) / self.resolution) as i32;
        if i >= 0 && j >= 0 && (i as usize) < self.width && (j as usize) < self.height {
            Some((i as usize, j as usize))
        } else {
            None
        }
    }

    pub fn to_world(&self, i: usize, j: usize) -> [f32; 2] {
        [
            self.origin_n + (i as f32 + 0.5) * self.resolution,
            self.origin_e + (j as f32 + 0.5) * self.resolution,
        ]
    }

    pub fn mark_obstacle(&mut self, n: f32, e: f32) {
        if let Some((i, j)) = self.to_grid(n, e) {
            for di in -1..=1i32 {
                for dj in -1..=1i32 {
                    let ni = i as i32 + di;
                    let nj = j as i32 + dj;
                    if ni >= 0
                        && nj >= 0
                        && (ni as usize) < self.width
                        && (nj as usize) < self.height
                    {
                        self.cells[ni as usize * self.height + nj as usize] = true;
                    }
                }
            }
        }
    }

    pub fn is_free(&self, i: usize, j: usize) -> bool {
        !self.cells[i * self.height + j]
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Node {
    i: usize,
    j: usize,
    f: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn plan_astar(grid: &OccupancyGrid, start: [f32; 2], goal: [f32; 2]) -> Option<Vec<[f32; 2]>> {
    let (si, sj) = grid.to_grid(start[0], start[1])?;
    let (gi, gj) = grid.to_grid(goal[0], goal[1])?;

    let mut open: BinaryHeap<Node> = BinaryHeap::new();
    let mut g_score = vec![i32::MAX; grid.width * grid.height];
    let mut parent: Vec<Option<(usize, usize)>> = vec![None; grid.width * grid.height];
    let mut closed = vec![false; grid.width * grid.height];

    let idx = |i: usize, j: usize| i * grid.height + j;
    g_score[idx(si, sj)] = 0;
    open.push(Node {
        i: si,
        j: sj,
        f: heuristic(si, sj, gi, gj),
    });

    while let Some(Node { i, j, .. }) = open.pop() {
        if closed[idx(i, j)] {
            continue;
        }
        closed[idx(i, j)] = true;
        if i == gi && j == gj {
            break;
        }

        for (di, dj) in &[
            (-1i32, 0i32),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (-1, 1),
            (1, -1),
            (1, 1),
        ] {
            let ni = i as i32 + di;
            let nj = j as i32 + dj;
            if ni < 0 || nj < 0 {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            if ni >= grid.width || nj >= grid.height {
                continue;
            }
            if !grid.is_free(ni, nj) {
                continue;
            }

            let step = if *di != 0 && *dj != 0 { 14 } else { 10 };
            let tentative = g_score[idx(i, j)] + step;
            if tentative < g_score[idx(ni, nj)] {
                g_score[idx(ni, nj)] = tentative;
                parent[idx(ni, nj)] = Some((i, j));
                let f = tentative + heuristic(ni, nj, gi, gj);
                open.push(Node { i: ni, j: nj, f });
            }
        }
    }

    let mut path = Vec::new();
    let mut cur = (gi, gj);
    loop {
        path.push(grid.to_world(cur.0, cur.1));
        if cur == (si, sj) {
            break;
        }
        cur = parent[idx(cur.0, cur.1)]?;
    }
    path.reverse();

    let simplified: Vec<[f32; 2]> = path
        .iter()
        .enumerate()
        .filter(|(k, _)| k % 10 == 0 || *k == path.len() - 1)
        .map(|(_, p)| *p)
        .collect();

    Some(simplified)
}

fn heuristic(i: usize, j: usize, gi: usize, gj: usize) -> i32 {
    let di = (i as i32 - gi as i32).abs();
    let dj = (j as i32 - gj as i32).abs();
    (di + dj) * 10
}
