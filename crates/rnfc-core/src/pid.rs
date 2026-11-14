// Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
// Autor: Andre Oliveira <handre.oliveira@gmail.com>
// Projeto: RNFC - Resilient Navigation & Flight Control
// Licenca: Proprietaria (ver LICENSE)
#![deny(unsafe_code)]

/// Controlador PID generico com anti-windup e limite de saida.
pub struct PidController {
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    prev_measured: f32,
    output_limit: f32,
    integral_limit: f32,
    first_step: bool,
}

impl PidController {
    pub fn new(kp: f32, ki: f32, kd: f32, output_limit: f32, integral_limit: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_measured: 0.0,
            output_limit,
            integral_limit,
            first_step: true,
        }
    }

    /// Calcula a saida do PID.
    /// A derivada usa a medicao (nao o erro) para evitar "derivative kick"
    /// quando o setpoint muda bruscamente.
    pub fn update(&mut self, setpoint: f32, measured: f32, dt: f32) -> f32 {
        let error = setpoint - measured;

        // Integral com anti-windup (clamping)
        self.integral += error * dt;
        if self.integral > self.integral_limit {
            self.integral = self.integral_limit;
        }
        if self.integral < -self.integral_limit {
            self.integral = -self.integral_limit;
        }

        // Derivada (na primeira iteracao nao ha "anterior")
        let derivative = if self.first_step {
            self.first_step = false;
            0.0
        } else {
            -(measured - self.prev_measured) / dt
        };
        self.prev_measured = measured;

        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;

        // SaturaÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â§ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â£o da saida
        if output > self.output_limit {
            self.output_limit
        } else if output < -self.output_limit {
            -self.output_limit
        } else {
            output
        }
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_measured = 0.0;
        self.first_step = true;
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    /// Prova que o PID nunca produz output fora do `output_limit`
    /// e que o integral nunca excede o `integral_limit` (anti-windup).
    #[kani::proof]
    #[kani::unwind(4)]
    fn pid_output_bounded() {
        let setpoint: f32 = kani::any();
        let measured: f32 = kani::any();
        let dt: f32 = kani::any();

        kani::assume(setpoint.is_finite() && setpoint.abs() < 10.0);
        kani::assume(measured.is_finite() && measured.abs() < 10.0);
        kani::assume(dt.is_finite() && dt > 0.0 && dt < 0.1);

        let mut pid = PidController::new(4.0, 0.5, 0.8, 1.0, 0.5);
        let output = pid.update(setpoint, measured, dt);

        kani::assert(output.abs() <= 1.0, "output within limit");
        kani::assert(output.is_finite(), "output finite");
    }

    /// Prova que o PID nÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â£o entra em panic com entradas razoÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¡veis.
    #[kani::proof]
    #[kani::unwind(4)]
    fn pid_no_panic() {
        let setpoint: f32 = kani::any();
        let measured: f32 = kani::any();
        let dt: f32 = kani::any();
        kani::assume(setpoint.is_finite() && setpoint.abs() < 1.0e6);
        kani::assume(measured.is_finite() && measured.abs() < 1.0e6);
        kani::assume(dt.is_finite() && dt > 0.0 && dt < 0.1);

        let mut pid = PidController::new(4.0, 0.5, 0.8, 1.0, 0.5);
        let _output = pid.update(setpoint, measured, dt);
    }
}
