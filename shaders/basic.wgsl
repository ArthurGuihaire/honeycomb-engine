struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) transform_col0: vec2<f32>,
    @location(3) transform_col1: vec2<f32>,
    @location(4) translation: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let transformed_pos = model.transform_col0 * model.position.x + model.transform_col1 * model.position.y;
    let world_pos = transformed_pos + model.translation;
    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    out.color = vec3<f32>(model.color);
    return out;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //no srgb conversion? maybe? who knows
    //let linear = srgb_to_linear(in.color);
    //return vec4(linear, 1.0);
    return vec4(in.color, 1.0);
}
