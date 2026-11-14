//! Surveillance payloads for the RNFC.
//! Copyright (c) 2026 Andre Oliveira. Todos os direitos reservados.
#![deny(unsafe_code)]

/// Estado do drone relevante para a vigilancia (lido do NavCore, nao o altera).
#[derive(Clone, Copy, Debug)]
pub struct SurveillanceContext {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub yaw: f32,
    pub time_s: f32,
    pub hovering: bool,
}

/// Contrato genÃƒÆ’Ã‚Â©rico para qualquer sensor de vigilancia.
pub trait Payload {
    type Error;
    type Frame;

    fn init(&mut self) -> Result<(), Self::Error>;
    fn capture(&mut self, ctx: &SurveillanceContext) -> Result<Self::Frame, Self::Error>;
    fn process(&mut self, frame: Self::Frame) -> Result<Self::Frame, Self::Error>;
}

/// Camara simulada - gera um frame sintetico quando o drone esta em HOVER.
pub struct SimulatedCamera {
    pub resolution: (u32, u32),
    pub frames_captured: u64,
}

impl SimulatedCamera {
    pub fn new(resolution: (u32, u32)) -> Self {
        Self {
            resolution,
            frames_captured: 0,
        }
    }
}

impl Payload for SimulatedCamera {
    type Error = &'static str;
    type Frame = (u64, u32, u32);

    fn init(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn capture(&mut self, ctx: &SurveillanceContext) -> Result<Self::Frame, Self::Error> {
        if !ctx.hovering {
            return Err("not hovering");
        }
        self.frames_captured += 1;
        Ok((self.frames_captured, self.resolution.0, self.resolution.1))
    }

    fn process(&mut self, frame: Self::Frame) -> Result<Self::Frame, Self::Error> {
        Ok(frame)
    }
}

/// Microfone simulado - capta "audio" (energia) quando em HOVER.
pub struct SimulatedMicrophone {
    pub samples: u64,
}

impl SimulatedMicrophone {
    pub fn new() -> Self {
        Self { samples: 0 }
    }
}

impl Default for SimulatedMicrophone {
    fn default() -> Self {
        Self::new()
    }
}

impl Payload for SimulatedMicrophone {
    type Error = &'static str;
    type Frame = u64;

    fn init(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn capture(&mut self, ctx: &SurveillanceContext) -> Result<Self::Frame, Self::Error> {
        if !ctx.hovering {
            return Err("not hovering");
        }
        self.samples += 1;
        Ok(42)
    }

    fn process(&mut self, f: Self::Frame) -> Result<Self::Frame, Self::Error> {
        Ok(f)
    }
}
