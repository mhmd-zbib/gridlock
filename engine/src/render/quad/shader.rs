pub(super) const SHADER: &str = r#"
struct Uniforms {
    screen_size:   vec2<f32>,
    player_pos:    vec2<f32>,
    player_dir:    vec2<f32>,
    cos_half_fov:  f32,
    cone_range:    f32,
    circle_radius: f32,
    ambient:       f32,
    _pad:          vec2<f32>,
}
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertIn  { @location(0) position:  vec2<f32> }
struct InstIn  {
    @location(1) center:    vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) color:     vec4<f32>,
}
struct VertOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color:      vec4<f32>,
    @location(1) screen_pos: vec2<f32>,
}

@vertex fn vs_main(v: VertIn, inst: InstIn) -> VertOut {
    let world = v.position * inst.half_size + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;
    return VertOut(vec4<f32>(nx, ny, 0.0, 1.0), inst.color, world);
}

fn visible(frag_pos: vec2<f32>) -> f32 {
    let v    = frag_pos - u.player_pos;
    let dist = length(v);
    if dist <= u.circle_radius { return 1.0; }
    if dist >  u.cone_range    { return 0.0; }
    let d = dot(v / dist, u.player_dir);
    if d < u.cos_half_fov { return 0.0; }
    return 1.0;
}

@fragment fn fs_plain(in: VertOut) -> @location(0) vec4<f32> {
    return in.color;
}

@fragment fn fs_scene(in: VertOut) -> @location(0) vec4<f32> {
    let vis    = visible(in.screen_pos);
    let factor = u.ambient + (1.0 - u.ambient) * vis;
    return vec4<f32>(in.color.rgb * factor, in.color.a);
}

@fragment fn fs_enemy(in: VertOut) -> @location(0) vec4<f32> {
    if visible(in.screen_pos) < 0.5 { discard; }
    return in.color;
}
"#;
