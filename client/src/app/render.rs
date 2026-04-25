use crate::render::entities::{entity_quads, EntityQuadLayers};
use crate::render::fog::{vision_cone_fog, vision_cone_mask};
use crate::render::geometry::play_geometry;
use crate::render::hud::{
    editor_texts, loadout_texts, lobby_texts, main_menu_texts, name_entry_texts, play_texts,
};
use crate::render::views::{
    AimConeView, DebugRoomView, DebugRoomsView, EnemyBodyView, EnemyConeView, EnemyDebugView,
    EntitiesView, FloorView, FogView, GeometryView, HudEnemyRow, HudPlayerView, HudView,
    ImpactView, PlayerCircleView, PropView, SightConeView, SoundFieldView, SoundRingView,
    TeammateConeFog, WorldView,
};
use crate::render::world::world_quads;
use game::ai::awareness::AiState;
use game::entity::enemy::EnemyKind;
use game::entity::weapon::attachment::AttachmentCategory;
use game::game::IMPACT_TTL;
use engine::render::geometry::GeoVertex;
use engine::render::light::{GpuLight, LightingUniform};
use engine::render::quad::{ConeVisInstance, QuadInstance, ShadedQuadInstance};
use engine::render::sprite::SpriteCommand;
use engine::render::text::TextSection;
use game::world::floor;
use game::world::prop;
use game::world::units::tiles_to_px;
use net::decode_rotation;

use super::{App, AppState};

impl App {
    pub(super) fn build_mask(&self, viewport_px: (f32, f32)) -> Vec<GeoVertex> {
        if matches!(&self.app_state, AppState::Playing) {
            let view = self.playing_fog_view();
            vision_cone_mask(&view, &self.camera, viewport_px)
        } else {
            vec![]
        }
    }

    pub(super) fn build_fog(
        &self,
        viewport_px: (f32, f32),
        outside_dim: f32,
    ) -> Option<engine::render::fog::FogUniforms> {
        if matches!(&self.app_state, AppState::Playing) {
            let view = self.playing_fog_view();
            Some(vision_cone_fog(&view, &self.camera, viewport_px, outside_dim))
        } else {
            None
        }
    }

    pub(super) fn build_geo(&self, viewport_px: (f32, f32)) -> (Vec<GeoVertex>, Vec<GeoVertex>) {
        if matches!(&self.app_state, AppState::Playing) {
            let view = self.playing_geometry_view();
            play_geometry(&view, &self.camera, viewport_px, &self.net_bullet_traces)
        } else {
            (vec![], vec![])
        }
    }

    pub(super) fn build_quads(
        &mut self,
        viewport_px: (f32, f32),
        mx: f32,
        my: f32,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>, Vec<QuadInstance>, Vec<ConeVisInstance>) {
        match &self.app_state {
            AppState::MainMenu(_) => (vec![], vec![], vec![], vec![]),
            AppState::NameEntry => (vec![], vec![], vec![], vec![]),
            AppState::Loadout(loadout) => {
                (loadout.instances(viewport_px.0, viewport_px.1), vec![], vec![], vec![])
            }
            AppState::Lobby(lobby) => {
                let selected_team = self.lobby_state.as_ref().map(|s| s.your_team).unwrap_or(0);
                let is_creator = self
                    .lobby_state
                    .as_ref()
                    .map(|state| state.is_creator)
                    .unwrap_or(false);
                let can_start = self
                    .lobby_state
                    .as_ref()
                    .map(|state| self.is_net_connected() && !state.game_started && state.is_creator)
                    .unwrap_or(false);
                (
                    lobby.instances(
                        viewport_px.0,
                        viewport_px.1,
                        mx,
                        my,
                        selected_team,
                        can_start,
                        is_creator,
                    ),
                    vec![],
                    vec![],
                    vec![],
                )
            }
            AppState::Playing => {
                let world_view = self.playing_world_view();
                let world = world_quads(&world_view, &self.camera, viewport_px);

                // Update vis_alphas (needs &mut self) before building the view (&self).
                self.update_enemy_vis_alphas();
                let entities_view = self.playing_entities_view(viewport_px);
                let EntityQuadLayers { scene: entity_scene, masked: entity_masked, cone_vis } =
                    entity_quads(&entities_view, &self.camera, viewport_px);

                let mut scene = world.scene;
                scene.extend(entity_scene);
                (scene, world.walls, entity_masked, cone_vis)
            }
            AppState::Editing => {
                let (scene, walls) =
                    self.editor.layered_instances(viewport_px.0, viewport_px.1, mx, my);
                (scene, walls, vec![], vec![])
            }
        }
    }

    pub(super) fn build_shaded_quads(
        &self,
        viewport_px: (f32, f32),
        mx: f32,
        my: f32,
    ) -> Vec<ShadedQuadInstance> {
        match &self.app_state {
            AppState::MainMenu(menu) => menu.instances(viewport_px.0, viewport_px.1, mx, my),
            _ => vec![],
        }
    }

    pub(super) fn build_texts(&self, sw: f32, sh: f32, _mx: f32, _my: f32) -> Vec<TextSection> {
        match &self.app_state {
            AppState::MainMenu(_) => main_menu_texts(sw, sh),
            AppState::NameEntry => name_entry_texts(sw, sh, &self.player_name),
            AppState::Loadout(loadout) => loadout_texts(sw, sh, loadout),
            AppState::Lobby(_) => lobby_texts(sw, sh, self.net.as_ref(), self.lobby_state.as_ref()),
            AppState::Playing => {
                let hud = self.playing_hud_view();
                play_texts(sw, sh, &hud, &self.camera)
            }
            AppState::Editing => editor_texts(sw, sh, &self.editor),
        }
    }

    fn playing_fog_view(&self) -> FogView<'_> {
        let player_pos = self.game.player_pos();
        let sight = self.game.player_sight();
        let is_spectating = self.server_me.map(|m| m.health == 0).unwrap_or(false);

        let teammate_cones = self
            .teammate_sight_cones
            .iter()
            .map(|tv| TeammateConeFog {
                pos: (tv.x, tv.y),
                sight_direction: decode_rotation(tv.rotation),
                sight_half_angle: tv.sight_half_angle,
                sight_range: tv.sight_range,
                sight_circle_radius: tv.sight_circle_radius,
            })
            .collect();

        if is_spectating {
            return FogView {
                player_pos,
                sight_direction: 0.0,
                sight_half_angle: 0.0,
                sight_range: 0.0,
                sight_circle_radius: 0.0,
                teammate_cones,
                walls: self.game.walls(),
            };
        }

        FogView {
            player_pos,
            sight_direction: sight.direction,
            sight_half_angle: sight.half_angle,
            sight_range: sight.range,
            sight_circle_radius: sight.circle_radius,
            teammate_cones,
            walls: self.game.walls(),
        }
    }

    fn playing_geometry_view(&self) -> GeometryView<'_> {
        let player_pos = self.game.player_pos();
        let sight = self.game.player_sight();
        let aim_cone = self.game.player_aim_cone();
        let active_stats = self.game.player_weapon_stats();
        let (aim_dir, aim_half) = if let Some(me) = self.server_me {
            (aim_cone.direction, me.aim_cone_half_angle)
        } else {
            (aim_cone.direction, aim_cone.half_angle())
        };

        let impacts = self
            .game
            .impact_views()
            .into_iter()
            .map(|iv| ImpactView {
                pos: (iv.x, iv.y),
                alpha: iv.alpha,
            })
            .collect();

        let enemies = self.game.enemy_views();
        let enemy_cones = enemies
            .iter()
            .filter_map(|enemy| {
                if enemy.kind == EnemyKind::TargetDummy {
                    return None;
                }

                let visible = enemy.visible_to_player;
                if !visible && !self.debug_mode {
                    return None;
                }

                let alpha_scale = if visible { 1.0 } else { 0.45 };
                let cone_color = if enemy.ai_in_combat {
                    [1.0, 0.08, 0.08, 0.35 * alpha_scale]
                } else if enemy.ai_is_alert {
                    [1.0, 0.50, 0.05, 0.28 * alpha_scale]
                } else {
                    [1.0, 0.85, 0.20, 0.14 * alpha_scale]
                };

                Some(EnemyConeView {
                    pos: (enemy.x, enemy.y),
                    circle_radius_px: tiles_to_px(enemy.sight.circle_radius),
                    circle_color: [1.0, 0.3, 0.3, if visible { 0.05 } else { 0.03 }],
                    cone_color,
                    sight_direction: enemy.sight.direction,
                    sight_half_angle: enemy.sight.half_angle,
                    sight_range: enemy.sight.range,
                })
            })
            .collect();

        let debug_rooms = if self.debug_mode {
            Some(DebugRoomsView {
                rooms: self
                    .game
                    .rooms()
                    .rooms
                    .iter()
                    .map(|room| DebugRoomView {
                        x: room.x,
                        y: room.y,
                        w: room.w,
                        h: room.h,
                        color: room.color,
                    })
                    .collect(),
                gaps: self.game.rooms().gaps.clone(),
            })
        } else {
            None
        };

        let teammate_cones: Vec<TeammateConeFog> = self
            .teammate_sight_cones
            .iter()
            .map(|tv| TeammateConeFog {
                pos: (tv.x, tv.y),
                sight_direction: decode_rotation(tv.rotation),
                sight_half_angle: tv.sight_half_angle,
                sight_range: tv.sight_range,
                sight_circle_radius: tv.sight_circle_radius,
            })
            .collect();

        let is_spectating = self.server_me.map(|m| m.health == 0).unwrap_or(false);

        let mut player_circles: Vec<PlayerCircleView> = if is_spectating {
            vec![]
        } else {
            vec![PlayerCircleView {
                pos: player_pos,
                color: [0.45, 0.80, 1.0, 1.0],
            }]
        };
        for remote in &self.net_players {
            let color = if self.my_team != 0 && remote.team == self.my_team {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [1.0, 0.20, 0.20, 1.0]
            };
            player_circles.push(PlayerCircleView {
                pos: (remote.x, remote.y),
                color,
            });
        }

        let sound_field = if is_spectating {
            None
        } else {
            self.game.player_sound_field().map(|sf| {
                use game::components::{Speed, VelocityFrac};
                let speed = self.game.world().get::<Speed>(self.game.player_entity())
                    .copied().unwrap_or_default().0;
                let vel_frac = self.game.world().get::<VelocityFrac>(self.game.player_entity())
                    .copied().unwrap_or_default().0;
                let speed_px = tiles_to_px(speed * vel_frac);
                let speed_frac = (speed_px / 200.0).clamp(0.0, 1.0);
                let rings = sf
                    .pulses
                    .iter()
                    .map(|p| SoundRingView {
                        radius_px: tiles_to_px(p.radius()),
                        alpha: p.amplitude(),
                    })
                    .collect();
                SoundFieldView {
                    pos: player_pos,
                    smoothed_radius_px: tiles_to_px(sf.smoothed_radius),
                    speed_frac,
                    rings,
                }
            })
        };

        GeometryView {
            walls: self.game.walls(),
            player_sight: SightConeView {
                pos: player_pos,
                direction: sight.direction,
                half_angle: sight.half_angle,
                range: sight.range,
                circle_radius: sight.circle_radius,
            },
            aim_cone: AimConeView {
                direction: aim_dir,
                half_angle: aim_half,
                render_range: active_stats.aim_cone_render_range,
            },
            impacts,
            enemy_cones,
            teammate_cones,
            player_circles,
            debug_rooms,
            is_spectating,
            sound_field,
        }
    }

    fn update_enemy_vis_alphas(&mut self) {
        const BETA_APPEAR: f32 = 0.25;
        const BETA_DISAPPEAR: f32 = 0.30;
        let enemies = self.game.enemy_views();
        let n = enemies.len();
        self.enemy_vis_alphas.resize(n, 0.0);
        for (i, enemy) in enemies.iter().enumerate() {
            let (target, beta) = if enemy.visible_to_player {
                (1.0_f32, BETA_APPEAR)
            } else {
                (0.0_f32, BETA_DISAPPEAR)
            };
            self.enemy_vis_alphas[i] += (target - self.enemy_vis_alphas[i]) * beta;
        }
    }

    fn playing_entities_view(&self, viewport_px: (f32, f32)) -> EntitiesView<'_> {
        let transform = self.camera.screen_transform(viewport_px);
        let ppu = transform.pixels_per_unit;
        let sight = self.game.player_sight();
        let player_pos = self.game.player_pos();
        let direction = sight.direction;
        let half_angle = sight.half_angle;
        let range_px = sight.range * ppu;
        let circle_radius_px = sight.circle_radius * ppu;

        const CONE_EDGE_FEATHER: f32 = 0.25;
        let feather = CONE_EDGE_FEATHER.min(half_angle * 0.4);
        let cos_inner = (half_angle - feather).cos();
        let cos_outer = half_angle.cos();
        let range_inner_px = range_px * 0.15;

        let (viewer_sx, viewer_sy) = transform.world_to_screen(player_pos);
        let vis_view_dir = (direction.cos(), direction.sin());

        let enemy_views = self.game.enemy_views();
        let enemies = enemy_views
            .iter()
            .enumerate()
            .map(|(i, enemy)| {
                let pos = (enemy.x, enemy.y);
                let debug = if self.debug_mode && enemy.kind == EnemyKind::Shooter {
                    Some(EnemyDebugView {
                        spawn_anchor: enemy.ai_spawn_anchor,
                        last_known_pos: enemy.ai_last_known_pos,
                        last_move_target: enemy.ai_last_move_target,
                        gap_waypoints: enemy.ai_gap_waypoints.clone(),
                    })
                } else {
                    None
                };

                let color = if enemy.visible_to_player {
                    let vis_alpha = self.enemy_vis_alphas.get(i).copied().unwrap_or(1.0);
                    enemy_body_color_with_alpha(enemy.kind, vis_alpha)
                } else {
                    enemy_body_color_dim(enemy.kind)
                };

                EnemyBodyView {
                    pos,
                    color,
                    is_visible_to_player: enemy.visible_to_player,
                    debug,
                }
            })
            .collect();

        let bullet_positions = self.game.bullet_positions();

        EntitiesView {
            enemies,
            bullet_positions,
            remote_players: &self.net_players,
            vis_viewer_pos: (viewer_sx, viewer_sy),
            vis_view_dir,
            vis_cos_inner: cos_inner,
            vis_cos_outer: cos_outer,
            vis_range_inner_px: range_inner_px,
            vis_range_outer_px: range_px,
            vis_circle_radius_px: circle_radius_px,
        }
    }

    fn playing_world_view(&self) -> WorldView<'_> {
        let floors = self
            .game
            .floors()
            .iter()
            .map(|f| FloorView {
                pos: (f.x, f.y),
                half_size: (f.width * 0.5, f.height * 0.5),
                color: floor::asset_color(&f.id, 1.0),
                texture_handle: self.floor_textures.get(&f.id).copied(),
            })
            .collect();

        let props = self
            .game
            .props()
            .iter()
            .map(|prop_instance| PropView {
                pos: (prop_instance.x, prop_instance.y),
                half_size: (prop_instance.width * 0.5, prop_instance.height * 0.5),
                color: prop::asset_color(&prop_instance.id, prop_instance.is_collider, 1.0),
                texture_handle: self.prop_textures.get(&prop_instance.id).copied(),
            })
            .collect();

        WorldView {
            level_bounds: self.game.level_bounds(),
            walls: self.game.walls(),
            floors,
            props,
        }
    }

    pub(super) fn playing_hud_view(&self) -> HudView<'_> {
        let player_pos = self.game.player_pos();
        let attachments_line = AttachmentCategory::all()
            .iter()
            .map(|category| {
                format!(
                    "{}={}",
                    category.label(),
                    self.game.player_attachment_name_for(*category).unwrap_or("-")
                )
            })
            .collect::<Vec<_>>()
            .join("  ");

        let (debug_rows, speed, room_idx, enemy_count) = if self.debug_mode {
            let enemies = self.game.enemy_views();
            let rows = enemies
                .iter()
                .enumerate()
                .map(|(idx, enemy)| {
                    let state_label = match enemy.ai_awareness {
                        AiState::Combat => "COMBAT",
                        AiState::Alert => "ALERT ",
                        AiState::Idle => "idle  ",
                    };
                    let color = if enemy.ai_in_combat {
                        [1.0, 0.35, 0.35, 1.0]
                    } else if enemy.ai_is_alert {
                        [1.0, 0.65, 0.2, 1.0]
                    } else {
                        [0.55, 0.8, 0.55, 1.0]
                    };

                    HudEnemyRow {
                        idx,
                        is_dummy: enemy.kind == EnemyKind::TargetDummy,
                        hp: enemy.hp,
                        state_label,
                        suspicion: enemy.ai_suspicion,
                        in_combat: enemy.ai_in_combat,
                        color,
                        pos: (enemy.x, enemy.y),
                        anchor: enemy.ai_spawn_anchor,
                        phase: enemy.ai_phase_name,
                        last_known_pos: enemy.ai_last_known_pos,
                        last_move_target: enemy.ai_last_move_target,
                    }
                })
                .collect();

            use game::components::{Speed, VelocityFrac};
            let speed_val = self.game.world().get::<Speed>(self.game.player_entity())
                .copied().unwrap_or_default().0;
            let vel_frac = self.game.world().get::<VelocityFrac>(self.game.player_entity())
                .copied().unwrap_or_default().0;

            (
                rows,
                speed_val * vel_frac,
                self.game.rooms().find_room_at(player_pos.0, player_pos.1),
                self.game.enemy_count(),
            )
        } else {
            (Vec::new(), 0.0, None, 0)
        };

        let ammo = self
            .server_me
            .map(|me| me.ammo)
            .unwrap_or(self.game.player_ammo_in_mag() as u8);
        let reloading = self
            .server_me
            .map(|me| me.reload_progress > 0)
            .unwrap_or(self.game.player_is_reloading());

        let health = self.server_me.map(|me| me.health).unwrap_or(100);

        HudView {
            player: HudPlayerView {
                weapon_name: self.game.player_weapon_name(),
                weapon_class: self.game.player_weapon_class_label(),
                ammo,
                mag_size: self.game.player_mag_size(),
                reloading,
                attachments_line,
                speed,
                pos: player_pos,
                room_idx,
                enemy_count,
            },
            enemies: debug_rows,
            net: self.net.as_ref(),
            health,
            match_state: self.match_state,
            kill_notification: self
                .kill_notification
                .as_ref()
                .map(|(name, _)| name.clone()),
        }
    }

    /// Build the per-frame lighting uniform from current game state.
    ///
    /// Sources (in priority order):
    /// 1. Persistent level lights — evaluated with their current time state
    /// 2. Player personal light
    /// 3. Remote player lights
    /// 4. Bullet tracer lights
    /// 5. Impact flash lights
    ///
    /// All world-space positions/radii are converted to framebuffer pixels via
    /// the camera transform before upload.
    pub(super) fn build_lighting(&self, viewport_px: (f32, f32)) -> LightingUniform {
        let AppState::Playing = &self.app_state else {
            return LightingUniform::default();
        };

        let transform = self.camera.screen_transform(viewport_px);
        let ppu = transform.pixels_per_unit; // pixels per tile

        let mut lighting = LightingUniform {
            ambient: [0.18, 0.18, 0.22, 1.0],
            ..Default::default()
        };

        for source in self.game.lights() {
            let ev = source.evaluate();
            let (sx, sy) = transform.world_to_screen((ev.x, ev.y));
            let gpu = match ev.cone {
                None => GpuLight::point([sx, sy], ev.radius * ppu, ev.intensity, ev.color),
                Some((dir_rad, half_angle_rad)) => {
                    let dir = [dir_rad.cos(), dir_rad.sin()];
                    GpuLight::cone(
                        [sx, sy],
                        ev.radius * ppu,
                        ev.intensity,
                        ev.color,
                        dir,
                        half_angle_rad.cos(),
                    )
                }
            };
            lighting.push(gpu);
        }

        // Player personal area light.
        let (px, py) = transform.world_to_screen(self.game.player_pos());
        lighting.push(GpuLight::point([px, py], 8.0 * ppu, 0.90, [0.95, 0.90, 0.75]));

        for remote in &self.net_players {
            let (rx, ry) = transform.world_to_screen((remote.x, remote.y));
            lighting.push(GpuLight::point([rx, ry], 5.0 * ppu, 0.70, [0.80, 0.90, 1.00]));
        }

        for (bx_w, by_w) in self.game.bullet_positions() {
            let (bx, by) = transform.world_to_screen((bx_w, by_w));
            lighting.push(GpuLight::point([bx, by], 1.5 * ppu, 1.50, [1.00, 0.80, 0.30]));
        }

        for iv in self.game.impact_views() {
            let (ix, iy) = transform.world_to_screen((iv.x, iv.y));
            lighting.push(GpuLight::point([ix, iy], 4.0 * ppu, 2.50 * iv.alpha, [1.00, 0.50, 0.10]));
        }

        lighting
    }

    /// Collect textured sprite draw commands for the current frame.
    ///
    /// Only floors/props whose textures were successfully loaded into the GPU
    /// cache emit sprite commands; the rest continue to render as solid-color
    /// quads.
    pub(super) fn build_sprites(
        &self,
        viewport_px: (f32, f32),
    ) -> (Vec<SpriteCommand>, Vec<SpriteCommand>) {
        if let AppState::Editing = &self.app_state {
            return self
                .editor
                .build_editor_sprites(&self.floor_textures, &self.prop_textures);
        }

        let AppState::Playing = &self.app_state else {
            return (vec![], vec![]);
        };

        let world_view = self.playing_world_view();
        let transform = self.camera.screen_transform(viewport_px);
        let mut floor_cmds = Vec::new();
        let mut prop_cmds = Vec::new();

        // Floors first so they render beneath props.
        for f in &world_view.floors {
            let Some(handle) = f.texture_handle else {
                continue;
            };
            let (cx, cy) = transform.world_to_screen(f.pos);
            let hw = f.half_size.0 * transform.pixels_per_unit;
            let hh = f.half_size.1 * transform.pixels_per_unit;
            let mut cmd = SpriteCommand::simple(handle, [cx, cy], [hw, hh], 0.0);
            cmd.uv_rect = floor_uv_rect_for_world_size(f.half_size);
            floor_cmds.push(cmd);
        }

        for p in &world_view.props {
            let Some(handle) = p.texture_handle else {
                continue;
            };
            let (cx, cy) = transform.world_to_screen(p.pos);
            let hw = p.half_size.0 * transform.pixels_per_unit;
            let hh = p.half_size.1 * transform.pixels_per_unit;
            prop_cmds.push(SpriteCommand::simple(handle, [cx, cy], [hw, hh], 1.0));
        }

        (floor_cmds, prop_cmds)
    }
}

fn floor_uv_rect_for_world_size(half_size: (f32, f32)) -> [f32; 4] {
    // Keep 1.0x1.0 floors mapped to the full texture. For sub-unit floors,
    // crop the sampled area instead of scaling so texture density stays fixed.
    let width_units = (half_size.0 * 2.0).max(0.0);
    let height_units = (half_size.1 * 2.0).max(0.0);
    let u_span = width_units.min(1.0);
    let v_span = height_units.min(1.0);
    let u_min = 0.5 - u_span * 0.5;
    let u_max = 0.5 + u_span * 0.5;
    let v_min = 0.5 - v_span * 0.5;
    let v_max = 0.5 + v_span * 0.5;
    [u_min, v_min, u_max, v_max]
}

fn enemy_body_color_with_alpha(kind: EnemyKind, alpha: f32) -> [f32; 4] {
    match kind {
        EnemyKind::Shooter => [1.0, 0.2, 0.2, alpha],
        EnemyKind::TargetDummy => [1.0, 0.85, 0.2, alpha],
    }
}

fn enemy_body_color_dim(kind: EnemyKind) -> [f32; 4] {
    match kind {
        EnemyKind::Shooter => [0.6, 0.15, 0.15, 0.55],
        EnemyKind::TargetDummy => [0.6, 0.5, 0.12, 0.55],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use net::{SelfState, encode_rotation};
    use std::sync::Once;

    fn ensure_workspace_cwd() {
        static INIT_CWD: Once = Once::new();
        INIT_CWD.call_once(|| {
            let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("client crate should have a workspace root");
            std::env::set_current_dir(repo_root).expect("failed to switch to workspace root");
        });
    }

    #[test]
    fn fog_view_prefers_client_predicted_rotation() {
        ensure_workspace_cwd();
        let mut app = App::default();
        app.game.set_player_sight_direction(1.0);
        app.server_me = Some(SelfState {
            rotation: encode_rotation(-0.85),
            health: 100,
            ..SelfState::default()
        });

        let view = app.playing_fog_view();
        let sight_direction = app.game.player_sight().direction;
        assert!((view.sight_direction - sight_direction).abs() < 1e-6);
    }

    #[test]
    fn hidden_enemy_cones_render_only_in_debug_mode() {
        ensure_workspace_cwd();
        let mut app = App::default();
        // Clear all existing bot entities and spawn one invisible enemy.
        {
            use game::components::{BotTag, VisibilityState};
            let bots: Vec<_> = app.game.world().entities_with::<BotTag>();
            for e in bots {
                app.game.world_mut().despawn(e);
            }
            let entity = game::bundles::spawn_enemy(
                app.game.world_mut(),
                2.0, 2.0,
                game::entity::enemy::EnemyKind::Shooter,
            );
            if let Some(vs) = app.game.world_mut().get_mut::<VisibilityState>(entity) {
                vs.score = 0.0;
            }
        }

        app.debug_mode = false;
        assert!(app.playing_geometry_view().enemy_cones.is_empty());

        app.debug_mode = true;
        assert_eq!(app.playing_geometry_view().enemy_cones.len(), 1);
    }

    #[test]
    fn hud_uses_server_ammo_and_reload_state() {
        ensure_workspace_cwd();
        let mut app = App::default();
        app.server_me = Some(SelfState {
            ammo: 7,
            reload_progress: 42,
            ..SelfState::default()
        });

        let view = app.playing_hud_view();
        assert_eq!(view.player.ammo, 7);
        assert!(view.player.reloading);
        assert!(view.enemies.is_empty());
    }

    #[test]
    fn floor_uv_rect_crops_subunit_tiles_instead_of_scaling() {
        let uv = floor_uv_rect_for_world_size((0.25, 0.4));
        assert!((uv[0] - 0.25).abs() < 1e-6);
        assert!((uv[1] - 0.1).abs() < 1e-6);
        assert!((uv[2] - 0.75).abs() < 1e-6);
        assert!((uv[3] - 0.9).abs() < 1e-6);
    }

    #[test]
    fn floor_uv_rect_uses_full_texture_for_unit_and_larger_tiles() {
        let uv = floor_uv_rect_for_world_size((1.0, 0.6));
        assert!((uv[0] - 0.0).abs() < 1e-6);
        assert!((uv[1] - 0.0).abs() < 1e-6);
        assert!((uv[2] - 1.0).abs() < 1e-6);
        assert!((uv[3] - 1.0).abs() < 1e-6);
    }
}
