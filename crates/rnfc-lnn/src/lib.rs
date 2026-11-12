#![cfg_attr(not(test), no_std)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::new_without_default)]
//! Liquid Neural Network (LNN) for Fault-Tolerant Control.
//!
//! Implementacao `no_std` via const generics (tamanho fixo em compile-time).
//! Forward pass com Euler ODE solver.
//!
//! Referencia: Hasani et al., "Liquid Time-constant Networks", AAAI 2021.

use libm::tanhf;

/// Uma LNN com `IN` inputs, `HID` neuronios liquidos, `OUT` outputs.
/// Todos os pesos armazenados em arrays fixos (no heap, no Vec).
#[derive(Clone, Copy)]
pub struct LiquidNetwork<const IN: usize, const HID: usize, const OUT: usize> {
    /// Pesos input -> hidden: w_in[hidden][input]
    pub w_in: [[f32; IN]; HID],
    /// Pesos recurrentes: w_rec[hidden][hidden]
    pub w_rec: [[f32; HID]; HID],
    /// Bias dos neuronios hidden
    pub b_hidden: [f32; HID],
    /// Pesos hidden -> output: w_out[output][hidden]
    pub w_out: [[f32; HID]; OUT],
    /// Bias dos outputs
    pub b_out: [f32; OUT],
    /// Constantes de tempo (aprendidas ou fixas)
    pub tau: [f32; HID],
    /// Estado interno (mutavel em forward_step)
    pub state: [f32; HID],
}

impl<const IN: usize, const HID: usize, const OUT: usize> LiquidNetwork<IN, HID, OUT> {
    /// Cria LNN com pesos zero (para ser substituido por treino ou loaded weights).
    pub fn zeros() -> Self {
        Self {
            w_in: [[0.0; IN]; HID],
            w_rec: [[0.0; HID]; HID],
            b_hidden: [0.0; HID],
            w_out: [[0.0; HID]; OUT],
            b_out: [0.0; OUT],
            tau: [1.0; HID],
            state: [0.0; HID],
        }
    }

    /// Reset do estado interno (usar entre episodios).
    pub fn reset(&mut self) {
        self.state = [0.0; HID];
    }

    /// Um passo de forward (ODE Euler). Actualiza `state` e devolve `output`.
    /// `dt` = passo temporal. Deve ser ~0.01s (100Hz).
    pub fn forward_step(&mut self, input: [f32; IN], dt: f32) -> [f32; OUT] {
        // 1. Computar derivada do estado
        let mut dx = [0.0_f32; HID];
        for i in 0..HID {
            // Entrada: w_in · input + b_hidden
            let mut pre_activation = self.b_hidden[i];
            for j in 0..IN {
                pre_activation += self.w_in[i][j] * input[j];
            }
            // Recurrente: w_rec · state
            for j in 0..HID {
                pre_activation += self.w_rec[i][j] * self.state[j];
            }
            // ODE: dx/dt = -state/tau + tanh(pre_activation)
            let tau_safe = if self.tau[i].abs() < 0.01 {
                0.01
            } else {
                self.tau[i]
            };
            dx[i] = -self.state[i] / tau_safe + tanhf(pre_activation);
        }

        // 2. Euler step
        for i in 0..HID {
            self.state[i] += dx[i] * dt;
            // Clamp para estabilidade numerica
            if self.state[i] > 10.0 {
                self.state[i] = 10.0;
            }
            if self.state[i] < -10.0 {
                self.state[i] = -10.0;
            }
        }

        // 3. Output: w_out · state + b_out
        let mut output = [0.0_f32; OUT];
        for k in 0..OUT {
            let mut out = self.b_out[k];
            for j in 0..HID {
                out += self.w_out[k][j] * self.state[j];
            }
            output[k] = out;
        }
        output
    }

    /// Total de parametros treinaveis.
    pub fn param_count() -> usize {
        IN * HID              // w_in
            + HID * HID       // w_rec
            + HID             // b_hidden
            + HID * OUT       // w_out
            + OUT             // b_out
            + HID // tau
    }

    /// Flatten para vector (util para evolucao genetica).
    pub fn to_vec(&self) -> [f32; 1024] {
        // Buffer grande o suficiente para IN,HID,OUT ate 8x8x8
        let mut v = [0.0_f32; 1024];
        let mut idx = 0;
        for i in 0..HID {
            for j in 0..IN {
                v[idx] = self.w_in[i][j];
                idx += 1;
            }
        }
        for i in 0..HID {
            for j in 0..HID {
                v[idx] = self.w_rec[i][j];
                idx += 1;
            }
        }
        for i in 0..HID {
            v[idx] = self.b_hidden[i];
            idx += 1;
        }
        for k in 0..OUT {
            for j in 0..HID {
                v[idx] = self.w_out[k][j];
                idx += 1;
            }
        }
        for k in 0..OUT {
            v[idx] = self.b_out[k];
            idx += 1;
        }
        for i in 0..HID {
            v[idx] = self.tau[i];
            idx += 1;
        }
        v
    }

    /// Carregar de vector (apos evolucao genetica).
    pub fn from_vec(&mut self, v: &[f32]) {
        let mut idx = 0;
        for i in 0..HID {
            for j in 0..IN {
                self.w_in[i][j] = v[idx];
                idx += 1;
            }
        }
        for i in 0..HID {
            for j in 0..HID {
                self.w_rec[i][j] = v[idx];
                idx += 1;
            }
        }
        for i in 0..HID {
            self.b_hidden[i] = v[idx];
            idx += 1;
        }
        for k in 0..OUT {
            for j in 0..HID {
                self.w_out[k][j] = v[idx];
                idx += 1;
            }
        }
        for k in 0..OUT {
            self.b_out[k] = v[idx];
            idx += 1;
        }
        for i in 0..HID {
            self.tau[i] = v[idx].abs().max(0.05); // tau positivo
            idx += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_step_deterministic() {
        let mut net1 = LiquidNetwork::<5, 8, 2>::zeros();
        let mut net2 = LiquidNetwork::<5, 8, 2>::zeros();
        // Pesos conhecidos
        net1.w_in[0][0] = 0.5;
        net2.w_in[0][0] = 0.5;
        let input = [0.1, 0.2, 0.3, 0.4, 0.5];
        let out1 = net1.forward_step(input, 0.01);
        let out2 = net2.forward_step(input, 0.01);
        for k in 0..2 {
            assert!((out1[k] - out2[k]).abs() < 1e-6, "determinismo quebrado");
        }
    }

    #[test]
    fn forward_step_no_nan_extreme() {
        let mut net = LiquidNetwork::<5, 8, 2>::zeros();
        for i in 0..8 {
            net.tau[i] = 0.5;
        }
        // Inputs extremos
        let cases = [
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [1e6, 1e6, 1e6, 1e6, 1e6],
            [-1e6, -1e6, -1e6, -1e6, -1e6],
            [1e-10, -1e-10, 0.0, 1.0, -1.0],
        ];
        for input in cases {
            let _ = net.forward_step(input, 0.01);
            for i in 0..8 {
                assert!(net.state[i].is_finite(), "state[{}] NaN/Inf", i);
            }
        }
    }

    #[test]
    fn forward_step_state_evolves() {
        let mut net = LiquidNetwork::<5, 8, 2>::zeros();
        net.w_in[0][0] = 1.0; // neuronio 0 responde a input[0]
        let input = [1.0, 0.0, 0.0, 0.0, 0.0];
        let initial_state = net.state[0];
        for _ in 0..50 {
            net.forward_step(input, 0.01);
        }
        assert!(
            (net.state[0] - initial_state).abs() > 0.01,
            "state nao evoluiu: {:.4} -> {:.4}",
            initial_state,
            net.state[0]
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut net = LiquidNetwork::<5, 8, 2>::zeros();
        net.w_in[0][0] = 1.0;
        for _ in 0..20 {
            net.forward_step([1.0, 0.0, 0.0, 0.0, 0.0], 0.01);
        }
        assert!(net.state[0].abs() > 0.01, "state deveria ser nao-zero");
        net.reset();
        for i in 0..8 {
            assert_eq!(net.state[i], 0.0);
        }
    }

    #[test]
    fn roundtrip_weights() {
        let mut net = LiquidNetwork::<5, 8, 2>::zeros();
        net.w_in[0][0] = 0.5;
        net.w_rec[1][1] = -0.3;
        net.b_hidden[2] = 0.7;
        net.w_out[0][0] = 1.2;
        net.b_out[1] = -0.4;
        net.tau[3] = 2.0;

        let v = net.to_vec();
        let mut net2 = LiquidNetwork::<5, 8, 2>::zeros();
        net2.from_vec(&v);

        assert!((net2.w_in[0][0] - 0.5).abs() < 1e-6);
        assert!((net2.w_rec[1][1] - (-0.3)).abs() < 1e-6);
        assert!((net2.b_hidden[2] - 0.7).abs() < 1e-6);
        assert!((net2.w_out[0][0] - 1.2).abs() < 1e-6);
        assert!((net2.b_out[1] - (-0.4)).abs() < 1e-6);
        assert!((net2.tau[3] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn param_count_correct() {
        // 5*8 + 8*8 + 8 + 8*2 + 2 + 8 = 40+64+8+16+2+8 = 138
        assert_eq!(LiquidNetwork::<5, 8, 2>::param_count(), 138);
    }
}
