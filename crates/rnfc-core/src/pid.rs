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
            kp, ki, kd,
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
        if self.integral > self.integral_limit { self.integral = self.integral_limit; }
        if self.integral < -self.integral_limit { self.integral = -self.integral_limit; }

        // Derivada (na primeira iteracao nao ha "anterior")
        let derivative = if self.first_step {
            self.first_step = false;
            0.0
        } else {
            -(measured - self.prev_measured) / dt
        };
        self.prev_measured = measured;

        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;

        // Saturação da saida
        if output > self.output_limit { self.output_limit }
        else if output < -self.output_limit { -self.output_limit }
        else { output }
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_measured = 0.0;
        self.first_step = true;
    }
}
