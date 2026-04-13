use crate::world::sight::Sight;
use crate::world::wall::Wall;

// ─────────────────────────────────────────────────────────────────────────────
// PerceptionContext
//
// A read-only snapshot of everything an observer can sense right now.
// Passed to every factor; extend this struct (never factors) when a new
// dimension of perception needs new inputs.
// ─────────────────────────────────────────────────────────────────────────────

pub struct PerceptionContext<'a> {
    pub observer_pos: (f32, f32),
    pub observer_sight: &'a Sight,
    pub target_pos: (f32, f32),
    pub walls: &'a [Wall],
}

// ─────────────────────────────────────────────────────────────────────────────
// PerceptionFactor
//
// One independently-measurable signal.  Returns 0.0 (undetected) … 1.0
// (fully detected).  Add new factors without touching existing ones.
// ─────────────────────────────────────────────────────────────────────────────

pub trait PerceptionFactor {
    fn evaluate(&self, ctx: &PerceptionContext) -> f32;
}

// ─────────────────────────────────────────────────────────────────────────────
// PerceptionResult
// ─────────────────────────────────────────────────────────────────────────────

pub struct PerceptionResult {
    /// Combined signal 0.0 (undetected) … 1.0 (fully detected).
    pub score: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// PerceptionSystem
//
// Holds a list of factors and combines them into a single result.
// Factors are multiplicative: one factor returning 0.0 silences detection
// entirely (e.g. no line-of-sight → invisible regardless of distance).
// ─────────────────────────────────────────────────────────────────────────────

pub struct PerceptionSystem {
    factors: Vec<Box<dyn PerceptionFactor>>,
}

impl PerceptionSystem {
    pub fn new() -> Self {
        Self {
            factors: Vec::new(),
        }
    }

    pub fn add_factor(&mut self, factor: impl PerceptionFactor + 'static) {
        self.factors.push(Box::new(factor));
    }

    pub fn evaluate(&self, ctx: &PerceptionContext) -> PerceptionResult {
        let score = self
            .factors
            .iter()
            .map(|f| f.evaluate(ctx))
            .fold(1.0_f32, |acc, v| acc * v)
            .clamp(0.0, 1.0);

        PerceptionResult { score }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VisibilityFactor
//
// Checks whether the target is inside the observer's sight cone (or nearby
// bubble) with an unobstructed line of sight, then attenuates the signal by
// distance so far-away targets raise suspicion more slowly than nearby ones.
// ─────────────────────────────────────────────────────────────────────────────

pub struct VisibilityFactor;

impl PerceptionFactor for VisibilityFactor {
    fn evaluate(&self, ctx: &PerceptionContext) -> f32 {
        let sight = ctx.observer_sight;

        // No cone/LOS → completely undetected.
        if !sight.can_see(ctx.observer_pos, ctx.target_pos, ctx.walls) {
            return 0.0;
        }

        // Distance attenuation: 1.0 when right next to observer, down to
        // MIN_VIS at the far edge of the cone.
        const MIN_VIS: f32 = 0.30;
        let dx = ctx.target_pos.0 - ctx.observer_pos.0;
        let dy = ctx.target_pos.1 - ctx.observer_pos.1;
        let dist = (dx * dx + dy * dy).sqrt();
        let t = (dist / sight.range).clamp(0.0, 1.0);

        1.0 - t * (1.0 - MIN_VIS)
    }
}
