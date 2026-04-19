pub(super) const SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _pad:        vec2<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;

struct VertIn {
    @location(0) pos: vec2<f32>,
}

struct InstIn {
    @location(1) center:    vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) rotation:  f32,
    @location(4) uv_min:    vec2<f32>,
    @location(5) uv_max:    vec2<f32>,
    @location(6) tint:      vec4<f32>,
}

struct VertOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv:         vec2<f32>,
    @location(1) tint:       vec4<f32>,
}

@vertex
fn vs_main(v: VertIn, inst: InstIn) -> VertOut {
    let cos_r = cos(inst.rotation);
    let sin_r = sin(inst.rotation);

    // Scale the unit quad by half_size then rotate.
    let local = v.pos * inst.half_size;
    let rotated = vec2<f32>(
        local.x * cos_r - local.y * sin_r,
        local.x * sin_r + local.y * cos_r,
    );

    // Translate to screen-space centre and convert to NDC (Y flipped).
    let world = rotated + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;

    // Map unit quad [-1,1] → [0,1] → [uv_min, uv_max].
    let t  = v.pos * 0.5 + 0.5;
    let uv = mix(inst.uv_min, inst.uv_max, t);

    return VertOut(vec4<f32>(nx, ny, 0.0, 1.0), uv, inst.tint);
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    return textureSample(t_sprite, s_sprite, in.uv) * in.tint;
}
"#;
