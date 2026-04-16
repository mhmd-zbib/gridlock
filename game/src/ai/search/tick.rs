use super::phase::Phase;
use super::scoring::HOLD_MIN_TIME;
use super::{
    AI_SCAN_LOG, ARRIVE_RADIUS, CALM_SUSPICION, ENV_REBUILD_DIST, FOCUS_STICK_MIN_TIME,
    FOCUS_SWITCH_SCORE_DELTA, MAX_SPAWN_DRIFT, RETURN_FOCUS_SCORE, SearchDecision, SearchPlanner,
    move_threshold,
};
use crate::world::ray::wrap_angle;
use crate::world::spatial;
use crate::world::wall::Wall;

pub(super) fn tick_move_to_last_known(
    planner: &mut SearchPlanner,
    pos: (f32, f32),
    last_known: (f32, f32),
    suspicion: f32,
    walls: &[Wall],
    dt: f32,
) -> SearchDecision {
    let dx = last_known.0 - pos.0;
    let dy = last_known.1 - pos.1;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= ARRIVE_RADIUS {
        planner.phase = Phase::Scanning;
        return tick_scanning(planner, pos, last_known, suspicion, walls, dt);
    }
    SearchDecision {
        move_target: Some(last_known),
        look_dir: dy.atan2(dx),
    }
}

pub(super) fn tick_scanning(
    planner: &mut SearchPlanner,
    pos: (f32, f32),
    last_known: (f32, f32),
    suspicion: f32,
    walls: &[Wall],
    dt: f32,
) -> SearchDecision {
    let moved_from_env_anchor = spatial::distance(pos, planner.env_anchor);
    if !planner.env_ready || moved_from_env_anchor > ENV_REBUILD_DIST {
        planner.env_anchor = pos;
        planner.env_ready = true;
        planner.focus_gap_id = None;
        planner.active_gap_id = None;
        planner.active_gap_timer = 0.0;
    }

    let graph = planner.build_graph(pos, walls);
    let mut evals = planner.score(graph, pos, suspicion, last_known);

    if evals.is_empty() {
        if spatial::distance(pos, planner.spawn_anchor) > MAX_SPAWN_DRIFT * 0.7 {
            planner.phase = Phase::ReturningToAnchor;
            return tick_return_to_anchor(planner, pos, last_known, suspicion, walls, dt);
        }
        planner.active_gap_timer += dt;
        let look = wrap_angle(planner.approach_dir + (planner.active_gap_timer * 2.2).sin() * 0.25);
        return SearchDecision {
            move_target: None,
            look_dir: look,
        };
    }

    evals.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let focus = evals[0];
    planner.focus_gap_id = Some(focus.gap_id);

    if !planner.eval_contains_active(&evals) {
        planner.active_gap_id = Some(focus.gap_id);
        planner.active_gap_timer = focus.attention_time.max(FOCUS_STICK_MIN_TIME);
    } else {
        planner.active_gap_timer -= dt;
        let active = planner
            .find_eval(&evals, planner.active_gap_id)
            .copied()
            .unwrap_or(focus);

        let can_switch = planner.active_gap_timer <= 0.0;
        let focus_is_better = focus.score >= active.score + FOCUS_SWITCH_SCORE_DELTA;
        if can_switch && active.gap_id != focus.gap_id && focus_is_better {
            planner.active_gap_id = Some(focus.gap_id);
            planner.active_gap_timer = focus.attention_time.max(FOCUS_STICK_MIN_TIME);
        } else if can_switch && active.gap_id == focus.gap_id {
            planner.active_gap_timer = active.attention_time.max(FOCUS_STICK_MIN_TIME);
        }
    }
    let current = planner
        .find_eval(&evals, planner.active_gap_id)
        .unwrap_or(&evals[0]);
    let look_dir = (current.pos.1 - pos.1).atan2(current.pos.0 - pos.0);

    if AI_SCAN_LOG {
        println!(
            "[ai/search] scan focus_gap={} active_gap={} score={:.2} attn={:.2}s suspicion={:.2}",
            planner.focus_gap_id.unwrap_or(current.gap_id),
            current.gap_id,
            current.score,
            current.attention_time,
            suspicion,
        );
    }

    let threshold = move_threshold(suspicion);
    if current.score >= threshold {
        let gap_dist = spatial::distance(pos, current.pos);
        if gap_dist > ARRIVE_RADIUS {
            planner.phase = Phase::HoldingGap;
            planner.hold_timer = current.attention_time.max(HOLD_MIN_TIME);
            return SearchDecision {
                move_target: Some(current.pos),
                look_dir,
            };
        }
        planner.phase = Phase::HoldingGap;
        planner.hold_timer = current.attention_time.max(HOLD_MIN_TIME);
    }

    if spatial::distance(pos, planner.spawn_anchor) > MAX_SPAWN_DRIFT
        && current.score < RETURN_FOCUS_SCORE
    {
        planner.phase = Phase::ReturningToAnchor;
        return tick_return_to_anchor(planner, pos, last_known, suspicion, walls, dt);
    }

    SearchDecision {
        move_target: None,
        look_dir,
    }
}

pub(super) fn tick_holding_gap(
    planner: &mut SearchPlanner,
    pos: (f32, f32),
    last_known: (f32, f32),
    suspicion: f32,
    walls: &[Wall],
    dt: f32,
) -> SearchDecision {
    let graph = planner.build_graph(pos, walls);
    let evals = planner.score(graph, pos, suspicion, last_known);
    let Some(current) = planner.find_eval(&evals, planner.active_gap_id) else {
        planner.phase = Phase::Scanning;
        return tick_scanning(planner, pos, last_known, suspicion, walls, dt);
    };

    let look = (current.pos.1 - pos.1).atan2(current.pos.0 - pos.0);
    let threshold = move_threshold(suspicion);
    let dist = spatial::distance(pos, current.pos);

    if dist > ARRIVE_RADIUS && current.score >= threshold {
        return SearchDecision {
            move_target: Some(current.pos),
            look_dir: look,
        };
    }

    planner.hold_timer -= dt;
    if planner.hold_timer <= 0.0 || current.score < threshold * 0.85 {
        planner.phase = Phase::Scanning;
    }

    if spatial::distance(pos, planner.spawn_anchor) > MAX_SPAWN_DRIFT
        && current.score < RETURN_FOCUS_SCORE
    {
        planner.phase = Phase::ReturningToAnchor;
    }

    SearchDecision {
        move_target: None,
        look_dir: look,
    }
}

pub(super) fn tick_return_to_anchor(
    planner: &mut SearchPlanner,
    pos: (f32, f32),
    last_known: (f32, f32),
    suspicion: f32,
    walls: &[Wall],
    dt: f32,
) -> SearchDecision {
    let anchor_dx = planner.spawn_anchor.0 - pos.0;
    let anchor_dy = planner.spawn_anchor.1 - pos.1;
    let anchor_dist = (anchor_dx * anchor_dx + anchor_dy * anchor_dy).sqrt();

    if anchor_dist <= ARRIVE_RADIUS {
        planner.phase = if suspicion <= CALM_SUSPICION {
            Phase::Done
        } else {
            Phase::Scanning
        };
        if planner.phase == Phase::Done {
            return SearchDecision {
                move_target: None,
                look_dir: planner.approach_dir,
            };
        }
        return tick_scanning(planner, pos, last_known, suspicion, walls, dt);
    }

    let graph = planner.build_graph(pos, walls);
    let evals = planner.score(graph, pos, suspicion, last_known);
    let look = if let Some(top) = evals.first() {
        (top.pos.1 - pos.1).atan2(top.pos.0 - pos.0)
    } else {
        anchor_dy.atan2(anchor_dx)
    };

    SearchDecision {
        move_target: Some(planner.spawn_anchor),
        look_dir: look,
    }
}
