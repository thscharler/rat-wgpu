struct VertexOutput {
    @location(0) UV: vec2<f32>,
    @builtin(position) gl_Position: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> ScreenSize: vec4<f32>;

@vertex
fn vs_main(
    @location(0) VertexCoord: vec2<f32>,
    @location(1) UV: vec2<f32>,
) -> VertexOutput {
    let gl_Position = vec4<f32>((2.0 * VertexCoord / ScreenSize.xy - 1.0) * vec2(1.0, -1.0), 0.0, 1.0);
    return VertexOutput(UV, gl_Position);
}

struct FragmentOutput {
    @location(0) FragColor: vec4<f32>,
}

@group(1) @binding(0)
var Sampler: sampler;
@group(1) @binding(1)
var Image: texture_2d<f32>;
@group(1) @binding(2)
var<uniform> UVTransform: mat2x3<f32>;
@group(1) @binding(3)
var<uniform> UVClip: vec4<f32>;

@fragment
fn fs_main(
    @location(0) UV: vec2<f32>,
) -> FragmentOutput {

    let clip0 = UVClip.xy;
    let clip1 = UVClip.zw;

    // outside the clip
    if UV.x < clip0.x || UV.x > clip1.x || UV.y < clip0.y || UV.y > clip1.y {
        return FragmentOutput(vec4<f32>(0.0, 0.0, 0.0, 0.0));
    }

    let UVTransformed = vec3<f32>(UV, 1.0) * UVTransform;

    // outside the texture
    if (UVTransformed.x < 0.0 || UVTransformed.x > 1.0 || UVTransformed.y < 0.0 || UVTransformed.y > 1.0) {
        return FragmentOutput(vec4<f32>(0.0, 0.0, 0.0, 0.0));
    }

    let imageSize = textureDimensions(Image);
    let size = vec2<f32>(f32(imageSize.x), f32(imageSize.y));

    var textureColor = textureSample(Image, Sampler, UVTransformed);

    return FragmentOutput(textureColor);
}