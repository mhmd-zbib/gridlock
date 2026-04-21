use super::types::{GradientQuadInstance, QuadInstance, ShadedQuadInstance};
use crate::render::screen::ScreenTransform;

pub fn quad(center: (f32, f32), half_size: (f32, f32), color: [f32; 4]) -> QuadInstance {
    QuadInstance {
        center: [center.0, center.1],
        half_size: [half_size.0, half_size.1],
        color,
    }
}

pub fn push_quad(
    out: &mut Vec<QuadInstance>,
    center: (f32, f32),
    half_size: (f32, f32),
    color: [f32; 4],
) {
    out.push(quad(center, half_size, color));
}

pub fn world_quad(
    transform: &ScreenTransform,
    world_center: (f32, f32),
    world_half_size: (f32, f32),
    color: [f32; 4],
) -> QuadInstance {
    let screen = transform.world_to_screen(world_center);
    quad(
        screen,
        (
            world_half_size.0 * transform.pixels_per_unit,
            world_half_size.1 * transform.pixels_per_unit,
        ),
        color,
    )
}

pub fn push_world_quad(
    out: &mut Vec<QuadInstance>,
    transform: &ScreenTransform,
    world_center: (f32, f32),
    world_half_size: (f32, f32),
    color: [f32; 4],
) {
    out.push(world_quad(transform, world_center, world_half_size, color));
}

pub fn gradient_quad(
    center: (f32, f32),
    half_size: (f32, f32),
    top_color: [f32; 4],
    bottom_color: [f32; 4],
) -> GradientQuadInstance {
    GradientQuadInstance {
        center: [center.0, center.1],
        half_size: [half_size.0, half_size.1],
        top_color,
        bottom_color,
    }
}

pub fn push_gradient_quad(
    out: &mut Vec<GradientQuadInstance>,
    center: (f32, f32),
    half_size: (f32, f32),
    top_color: [f32; 4],
    bottom_color: [f32; 4],
) {
    out.push(gradient_quad(center, half_size, top_color, bottom_color));
}

pub fn shaded_quad(
    center: (f32, f32),
    half_size: (f32, f32),
    top_color: [f32; 4],
    bottom_color: [f32; 4],
    stroke_color: [f32; 4],
    accent_color: [f32; 4],
    params: [f32; 4],
) -> ShadedQuadInstance {
    ShadedQuadInstance {
        center: [center.0, center.1],
        half_size: [half_size.0, half_size.1],
        top_color,
        bottom_color,
        stroke_color,
        accent_color,
        params,
    }
}

pub fn push_shaded_quad(
    out: &mut Vec<ShadedQuadInstance>,
    center: (f32, f32),
    half_size: (f32, f32),
    top_color: [f32; 4],
    bottom_color: [f32; 4],
    stroke_color: [f32; 4],
    accent_color: [f32; 4],
    params: [f32; 4],
) {
    out.push(shaded_quad(
        center,
        half_size,
        top_color,
        bottom_color,
        stroke_color,
        accent_color,
        params,
    ));
}
