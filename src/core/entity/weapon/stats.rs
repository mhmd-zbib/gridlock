use super::WeaponClass;

#[derive(Clone, Copy, Debug)]
pub struct WeaponStats {
    pub class: WeaponClass,
    pub name: &'static str,
    pub visibility_range: f32,
    pub visibility_half_angle_deg: f32,
    pub aim_cone_render_range: f32,
    pub aim_base_half_angle_deg: f32,
    pub movement_spread_max_deg: f32,
    pub bullet_speed: f32,
    pub bullet_damage: u32,
    pub recoil_per_shot_deg: f32,
    pub recoil_max_deg: f32,
    pub recoil_decay_deg_per_sec: f32,
    pub fire_rate_rps: f32,
    pub mag_size: u32,
    pub reload_time_secs: f32,
}
