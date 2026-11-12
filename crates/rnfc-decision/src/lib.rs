//! Decision — multi-objective route selection with rationale.
//!
//! The drone generates N candidate routes, scores each with a weighted
//! cost function (distance + wind + obstacle + battery), picks the best,
//! and explains why. The decision can be recorded in a CertChain.
//!
//! Note: this is deterministic ranking, not ML. It does not predict
//! weather; wind is estimated from current state, not forecast.

#[allow(unused_imports)]
use rnfc_certchain::{CertChain, SafetyState};

#[derive(Clone, Debug)]
pub struct RouteCandidate {
    pub name: String,
    pub waypoints: Vec<[f32; 2]>, // includes start as first, target as last
    pub distance_m: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CostWeights {
    pub distance: f32,
    pub wind: f32,
    pub obstacle: f32,
    pub battery: f32,
}

impl Default for CostWeights {
    fn default() -> Self {
        Self {
            distance: 1.0,
            wind: 1.0,
            obstacle: 2.0,
            battery: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DecisionContext {
    pub wind: [f32; 2],
    pub obstacles: [([f32; 2], f32); 3],
    pub battery_pct: f32,
}

#[derive(Clone, Debug)]
pub struct ScoredRoute {
    pub name: String,
    pub cost: f32,
    pub distance_m: f32,
    pub wind_cost: f32,
    pub obstacle_cost: f32,
    pub battery_cost: f32,
}

#[derive(Clone, Debug)]
pub struct Decision {
    pub chosen: String,
    pub scores: Vec<ScoredRoute>,
    pub rationale: String,
}

pub struct DecisionEngine {
    pub weights: CostWeights,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self {
            weights: CostWeights::default(),
        }
    }

    pub fn generate_candidates(start: [f32; 2], target: [f32; 2]) -> Vec<RouteCandidate> {
        let mid = [(start[0] + target[0]) / 2.0, (start[1] + target[1]) / 2.0];
        let offset = 30.0;
        let direct_dist = dist(start, target);
        let detour_dist =
            dist(start, [mid[0], mid[1] + offset]) + dist([mid[0], mid[1] + offset], target);
        vec![
            RouteCandidate {
                name: "direct".into(),
                waypoints: vec![start, target],
                distance_m: direct_dist,
            },
            RouteCandidate {
                name: "north_detour".into(),
                waypoints: vec![start, [mid[0], mid[1] + offset], target],
                distance_m: detour_dist,
            },
            RouteCandidate {
                name: "south_detour".into(),
                waypoints: vec![start, [mid[0], mid[1] - offset], target],
                distance_m: detour_dist,
            },
        ]
    }

    pub fn score(&self, route: &RouteCandidate, ctx: &DecisionContext) -> ScoredRoute {
        let wind_mag = (ctx.wind[0] * ctx.wind[0] + ctx.wind[1] * ctx.wind[1]).sqrt();
        let wind_cost = wind_mag * route.distance_m * 0.1 * self.weights.wind;

        let min_clear = min_clearance(route, &ctx.obstacles);
        let obstacle_cost = if min_clear < 5.0 {
            (5.0 - min_clear) * 10.0 * self.weights.obstacle
        } else {
            0.0
        };

        let urgency = if ctx.battery_pct < 50.0 {
            (50.0 - ctx.battery_pct) / 50.0
        } else {
            0.0
        };
        let battery_cost = route.distance_m * urgency * 0.05 * self.weights.battery;

        let total =
            route.distance_m * self.weights.distance + wind_cost + obstacle_cost + battery_cost;
        ScoredRoute {
            name: route.name.clone(),
            cost: total,
            distance_m: route.distance_m,
            wind_cost,
            obstacle_cost,
            battery_cost,
        }
    }

    pub fn decide(&self, start: [f32; 2], target: [f32; 2], ctx: &DecisionContext) -> Decision {
        let candidates = Self::generate_candidates(start, target);
        let mut scores: Vec<ScoredRoute> = candidates.iter().map(|r| self.score(r, ctx)).collect();
        scores.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let chosen = scores[0].clone();
        let runner = scores.get(1).cloned();
        let mut r = String::new();
        r.push_str(&format!(
            "Chose {} (cost {:.2}). ",
            chosen.name, chosen.cost
        ));
        if let Some(ru) = runner {
            let pct = ((ru.cost / chosen.cost.max(0.01)) - 1.0) * 100.0;
            r.push_str(&format!(
                "Runner-up {} cost {:.2} (+{:.1}%). ",
                ru.name, ru.cost, pct
            ));
        }
        if chosen.wind_cost > 1.0 {
            r.push_str(&format!("Wind adds {:.1}. ", chosen.wind_cost));
        }
        if chosen.obstacle_cost > 0.0 {
            r.push_str(&format!(
                "Obstacle proximity adds {:.1}. ",
                chosen.obstacle_cost
            ));
        }
        if chosen.battery_cost > 0.0 {
            r.push_str(&format!("Battery urgency adds {:.1}.", chosen.battery_cost));
        }
        Decision {
            chosen: chosen.name,
            scores,
            rationale: r,
        }
    }

    pub fn prove(&self, chain: &CertChain, d: &Decision) -> u64 {
        let mut buf = Vec::new();
        buf.extend_from_slice(&chain.root_hash().to_le_bytes());
        buf.extend_from_slice(d.chosen.as_bytes());
        buf.extend_from_slice(d.rationale.as_bytes());
        for s in &d.scores {
            buf.extend_from_slice(s.name.as_bytes());
            buf.extend_from_slice(&s.cost.to_le_bytes());
        }
        fnv1a_64(&buf)
    }
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn min_clearance(route: &RouteCandidate, obs: &[([f32; 2], f32); 3]) -> f32 {
    let mut min = f32::INFINITY;
    for w in route.waypoints.windows(2) {
        let a = w[0];
        let b = w[1];
        for t in 0..=10 {
            let f = t as f32 / 10.0;
            let p = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
            for (op, rad) in obs {
                let d = dist(p, *op) - rad;
                if d < min {
                    min = d;
                }
            }
        }
    }
    min
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calm() -> DecisionContext {
        DecisionContext {
            wind: [0.0, 0.0],
            obstacles: [([1000.0, 1000.0], 1.0); 3],
            battery_pct: 100.0,
        }
    }

    fn make_chain() -> CertChain {
        let mut c = CertChain::new(1, "decision_test");
        c.append(1.0, 2, 0xAA, SafetyState::Nominal).unwrap();
        c.append(2.0, 1, 0xBB, SafetyState::Nominal).unwrap();
        c
    }

    #[test]
    fn direct_has_lowest_distance() {
        let c = DecisionEngine::generate_candidates([0.0, 0.0], [100.0, 0.0]);
        assert_eq!(c[0].name, "direct");
        assert!(c[0].distance_m < c[1].distance_m);
        assert!(c[0].distance_m < c[2].distance_m);
    }

    #[test]
    fn calm_chooses_direct() {
        let e = DecisionEngine::new();
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        assert_eq!(d.chosen, "direct");
    }

    #[test]
    fn wind_increases_cost() {
        let e = DecisionEngine::new();
        let route = &DecisionEngine::generate_candidates([0.0, 0.0], [100.0, 0.0])[0];
        let c = e.score(route, &calm());
        let mut w = calm();
        w.wind = [0.0, 15.0];
        let wc = e.score(route, &w);
        assert!(wc.cost > c.cost);
    }

    #[test]
    fn obstacle_penalizes_near_route() {
        let e = DecisionEngine::new();
        let route = &DecisionEngine::generate_candidates([0.0, 0.0], [100.0, 0.0])[0];
        let mut ctx = calm();
        ctx.obstacles = [([50.0, 0.0], 3.0); 3];
        let with_obs = e.score(route, &ctx);
        let no_obs = e.score(route, &calm());
        assert!(with_obs.cost > no_obs.cost);
        assert!(with_obs.obstacle_cost > 0.0);
    }

    #[test]
    fn obstacle_forces_detour() {
        let e = DecisionEngine::new();
        let mut ctx = calm();
        ctx.obstacles = [([50.0, 0.0], 8.0); 3];
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &ctx);
        assert_ne!(d.chosen, "direct", "should detour around obstacle");
        assert!(d.chosen.contains("detour"));
    }

    #[test]
    fn low_battery_increases_cost() {
        let e = DecisionEngine::new();
        let route = &DecisionEngine::generate_candidates([0.0, 0.0], [100.0, 0.0])[0];
        let full = e.score(route, &calm());
        let mut low = calm();
        low.battery_pct = 20.0;
        let low_s = e.score(route, &low);
        assert!(low_s.cost > full.cost);
    }

    #[test]
    fn scores_sorted_ascending() {
        let e = DecisionEngine::new();
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        for i in 1..d.scores.len() {
            assert!(d.scores[i].cost >= d.scores[i - 1].cost);
        }
    }

    #[test]
    fn rationale_mentions_chosen() {
        let e = DecisionEngine::new();
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        assert!(d.rationale.contains(&d.chosen));
        assert!(!d.rationale.is_empty());
    }

    #[test]
    fn decision_deterministic() {
        let e = DecisionEngine::new();
        let d1 = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        let d2 = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        assert_eq!(d1.chosen, d2.chosen);
        assert_eq!(d1.scores[0].cost, d2.scores[0].cost);
    }

    #[test]
    fn proof_deterministic() {
        let e = DecisionEngine::new();
        let chain = make_chain();
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        assert_eq!(e.prove(&chain, &d), e.prove(&chain, &d));
    }

    #[test]
    fn different_chains_different_proofs() {
        let e = DecisionEngine::new();
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        let c1 = make_chain();
        let mut c2 = CertChain::new(2, "other");
        c2.append(1.0, 1, 0xFF, SafetyState::Nominal).unwrap();
        assert_ne!(e.prove(&c1, &d), e.prove(&c2, &d));
    }

    #[test]
    fn decision_recorded_in_chain() {
        let e = DecisionEngine::new();
        let mut chain = make_chain();
        let d = e.decide([0.0, 0.0], [100.0, 0.0], &calm());
        let proof = e.prove(&chain, &d);
        let _ = chain.append(3.0, 1, proof, SafetyState::Nominal);
        assert!(chain.verify());
    }
}
