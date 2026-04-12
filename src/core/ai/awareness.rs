// ─────────────────────────────────────────────────────────────────────────────
// Awareness
//
// The enemy's internal belief about the player.  Completely decoupled from
// game state — it only receives a perception score (0.0..=1.0) each frame and
// the current target position, then drives its own state machine.
// ─────────────────────────────────────────────────────────────────────────────

/// Suspicion climbs by (score × RISE × dt) when the target is perceived.
const SUSPICION_RISE: f32 = 2.0;
/// Suspicion falls by (FALL × dt) per second when the target is not perceived.
const SUSPICION_FALL: f32 = 0.35;

const COMBAT_THRESHOLD: f32 = 0.70;
const ALERT_THRESHOLD: f32 = 0.20;

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AiState {
    /// Low suspicion — patrol / idle behaviour.
    Idle,
    /// Medium suspicion — go to last known position, search area.
    Alert,
    /// High suspicion — actively pursue and engage.
    Combat,
}

/// Retained knowledge about the last sighting.
pub struct Memory {
    pub last_known_pos: (f32, f32),
}

pub struct Awareness {
    /// 0.0 (oblivious) … 1.0 (full combat certainty).
    pub suspicion: f32,
    /// Current behavioural state derived from suspicion.
    pub state: AiState,
    /// None until the target is seen at least once.
    pub memory: Option<Memory>,
}

impl Awareness {
    pub fn new() -> Self {
        Self {
            suspicion: 0.0,
            state: AiState::Idle,
            memory: None,
        }
    }

    /// Call once per frame with the raw perception score and the target's
    /// current position (used only when score > 0 to update memory).
    pub fn update(&mut self, perception_score: f32, target_pos: (f32, f32), dt: f32) {
        if perception_score > 0.0 {
            self.suspicion = (self.suspicion + perception_score * SUSPICION_RISE * dt).min(1.0);
            self.memory = Some(Memory {
                last_known_pos: target_pos,
            });
        } else {
            self.suspicion = (self.suspicion - SUSPICION_FALL * dt).max(0.0);
        }

        self.state = if self.suspicion >= COMBAT_THRESHOLD {
            AiState::Combat
        } else if self.suspicion >= ALERT_THRESHOLD {
            AiState::Alert
        } else {
            AiState::Idle
        };
    }

    pub fn last_known_pos(&self) -> Option<(f32, f32)> {
        self.memory.as_ref().map(|m| m.last_known_pos)
    }

    pub fn in_combat(&self) -> bool {
        self.state == AiState::Combat
    }
    pub fn is_alert(&self) -> bool {
        self.state == AiState::Alert
    }
}
