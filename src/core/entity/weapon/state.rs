use super::{WeaponId, WeaponStats};

#[derive(Clone, Copy, Debug)]
pub struct WeaponState {
    id: WeaponId,
    rounds_in_mag: u32,
    shot_cooldown_left: f32,
    reload_left: f32,
}

impl WeaponState {
    pub fn new(id: WeaponId) -> Self {
        let mag_size = id.stats().mag_size;
        Self {
            id,
            rounds_in_mag: mag_size,
            shot_cooldown_left: 0.0,
            reload_left: 0.0,
        }
    }

    pub fn stats(&self) -> WeaponStats {
        self.id.stats()
    }

    pub fn id(&self) -> WeaponId {
        self.id
    }

    pub fn ammo_in_mag(&self) -> u32 {
        self.rounds_in_mag
    }

    pub fn is_reloading(&self) -> bool {
        self.reload_left > 0.0
    }

    pub fn tick(&mut self, dt: f32) {
        self.shot_cooldown_left = (self.shot_cooldown_left - dt).max(0.0);

        if self.reload_left > 0.0 {
            self.reload_left = (self.reload_left - dt).max(0.0);
            if self.reload_left <= 0.0 {
                self.rounds_in_mag = self.stats().mag_size;
            }
        }
    }

    pub fn try_start_reload(&mut self) -> bool {
        if self.is_reloading() || self.rounds_in_mag == self.stats().mag_size {
            return false;
        }
        self.reload_left = self.stats().reload_time_secs;
        true
    }

    pub fn try_fire(&mut self) -> bool {
        if self.is_reloading() || self.shot_cooldown_left > 0.0 || self.rounds_in_mag == 0 {
            return false;
        }

        let fire_rate = self.stats().fire_rate_rps.max(0.01);
        self.rounds_in_mag -= 1;
        self.shot_cooldown_left = 1.0 / fire_rate;
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::core::entity::weapon::WeaponId;

    use super::WeaponState;

    fn weapon(name: &str) -> WeaponId {
        WeaponId::from_name(name).unwrap_or_else(|| panic!("missing weapon in catalog: {name}"))
    }

    #[test]
    fn fire_rate_blocks_shots_until_cooldown_elapsed() {
        let mut weapon = WeaponState::new(weapon("AK-47"));
        assert!(weapon.try_fire());
        assert!(!weapon.try_fire());
        weapon.tick(0.09);
        assert!(!weapon.try_fire());
        weapon.tick(0.02);
        assert!(weapon.try_fire());
    }

    #[test]
    fn reload_restores_magazine() {
        let mut weapon = WeaponState::new(weapon("MP5"));
        for _ in 0..weapon.stats().mag_size {
            assert!(weapon.try_fire());
            weapon.tick(1.0 / weapon.stats().fire_rate_rps);
        }
        assert_eq!(weapon.ammo_in_mag(), 0);
        assert!(weapon.try_start_reload());
        assert!(weapon.is_reloading());
        weapon.tick(weapon.stats().reload_time_secs - 0.01);
        assert_eq!(weapon.ammo_in_mag(), 0);
        weapon.tick(0.02);
        assert_eq!(weapon.ammo_in_mag(), weapon.stats().mag_size);
        assert!(!weapon.is_reloading());
    }

    #[test]
    fn smg_is_less_visible_by_distance_worse_still_better_moving_than_ak47() {
        let ak = weapon("AK-47").stats();
        let mp5 = weapon("MP5").stats();

        assert!(mp5.visibility_range < ak.visibility_range);
        assert_eq!(mp5.visibility_half_angle_deg, ak.visibility_half_angle_deg);
        assert!(mp5.aim_base_half_angle_deg > ak.aim_base_half_angle_deg);
        assert!(mp5.movement_spread_max_deg < ak.movement_spread_max_deg);
    }

    #[test]
    fn sniper_has_huge_visibility_and_movement_penalty() {
        let ak = weapon("AK-47").stats();
        let sniper = weapon("Sniper").stats();

        assert!(sniper.visibility_range > ak.visibility_range);
        assert!(sniper.visibility_half_angle_deg > ak.visibility_half_angle_deg);
        assert!(sniper.aim_base_half_angle_deg < ak.aim_base_half_angle_deg);
        assert!(sniper.movement_spread_max_deg > ak.movement_spread_max_deg);
    }

    #[test]
    fn aim_cone_render_distance_is_sniper_then_ak_then_smg() {
        let ak = weapon("AK-47").stats();
        let mp5 = weapon("MP5").stats();
        let sniper = weapon("Sniper").stats();

        assert!(sniper.aim_cone_render_range > ak.aim_cone_render_range);
        assert!(ak.aim_cone_render_range > mp5.aim_cone_render_range);
    }

    #[test]
    fn sniper_has_highest_bullet_damage_and_speed() {
        let ak = weapon("AK-47").stats();
        let mp5 = weapon("MP5").stats();
        let sniper = weapon("Sniper").stats();

        assert!(sniper.bullet_speed > ak.bullet_speed);
        assert!(ak.bullet_speed > mp5.bullet_speed);
        assert!(sniper.bullet_damage > ak.bullet_damage);
    }
}
