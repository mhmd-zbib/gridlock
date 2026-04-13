// ─────────────────────────────────────────────────────────────────────────────
// Spatial scanning utilities
//
// Pure geometric helpers for radial environment scanning.  No AI, no entity
// state — just geometry derived from walls.
//
// Used by:
//   • world::rooms   — level-wide room / gap analysis
//   • ai::search     — reactive gap-based search planner
//   • ai::guard      — idle entrance watcher
//   • ai::brain      — angle-stepping helper
// ─────────────────────────────────────────────────────────────────────────────

use crate::core::world::ray::{cast_ray, wrap_angle};
use crate::core::world::units::px_to_tiles;
use crate::core::world::wall::Wall;
use std::f32::consts::PI;

pub const SCAN_RANGE: f32 = px_to_tiles(230.0);
pub const SCAN_SECTORS: usize = 36;
pub const GAP_OPEN_HIT: f32 = SCAN_RANGE * 0.70;
pub const GAP_SIDE_BLOCK_HIT: f32 = SCAN_RANGE * 0.50;

/// Cast one ray per sector and return the hit distance for each.
pub fn sample_sector_hits(pos: (f32, f32), walls: &[Wall]) -> [f32; SCAN_SECTORS] {
    let mut hits = [0.0f32; SCAN_SECTORS];
    for (i, slot) in hits.iter_mut().enumerate() {
        let angle = sector_center_angle(i);
        let dir = (angle.cos(), angle.sin());
        *slot = cast_ray(pos, dir, SCAN_RANGE, walls);
    }
    hits
}

/// Mark sectors that look like gap openings: open beyond threshold and
/// adjacent to a blocked sector (sudden transition = entrance edge).
pub fn detect_gap_sectors(hits: &[f32; SCAN_SECTORS]) -> [bool; SCAN_SECTORS] {
    let mut candidate = [false; SCAN_SECTORS];
    for i in 0..SCAN_SECTORS {
        let l = hits[(i + SCAN_SECTORS - 1) % SCAN_SECTORS];
        let c = hits[i];
        let r = hits[(i + 1) % SCAN_SECTORS];
        let side_blocked = l < GAP_SIDE_BLOCK_HIT || r < GAP_SIDE_BLOCK_HIT;
        let sudden_open = c > l + px_to_tiles(32.0) || c > r + px_to_tiles(32.0);
        candidate[i] = c >= GAP_OPEN_HIT && side_blocked && sudden_open;
    }
    candidate
}

/// Group adjacent candidate sectors into clusters, merging any wrap-around
/// cluster that straddles the 0/SCAN_SECTORS boundary.
pub fn cluster_gap_sectors(candidate: &[bool; SCAN_SECTORS]) -> Vec<Vec<usize>> {
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    let mut i = 0;
    while i < SCAN_SECTORS {
        if !candidate[i] {
            i += 1;
            continue;
        }
        let mut cluster = Vec::new();
        while i < SCAN_SECTORS && candidate[i] {
            cluster.push(i);
            i += 1;
        }
        clusters.push(cluster);
    }
    // Merge the last and first clusters if they form one opening that wraps.
    if clusters.len() >= 2 && candidate[0] && candidate[SCAN_SECTORS - 1] {
        let mut tail = clusters.pop().unwrap_or_default();
        let mut head = clusters.remove(0);
        tail.append(&mut head);
        clusters.insert(0, tail);
    }
    clusters
}

/// Return the sector index with the highest ray hit within a cluster.
pub fn cluster_center_idx(cluster: &[usize], hits: &[f32; SCAN_SECTORS]) -> usize {
    let mut best = cluster[0];
    let mut best_hit = hits[best];
    for &idx in cluster.iter().skip(1) {
        if hits[idx] > best_hit {
            best = idx;
            best_hit = hits[idx];
        }
    }
    best
}

/// World angle (radians) for the centre of sector `idx`.
pub fn sector_center_angle(idx: usize) -> f32 {
    let span = (2.0 * PI) / SCAN_SECTORS as f32;
    wrap_angle(-PI + span * (idx as f32 + 0.5))
}

/// Convert a world angle to the nearest sector index.
pub fn angle_to_sector(angle: f32) -> usize {
    let t = ((wrap_angle(angle) + PI) / (2.0 * PI)).clamp(0.0, 1.0);
    let idx = (t * SCAN_SECTORS as f32).floor() as usize;
    idx.min(SCAN_SECTORS - 1)
}

/// Rotate `from` toward `to` by at most `max_step` radians.
pub fn step_angle(from: f32, to: f32, max_step: f32) -> f32 {
    let delta = wrap_angle(to - from);
    if delta.abs() <= max_step {
        wrap_angle(to)
    } else {
        wrap_angle(from + delta.signum() * max_step)
    }
}

/// Euclidean distance between two 2-D points.
pub fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
