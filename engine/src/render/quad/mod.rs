mod pipeline;
mod primitives;
mod renderer;
mod shader;
mod types;

pub use primitives::{
    gradient_quad, push_gradient_quad, push_quad, push_shaded_quad, push_world_quad, quad,
    shaded_quad, world_quad,
};
pub use renderer::Renderer;
pub use types::{GradientQuadInstance, QuadInstance, ShadedQuadInstance};
