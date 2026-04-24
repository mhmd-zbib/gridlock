pub(super) const SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _pad:        vec2<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

// GpuLight — 48 bytes, stride 48 (matches Rust repr(C))
// offset  0  screen_pos      vec2<f32>   align 8
// offset  8  radius          f32
// offset 12  intensity       f32
// offset 16  color           vec3<f32>   align 16
// offset 28  cos_half_angle  f32   (< -1.0 → point light, no cone)
// offset 32  direction       vec2<f32>   align 8
// offset 40  _pad            vec2<f32>
struct GpuLight {
    screen_pos:     vec2<f32>,
    radius:         f32,
    intensity:      f32,
    color:          vec3<f32>,
    cos_half_angle: f32,
    direction:      vec2<f32>,
    _pad:           vec2<f32>,
}
struct Lighting {
    ambient: vec4<f32>,
    count:   u32,
    _pad0:   u32,
    _pad1:   u32,
    _pad2:   u32,
    lights:  array<GpuLight, 16>,
}
@group(0) @binding(1) var<uniform> lighting: Lighting;

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
    let local   = v.pos * inst.half_size;
    let rotated = vec2<f32>(
        local.x * cos_r - local.y * sin_r,
        local.x * sin_r + local.y * cos_r,
    );
    let world = rotated + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;
    let t  = v.pos * 0.5 + 0.5;
    let uv = mix(inst.uv_min, inst.uv_max, t);
    return VertOut(vec4<f32>(nx, ny, 0.0, 1.0), uv, inst.tint);
}

// Evaluate a single light's contribution at screen position `p`.
//
// Point lights: cos_half_angle < -1.0 (sentinel -2.0)
// Cone  lights: apply directional falloff on top of distance falloff
//   f_cone = max(0, (dot(dir, to_frag) - cos_ha) / (1 - cos_ha))
fn light_contrib(p: vec2<f32>, L: GpuLight) -> vec3<f32> {
    let d   = distance(p, L.screen_pos);
    let att = max(0.0, 1.0 - d / L.radius);
    var c   = L.color * (L.intensity * att * att);

    if L.cos_half_angle > -1.5 {
        let to_frag  = normalize(p - L.screen_pos);
        let cos_theta = dot(L.direction, to_frag);
        let denom     = 1.0 - L.cos_half_angle;
        let cone_att  = max(0.0, (cos_theta - L.cos_half_angle) / denom);
        c = c * cone_att;
    }
    return c;
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    let base  = textureSample(t_sprite, s_sprite, in.uv) * in.tint;

    // C_final = albedo * clamp(ambient + Σ light_i(P))
    var accum = lighting.ambient.rgb;
    for (var i = 0u; i < lighting.count; i++) {
        accum += light_contrib(in.clip.xy, lighting.lights[i]);
    }

    return vec4<f32>(base.rgb * clamp(accum, vec3<f32>(0.0), vec3<f32>(1.0)), base.a);
}
"#;
