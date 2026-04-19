use crate::net::ConnState;
use crate::render::entities::entity_quads;
use crate::render::fog::vision_cone_mask;
use crate::render::geometry::play_geometry;
use crate::render::hud::{
    editor_texts, loadout_texts, lobby_texts, main_menu_texts, name_entry_texts, play_texts,
};
use crate::render::views::{
    AimConeView, DebugRoomView, DebugRoomsView, EnemyBodyView, EnemyConeView, EnemyDebugView,
    EntitiesView, FogView, GeometryView, HudEnemyRow, HudPlayerView, HudView, ImpactView,
    PlayerCircleView, PropView, SightConeView, TeammateConeFog, WorldView,
};
use crate::render::world::world_quads;
use game::ai::awareness::AiState;
use game::entity::enemy::EnemyKind;
use game::entity::weapon::attachment::AttachmentCategory;
use game::game::IMPACT_TTL;
use game::render::geometry::GeoVertex;
use game::render::quad::QuadInstance;
use game::render::text::TextSection;
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

    pub(super) fn build_geo(&self, viewport_px: (f32, f32)) -> (Vec<GeoVertex>, Vec<GeoVertex>) {
        if matches!(&self.app_state, AppState::Playing) {
            let view = self.playing_geometry_view();
            play_geometry(&view, &self.camera, viewport_px, &self.net_bullet_traces)
        } else {
            (vec![], vec![])
        }
    }

    pub(super) fn build_quads(
        &self,
        viewport_px: (f32, f32),
        mx: f32,
        my: f32,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
        let (sw, sh) = viewport_px;
        match &self.app_state {
            AppState::MainMenu(menu) => {
                (menu.instances(viewport_px.0, viewport_px.1, mx, my), vec![])
            }
            AppState::NameEntry => {
                // Dark panel behind the name input.
                (
                    vec![
                        QuadInstance {
                            center: [sw * 0.5, sh * 0.5],
                            half_size: [sw * 0.30, sh * 0.14],
                            color: [0.06, 0.06, 0.08, 0.96],
                        },
                        // Input box outline
                        QuadInstance {
                            center: [sw * 0.5, sh * 0.56],
                            half_size: [sw * 0.22, sh * 0.04],
                            color: [0.18, 0.18, 0.22, 1.0],
                        },
                    ],
                    vec![],
                )
            }
            AppState::Loadout(loadout) => (loadout.instances(viewport_px.0, viewport_px.1), vec![]),
            AppState::Lobby(lobby) => {
                let ls = self.lobby_state.as_ref();
                let selected_team = ls.map(|s| s.your_team).unwrap_or(0);
                let is_creator = ls.map(|s| s.is_creator).unwrap_or(false);
                let game_started = ls.map(|s| s.game_started).unwrap_or(false);
                let connected = self
                    .net
                    .as_ref()
                    .map(|n| matches!(n.state(), ConnState::Connected { .. }))
                    .unwrap_or(false);
                let can_start = connected && !game_started && is_creator;

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
                )
            }
            AppState::Playing => {
                let world_view = self.playing_world_view();
                let world = world_quads(&world_view, &self.camera, viewport_px);

                let entities_view = self.playing_entities_view();
                let (entity_scene, entity_masked) =
                    entity_quads(&entities_view, &self.camera, viewport_px);

                let mut scene = world;
                scene.extend(entity_scene);
                (scene, entity_masked)
            }
            AppState::Editing => (
                self.editor.instances(viewport_px.0, viewport_px.1, mx, my),
                vec![],
            ),
        }
    }

    pub(super) fn build_texts(&self, sw: f32, sh: f32, _mx: f32, _my: f32) -> Vec<TextSection> {
        match &self.app_state {
            AppState::MainMenu(_) => main_menu_texts(sw, sh),
            AppState::NameEntry => name_entry_texts(sw, sh, &self.player_name),
            AppState::Loadout(loadout) => loadout_texts(sw, sh, loadout),
            AppState::Lobby(_) => lobby_texts(sw, sh, self.net.as_ref(), self.lobby_state.as_ref()),
            AppState::Playing => {
                let view = self.playing_hud_view();
                play_texts(sw, sh, &view, &self.camera)
            }
            AppState::Editing => editor_texts(sw, sh, &self.editor),
        }
    }

    fn playing_fog_view(&self) -> FogView<'_> {
        let player = &self.game.player;
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

        // Spectators have no sight cone of their own — fog is purely the union
        // of living teammates' cones (sent by the server for spectators too).
        if is_spectating {
            return FogView {
                player_pos: (player.movement.x, player.movement.y),
                sight_direction: 0.0,
                sight_half_angle: 0.0,
                sight_range: 0.0,
                sight_circle_radius: 0.0,
                teammate_cones,
                walls: &self.game.walls,
            };
        }

        FogView {
            player_pos: (player.movement.x, player.movement.y),
            // Keep look direction fully client-side responsive; server still
            // authoritatively resolves visibility/combat in snapshots.
            sight_direction: player.sight.direction,
            sight_half_angle: player.sight.half_angle,
            sight_range: player.sight.range,
            sight_circle_radius: player.sight.circle_radius,
            teammate_cones,
            walls: &self.game.walls,
        }
    }

    fn playing_geometry_view(&self) -> GeometryView<'_> {
        let player = &self.game.player;
        let player_pos = (player.movement.x, player.movement.y);
        let active_stats = player.active_weapon_stats();
        let (aim_dir, aim_half) = if let Some(me) = self.server_me {
            (player.aim_cone.direction, me.aim_cone_half_angle)
        } else {
            (player.aim_cone.direction, player.aim_cone.half_angle())
        };

        let impacts = self
            .game
            .impacts
            .iter()
            .map(|impact| ImpactView {
                pos: (impact.x, impact.y),
                alpha: (impact.ttl / IMPACT_TTL).clamp(0.0, 1.0),
            })
            .collect();

        let enemy_cones = self
            .game
            .enemies
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
                let cone_color = if enemy.brain.awareness.in_combat() {
                    [1.0, 0.08, 0.08, 0.35 * alpha_scale]
                } else if enemy.brain.awareness.is_alert() {
                    [1.0, 0.50, 0.05, 0.28 * alpha_scale]
                } else {
                    [1.0, 0.85, 0.20, 0.14 * alpha_scale]
                };

                Some(EnemyConeView {
                    pos: (enemy.movement.x, enemy.movement.y),
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
                    .rooms
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
                gaps: self.game.rooms.gaps.clone(),
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

        // Local player circle — omitted when spectating (no character on the map).
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

        GeometryView {
            walls: &self.game.walls,
            player_sight: SightConeView {
                pos: player_pos,
                direction: player.sight.direction,
                half_angle: player.sight.half_angle,
                range: player.sight.range,
                circle_radius: player.sight.circle_radius,
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
        }
    }

    fn playing_entities_view(&self) -> EntitiesView<'_> {
        let enemies = self
            .game
            .enemies
            .iter()
            .map(|enemy| {
                let pos = (enemy.movement.x, enemy.movement.y);
                let debug = if self.debug_mode && enemy.kind == EnemyKind::Shooter {
                    Some(EnemyDebugView {
                        spawn_anchor: enemy.brain.spawn_anchor(),
                        last_known_pos: enemy.brain.awareness.last_known_pos(),
                        last_move_target: enemy.brain.last_move_target,
                        gap_waypoints: enemy.brain.debug_gaps(pos, &self.game.walls),
                    })
                } else {
                    None
                };

                EnemyBodyView {
                    pos,
                    color: enemy_body_color(enemy.kind, enemy.visible_to_player),
                    debug,
                }
            })
            .collect();

        let bullet_positions = self
            .game
            .bullets
            .iter()
            .map(|bullet| (bullet.x, bullet.y))
            .collect();

        EntitiesView {
            enemies,
            bullet_positions,
            remote_players: &self.net_players,
        }
    }

    fn playing_world_view(&self) -> WorldView<'_> {
        let props = self
            .game
            .props
            .iter()
            .map(|prop_instance| PropView {
                pos: (prop_instance.x, prop_instance.y),
                half_size: (prop_instance.width * 0.5, prop_instance.height * 0.5),
                color: prop::asset_color(&prop_instance.id, prop_instance.is_collider, 1.0),
            })
            .collect();

        WorldView {
            level_bounds: self.game.level_bounds,
            walls: &self.game.walls,
            props,
        }
    }

    fn playing_hud_view(&self) -> HudView<'_> {
        let player = &self.game.player;
        let attachments_line = AttachmentCategory::all()
            .iter()
            .map(|category| {
                format!(
                    "{}={}",
                    category.label(),
                    player.attachment_name_for(*category).unwrap_or("-")
                )
            })
            .collect::<Vec<_>>()
            .join("  ");

        let (debug_rows, speed, room_idx, enemy_count) = if self.debug_mode {
            let rows = self
                .game
                .enemies
                .iter()
                .enumerate()
                .map(|(idx, enemy)| {
                    let state_label = match enemy.brain.awareness.state {
                        AiState::Combat => "COMBAT",
                        AiState::Alert => "ALERT ",
                        AiState::Idle => "idle  ",
                    };
                    let color = if enemy.brain.awareness.in_combat() {
                        [1.0, 0.35, 0.35, 1.0]
                    } else if enemy.brain.awareness.is_alert() {
                        [1.0, 0.65, 0.2, 1.0]
                    } else {
                        [0.55, 0.8, 0.55, 1.0]
                    };

                    HudEnemyRow {
                        idx,
                        is_dummy: enemy.kind == EnemyKind::TargetDummy,
                        hp: enemy.hp,
                        state_label,
                        suspicion: enemy.brain.awareness.suspicion,
                        in_combat: enemy.brain.awareness.state == AiState::Combat,
                        color,
                        pos: (enemy.movement.x, enemy.movement.y),
                        anchor: enemy.brain.spawn_anchor(),
                        phase: enemy.brain.phase_name(),
                        last_known_pos: enemy.brain.awareness.last_known_pos(),
                        last_move_target: enemy.brain.last_move_target,
                    }
                })
                .collect();

            (
                rows,
                player.movement.speed * player.movement.velocity_frac,
                self.game
                    .rooms
                    .find_room_at(player.movement.x, player.movement.y),
                self.game.enemies.len(),
            )
        } else {
            (Vec::new(), 0.0, None, 0)
        };

        let ammo = self
            .server_me
            .map(|me| me.ammo)
            .unwrap_or(player.ammo_in_mag() as u8);
        let reloading = self
            .server_me
            .map(|me| me.reload_progress > 0)
            .unwrap_or(player.is_reloading());

        let health = self.server_me.map(|me| me.health).unwrap_or(100);

        HudView {
            player: HudPlayerView {
                weapon_name: player.weapon_name(),
                weapon_class: player.weapon_class_label(),
                ammo,
                mag_size: player.mag_size(),
                reloading,
                attachments_line,
                speed,
                pos: (player.movement.x, player.movement.y),
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
}

fn enemy_body_color(kind: EnemyKind, visible: bool) -> [f32; 4] {
    match (kind, visible) {
        (EnemyKind::Shooter, true) => [1.0, 0.2, 0.2, 1.0],
        (EnemyKind::Shooter, false) => [0.6, 0.15, 0.15, 0.55],
        (EnemyKind::TargetDummy, true) => [1.0, 0.85, 0.2, 1.0],
        (EnemyKind::TargetDummy, false) => [0.6, 0.5, 0.12, 0.55],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::entity::enemy::Enemy;
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
        app.game.player.sight.direction = 1.0;
        app.server_me = Some(SelfState {
            rotation: encode_rotation(-0.85),
            health: 100,
            ..SelfState::default()
        });

        let view = app.playing_fog_view();
        assert!((view.sight_direction - app.game.player.sight.direction).abs() < 1e-6);
    }

    #[test]
    fn hidden_enemy_cones_render_only_in_debug_mode() {
        ensure_workspace_cwd();
        let mut app = App::default();
        app.game.enemies.clear();
        let mut enemy = Enemy::new(2.0, 2.0);
        enemy.visible_to_player = false;
        app.game.enemies.push(enemy);

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
}
