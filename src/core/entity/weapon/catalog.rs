#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponClass {
    Rifle,
    Smg,
    Sniper,
}

impl WeaponClass {
    pub fn label(self) -> &'static str {
        match self {
            WeaponClass::Rifle => "Rifle",
            WeaponClass::Smg => "SMG",
            WeaponClass::Sniper => "Sniper",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponId {
    Ak47,
    Mp5,
    Sniper,
}

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

impl WeaponId {
    pub fn stats(self) -> WeaponStats {
        match self {
            WeaponId::Ak47 => WeaponStats {
                class: WeaponClass::Rifle,
                name: "AK-47",
                visibility_range: 320.0,
                visibility_half_angle_deg: 40.0,
                aim_cone_render_range: 220.0,
                aim_base_half_angle_deg: 1.4,
                movement_spread_max_deg: 12.5,
                bullet_speed: 780.0,
                bullet_damage: 1,
                recoil_per_shot_deg: 3.6,
                recoil_max_deg: 22.0,
                recoil_decay_deg_per_sec: 8.0,
                fire_rate_rps: 10.0,
                mag_size: 30,
                reload_time_secs: 2.4,
            },
            WeaponId::Mp5 => WeaponStats {
                class: WeaponClass::Smg,
                name: "MP5",
                visibility_range: 270.0,
                visibility_half_angle_deg: 40.0,
                aim_cone_render_range: 150.0,
                aim_base_half_angle_deg: 2.0,
                movement_spread_max_deg: 9.5,
                bullet_speed: 620.0,
                bullet_damage: 1,
                recoil_per_shot_deg: 2.1,
                recoil_max_deg: 15.0,
                recoil_decay_deg_per_sec: 11.0,
                fire_rate_rps: 13.0,
                mag_size: 30,
                reload_time_secs: 1.9,
            },
            WeaponId::Sniper => WeaponStats {
                class: WeaponClass::Sniper,
                name: "Sniper",
                visibility_range: 620.0,
                visibility_half_angle_deg: 20.0,
                aim_cone_render_range: 450.0,
                aim_base_half_angle_deg: 0.35,
                movement_spread_max_deg: 80.0,
                bullet_speed: 1200.0,
                bullet_damage: 3,
                recoil_per_shot_deg: 8.0,
                recoil_max_deg: 32.0,
                recoil_decay_deg_per_sec: 5.0,
                fire_rate_rps: 1.1,
                mag_size: 5,
                reload_time_secs: 3.0,
            },
        }
    }
}
