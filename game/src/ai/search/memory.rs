use crate::world::spatial::{self, SCAN_SECTORS};


pub(super) fn update(
    pos: (f32, f32),
    last_known: (f32, f32),
    suspicion: f32,
    dt: f32,
    threat_age: &mut [f32; SCAN_SECTORS],
    threat_dir: &mut [f32; SCAN_SECTORS],
    danger_history: &mut [f32; SCAN_SECTORS],
    prev_suspicion: &mut f32,
) {
    for i in 0..SCAN_SECTORS {
        threat_age[i] += dt;
        danger_history[i] = (danger_history[i] - dt * 0.06).max(0.0);
    }

    if suspicion > *prev_suspicion + 0.012 {
        let dir = (last_known.1 - pos.1).atan2(last_known.0 - pos.0);
        let center = spatial::angle_to_sector(dir);
        let gain = 0.38 + suspicion.clamp(0.0, 1.0) * 0.35;
        register(center, dir, gain, threat_age, threat_dir, danger_history);
        register(
            (center + 1) % SCAN_SECTORS,
            dir,
            gain * 0.42,
            threat_age,
            threat_dir,
            danger_history,
        );
        register(
            (center + SCAN_SECTORS - 1) % SCAN_SECTORS,
            dir,
            gain * 0.42,
            threat_age,
            threat_dir,
            danger_history,
        );
    }

    *prev_suspicion = suspicion;
}

fn register(
    sector: usize,
    dir: f32,
    gain: f32,
    threat_age: &mut [f32; SCAN_SECTORS],
    threat_dir: &mut [f32; SCAN_SECTORS],
    danger_history: &mut [f32; SCAN_SECTORS],
) {
    threat_age[sector] = 0.0;
    threat_dir[sector] = dir;
    danger_history[sector] = (danger_history[sector] + gain).min(2.5);
}
