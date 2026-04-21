pub(super) const SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _pad:        vec2<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertIn  { @location(0) position:  vec2<f32> }
struct InstSolid  {
    @location(1) center:    vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) color:     vec4<f32>,
}
struct InstGradient {
    @location(1) center:       vec2<f32>,
    @location(2) half_size:    vec2<f32>,
    @location(3) top_color:    vec4<f32>,
    @location(4) bottom_color: vec4<f32>,
}
struct InstShaded {
    @location(1) center:       vec2<f32>,
    @location(2) half_size:    vec2<f32>,
    @location(3) top_color:    vec4<f32>,
    @location(4) bottom_color: vec4<f32>,
    @location(5) stroke_color: vec4<f32>,
    @location(6) accent_color: vec4<f32>,
    @location(7) params:       vec4<f32>,
}
struct VertOutSolid {
    @builtin(position) clip: vec4<f32>,
    @location(0) color:      vec4<f32>,
}
struct VertOutGradient {
    @builtin(position) clip: vec4<f32>,
    @location(0) t:          f32,
    @location(1) top_color:  vec4<f32>,
    @location(2) bottom_color: vec4<f32>,
}
struct VertOutShaded {
    @builtin(position) clip:  vec4<f32>,
    @location(0) local:       vec2<f32>,
    @location(1) top_color:   vec4<f32>,
    @location(2) bottom_color: vec4<f32>,
    @location(3) stroke_color: vec4<f32>,
    @location(4) accent_color: vec4<f32>,
    @location(5) params:      vec4<f32>,
}

fn rounded_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - (half_size - vec2<f32>(radius, radius));
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@vertex fn vs_main(v: VertIn, inst: InstSolid) -> VertOutSolid {
    let world = v.position * inst.half_size + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;
    return VertOutSolid(vec4<f32>(nx, ny, 0.0, 1.0), inst.color);
}

@vertex fn vs_gradient(v: VertIn, inst: InstGradient) -> VertOutGradient {
    let world = v.position * inst.half_size + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;
    let t = v.position.y * 0.5 + 0.5;
    return VertOutGradient(vec4<f32>(nx, ny, 0.0, 1.0), t, inst.top_color, inst.bottom_color);
}

@vertex fn vs_shaded(v: VertIn, inst: InstShaded) -> VertOutShaded {
    let world = v.position * inst.half_size + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;
    return VertOutShaded(
        vec4<f32>(nx, ny, 0.0, 1.0),
        v.position,
        inst.top_color,
        inst.bottom_color,
        inst.stroke_color,
        inst.accent_color,
        inst.params,
    );
}

@fragment fn fs_plain(in: VertOutSolid) -> @location(0) vec4<f32> {
    return in.color;
}

@fragment fn fs_gradient(in: VertOutGradient) -> @location(0) vec4<f32> {
    return mix(in.bottom_color, in.top_color, in.t);
}

@fragment fn fs_shaded(in: VertOutShaded) -> @location(0) vec4<f32> {
    let aa = 0.014;
    let p = in.local;
    let uv = p * 0.5 + vec2<f32>(0.5, 0.5);

    let border_frac = clamp(in.params.x, 0.001, 0.30);
    let corner_frac = clamp(in.params.y, 0.0, 0.45);
    let hover_strength = clamp(in.params.w, 0.0, 1.0);

    let outer_sdf = rounded_rect_sdf(p, vec2<f32>(1.0, 1.0), corner_frac);
    let shape_mask = 1.0 - smoothstep(0.0, aa, outer_sdf);

    let inner_half = vec2<f32>(max(1.0 - border_frac, 0.001), max(1.0 - border_frac, 0.001));
    let inner_radius = max(corner_frac - border_frac, 0.0);
    let inner_sdf = rounded_rect_sdf(p, inner_half, inner_radius);
    let inner_mask = 1.0 - smoothstep(0.0, aa, inner_sdf);
    let border_mask = clamp(shape_mask - inner_mask, 0.0, 1.0);

    let accent_w = clamp(in.params.z, 0.0, 0.45);
    let accent_shift = (1.0 - uv.y) * 0.18;
    let accent_uv = uv.x + accent_shift;
    let accent_start = 1.0 - accent_w;
    let accent_mask = smoothstep(accent_start - aa, accent_start + aa, accent_uv)
        * (1.0 - smoothstep(1.0 - aa, 1.0 + aa, accent_uv));

    let g = smoothstep(0.0, 1.0, uv.y);
    var base = mix(in.bottom_color, in.top_color, g);

    // Modern tactical depth: diagonal modulation + subtle center lift.
    let diag = clamp(uv.y * 0.75 + (1.0 - uv.x) * 0.25, 0.0, 1.0);
    let center = 1.0 - abs(uv.x * 2.0 - 1.0);
    var base_rgb = mix(base.rgb * 0.86, base.rgb * 1.06, diag);
    base_rgb = base_rgb * (0.94 + center * 0.08);

    // Top sheen + bottom weight + subtle hover lift.
    let top_sheen = (1.0 - smoothstep(0.32, 0.86, uv.y)) * (0.10 + hover_strength * 0.04);
    let lower_weight = smoothstep(0.54, 1.0, uv.y) * 0.10;
    base_rgb = base_rgb + vec3<f32>(top_sheen) - vec3<f32>(lower_weight);
    base = vec4<f32>(base_rgb, base.a);

    var color = base;
    color = mix(color, in.stroke_color, border_mask * in.stroke_color.a);
    color = mix(color, in.accent_color, accent_mask * in.accent_color.a);
    color.a = color.a * shape_mask;
    return color;
}
"#;
