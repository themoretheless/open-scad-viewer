struct Camera {
    screen: vec2<f32>,
    origin: vec2<f32>,
    offset: vec2<f32>,
    zoom: f32,
    dithering: f32,
};
@group(0) @binding(0) var<uniform> camera: Camera;
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};
@vertex fn vertex(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> Out {
    var out: Out;
    let screen = camera.origin + camera.offset + position * camera.zoom;
    out.position = vec4<f32>(2.0 * screen.x / camera.screen.x - 1.0, 1.0 - 2.0 * screen.y / camera.screen.y, 0.0, 1.0);
    out.color = color;
    return out;
}
@fragment fn fragment(input: Out) -> @location(0) vec4<f32> {
    var color = input.color;
    if camera.dithering == 1.0 {
        let f = 0.06711056 * input.position.x + 0.00583715 * input.position.y;
        let noise = (fract(52.9829189 * fract(f)) - 0.5) * 0.95;
        color = vec4<f32>(color.rgb + noise / 255.0, color.a);
    }
    return color;
}
