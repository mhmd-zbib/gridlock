use crate::net::ConnState;
use crate::render::entities::entity_quads;
use crate::render::fog::vision_cone_mask;
use crate::render::geometry::play_geometry;
use crate::render::hud::{editor_texts, loadout_texts, lobby_texts, main_menu_texts, play_texts};
use crate::render::world::world_quads;
use engine::render::geometry::GeoVertex;
use engine::render::quad::QuadInstance;
use engine::render::text::TextSection;

use super::{App, AppState};

impl App {
    pub(super) fn build_mask(&self, viewport_px: (f32, f32)) -> Vec<GeoVertex> {
        if matches!(&self.app_state, AppState::Playing) {
            vision_cone_mask(
                &self.game,
                &self.camera,
                viewport_px,
                self.server_me.as_ref(),
            )
        } else {
            vec![]
        }
    }

    pub(super) fn build_geo(&self, viewport_px: (f32, f32)) -> (Vec<GeoVertex>, Vec<GeoVertex>) {
        if matches!(&self.app_state, AppState::Playing) {
            play_geometry(
                &self.game,
                &self.camera,
                self.debug_mode,
                viewport_px,
                &self.net_bullet_traces,
                self.server_me.as_ref(),
            )
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
        match &self.app_state {
            AppState::MainMenu(menu) => {
                (menu.instances(viewport_px.0, viewport_px.1, mx, my), vec![])
            }
            AppState::Loadout(loadout) => (loadout.instances(viewport_px.0, viewport_px.1), vec![]),
            AppState::Lobby(lobby) => {
                let selected_team = self.lobby_state.as_ref().map(|s| s.your_team).unwrap_or(0);
                let can_start = self
                    .net
                    .as_ref()
                    .map(|n| matches!(n.state(), ConnState::Connected { .. }))
                    .unwrap_or(false)
                    && !self
                        .lobby_state
                        .as_ref()
                        .map(|s| s.game_started)
                        .unwrap_or(false);

                (
                    lobby.instances(
                        viewport_px.0,
                        viewport_px.1,
                        mx,
                        my,
                        selected_team,
                        can_start,
                    ),
                    vec![],
                )
            }
            AppState::Playing => {
                let world = world_quads(&self.game, &self.camera, viewport_px);
                let (entity_scene, entity_masked) = entity_quads(
                    &self.game,
                    &self.camera,
                    self.debug_mode,
                    viewport_px,
                    &self.net_players,
                );

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
            AppState::Loadout(loadout) => loadout_texts(sw, sh, loadout),
            AppState::Lobby(_) => lobby_texts(sw, sh, self.net.as_ref(), self.lobby_state.as_ref()),
            AppState::Playing => play_texts(
                sw,
                sh,
                &self.game,
                &self.camera,
                self.debug_mode,
                self.net.as_ref(),
                self.server_me.as_ref(),
            ),
            AppState::Editing => editor_texts(sw, sh, &self.editor),
        }
    }
}
