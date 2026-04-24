use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Runtime light source
// ---------------------------------------------------------------------------

/// A persistent, time-varying light source placed in the world.
///
/// Call [`tick`] every game update with the elapsed delta, then
/// [`evaluate`] to obtain the resolved parameters for that instant.
pub struct LightSource {
    pub x: f32,
    pub y: f32,
    pub color: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
    pub kind: LightKind,
    time: f32,
}

/// Behaviour of a [`LightSource`] over time.
pub enum LightKind {
    /// No time dependence.
    Static,
    /// Intensity oscillates: I(t) = base · (1 + amp · sin(2π · freq · t))
    Flicker { amplitude: f32, frequency: f32 },
    /// Radius oscillates: R(t) = base + amp · sin(2π · freq · t)
    Pulse { amplitude: f32, frequency: f32 },
    /// Light orbits its base position in a circle.
    /// p(t) = base + orbit_radius · (cos(2π · freq · t), sin(2π · freq · t))
    Orbit { orbit_radius: f32, frequency: f32 },
    /// Directional cone — emits light only within `half_angle` of `direction`.
    /// `direction` is an angle in radians (0 = +x, π/2 = +y).
    Cone { direction: f32, half_angle: f32 },
}

/// Resolved light parameters for the current instant.
pub struct EvaluatedLight {
    pub x: f32,
    pub y: f32,
    pub color: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
    /// For cone lights: (direction_radians, half_angle_radians).
    pub cone: Option<(f32, f32)>,
}

impl LightSource {
    pub fn new(x: f32, y: f32, color: [f32; 3], radius: f32, intensity: f32, kind: LightKind) -> Self {
        Self { x, y, color, radius, intensity, kind, time: 0.0 }
    }

    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
    }

    pub fn evaluate(&self) -> EvaluatedLight {
        use std::f32::consts::TAU;
        match &self.kind {
            LightKind::Static => EvaluatedLight {
                x: self.x,
                y: self.y,
                color: self.color,
                radius: self.radius,
                intensity: self.intensity,
                cone: None,
            },
            LightKind::Flicker { amplitude, frequency } => {
                let i = self.intensity * (1.0 + amplitude * (TAU * frequency * self.time).sin());
                EvaluatedLight {
                    x: self.x,
                    y: self.y,
                    color: self.color,
                    radius: self.radius,
                    intensity: i.max(0.0),
                    cone: None,
                }
            }
            LightKind::Pulse { amplitude, frequency } => {
                let r = self.radius + amplitude * (TAU * frequency * self.time).sin();
                EvaluatedLight {
                    x: self.x,
                    y: self.y,
                    color: self.color,
                    radius: r.max(0.0),
                    intensity: self.intensity,
                    cone: None,
                }
            }
            LightKind::Orbit { orbit_radius, frequency } => {
                let phase = TAU * frequency * self.time;
                EvaluatedLight {
                    x: self.x + orbit_radius * phase.cos(),
                    y: self.y + orbit_radius * phase.sin(),
                    color: self.color,
                    radius: self.radius,
                    intensity: self.intensity,
                    cone: None,
                }
            }
            LightKind::Cone { direction, half_angle } => EvaluatedLight {
                x: self.x,
                y: self.y,
                color: self.color,
                radius: self.radius,
                intensity: self.intensity,
                cone: Some((*direction, *half_angle)),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Level-file representation (serialisable)
// ---------------------------------------------------------------------------

/// Light definition stored in a level JSON file.
#[derive(Serialize, Deserialize, Clone)]
pub struct LevelLight {
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_white")]
    pub color: [f32; 3],
    pub radius: f32,
    #[serde(default = "default_one")]
    pub intensity: f32,
    #[serde(default)]
    pub kind: LevelLightKind,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LevelLightKind {
    #[default]
    Static,
    Flicker { amplitude: f32, frequency: f32 },
    Pulse { amplitude: f32, frequency: f32 },
    Orbit { orbit_radius: f32, frequency: f32 },
    Cone { direction: f32, half_angle: f32 },
}

impl LevelLight {
    pub fn into_source(self) -> LightSource {
        let kind = match self.kind {
            LevelLightKind::Static => LightKind::Static,
            LevelLightKind::Flicker { amplitude, frequency } => LightKind::Flicker { amplitude, frequency },
            LevelLightKind::Pulse { amplitude, frequency } => LightKind::Pulse { amplitude, frequency },
            LevelLightKind::Orbit { orbit_radius, frequency } => LightKind::Orbit { orbit_radius, frequency },
            LevelLightKind::Cone { direction, half_angle } => LightKind::Cone { direction, half_angle },
        };
        LightSource::new(self.x, self.y, self.color, self.radius, self.intensity, kind)
    }
}

fn default_white() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn default_one() -> f32 {
    1.0
}
