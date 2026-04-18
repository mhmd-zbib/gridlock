pub(super) const SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _pad:        vec2<f32>,
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
}

@vertex fn vs_main(v: VertIn, inst: InstIn) -> VertOut {
    let world = v.position * inst.half_size + inst.center;
    let nx =  (world.x / u.screen_size.x) * 2.0 - 1.0;
    let ny = -(world.y / u.screen_size.y) * 2.0 + 1.0;
    return VertOut(vec4<f32>(nx, ny, 0.0, 1.0), inst.color);
}

@fragment fn fs_plain(in: VertOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;
