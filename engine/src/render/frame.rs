use super::{
    geometry::GeoVertex,
    quad::{GradientQuadInstance, QuadInstance, ShadedQuadInstance},
    sprite::SpriteCommand,
    text::TextSection,
};

/// All draw commands for one rendered frame.
///
/// Build a `Frame` each tick, fill its layers, then pass it to
/// `State::render_frame`. The split into named layers mirrors the render
/// pipeline passes; callers choose the right layer based on game semantics,
/// not GPU internals.
pub struct Frame {
    /// Cone geometry written into the stencil buffer.
    /// Empty = no masking (menus, lobby).
    pub fov_mask: Vec<GeoVertex>,
    /// Black overlay alpha applied only outside the vision cone.
    pub outside_dim: f32,
    /// Base quads: UI / non-wall world quads / scene markers.
    pub scene_quads: Vec<QuadInstance>,
    pub scene_gradient_quads: Vec<GradientQuadInstance>,
    pub scene_shaded_quads: Vec<ShadedQuadInstance>,
    /// Non-world entities hidden completely outside the vision cone.
    pub masked_quads: Vec<QuadInstance>,
    /// Textured floor quads (rendered below props and walls).
    pub floor_sprites: Vec<SpriteCommand>,
    /// Textured prop quads (rendered below walls).
    pub prop_sprites: Vec<SpriteCommand>,
    /// Wall quads composited above world quads and sprites.
    pub wall_quads: Vec<QuadInstance>,
    /// Overlays always visible on top (UI / debug visuals).
    pub geo: Vec<GeoVertex>,
    /// Overlays hidden outside the vision cone (impacts, traces, particles).
    pub masked_geo: Vec<GeoVertex>,
    pub texts: Vec<TextSection>,
}

impl Frame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.fov_mask.clear();
        self.outside_dim = 0.0;
        self.scene_quads.clear();
        self.scene_gradient_quads.clear();
        self.scene_shaded_quads.clear();
        self.masked_quads.clear();
        self.floor_sprites.clear();
        self.prop_sprites.clear();
        self.wall_quads.clear();
        self.geo.clear();
        self.masked_geo.clear();
        self.texts.clear();
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            fov_mask: Vec::new(),
            outside_dim: 0.0,
            scene_quads: Vec::new(),
            scene_gradient_quads: Vec::new(),
            scene_shaded_quads: Vec::new(),
            masked_quads: Vec::new(),
            floor_sprites: Vec::new(),
            prop_sprites: Vec::new(),
            wall_quads: Vec::new(),
            geo: Vec::new(),
            masked_geo: Vec::new(),
            texts: Vec::new(),
        }
    }
}
