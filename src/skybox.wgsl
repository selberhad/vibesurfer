struct SkyboxUniforms {
    inv_view_proj: mat4x4<f32>,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: SkyboxUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc_pos: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;

    // Fullscreen triangle
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);

    output.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    output.ndc_pos = vec2<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0);

    return output;
}

// Hash function for procedural stars
fn hash3(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Generate stars with twinkling
fn stars(dir: vec3<f32>, density: f32, time: f32) -> f32 {
    let p = dir * 100.0;
    let i = floor(p);
    let f = fract(p);

    var star = 0.0;

    // Check neighboring cells
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            for (var z = -1; z <= 1; z++) {
                let offset = vec3<f32>(f32(x), f32(y), f32(z));
                let cell = i + offset;
                let h = hash3(cell);

                // Only place star if hash is above threshold (controls density)
                if (h > 1.0 - density) {
                    // Star position within cell
                    let star_pos = vec3<f32>(
                        hash3(cell + vec3<f32>(1.0, 0.0, 0.0)),
                        hash3(cell + vec3<f32>(0.0, 1.0, 0.0)),
                        hash3(cell + vec3<f32>(0.0, 0.0, 1.0))
                    );

                    let cell_pos = offset + star_pos;
                    let dist = length(f - cell_pos);

                    // Star size and brightness (much larger stars)
                    let size = 0.05 + hash3(cell + vec3<f32>(10.0, 20.0, 30.0)) * 0.1;
                    let brightness = smoothstep(size, 0.0, dist);

                    // Twinkle: each star has unique phase offset and frequency
                    let twinkle_phase = hash3(cell + vec3<f32>(50.0, 60.0, 70.0)) * 6.28318; // 0 to 2π
                    let twinkle_speed = 0.5 + hash3(cell + vec3<f32>(80.0, 90.0, 100.0)) * 1.5; // 0.5 to 2.0
                    let twinkle = 0.7 + 0.3 * sin(time * twinkle_speed + twinkle_phase); // Oscillates 0.7-1.0

                    star = max(star, brightness * twinkle);
                }
            }
        }
    }

    return star;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct world space direction from NDC
    let ndc = vec4<f32>(input.ndc_pos.x, -input.ndc_pos.y, 1.0, 1.0);
    var world_pos = uniforms.inv_view_proj * ndc;
    world_pos = world_pos / world_pos.w;

    let dir = normalize(world_pos.xyz);

    // Pure black background
    let sky_color = vec3<f32>(0.0, 0.0, 0.0);

    // Add stars everywhere with twinkling
    let star_density = 0.02; // Increased from 0.003 to 0.02 (much more stars)
    let star_brightness = stars(dir, star_density, uniforms.time);

    // Star color variation (white to blue-white)
    let star_tint = vec3<f32>(
        0.9 + hash3(dir * 123.45) * 0.1,
        0.9 + hash3(dir * 234.56) * 0.1,
        1.0
    );

    let star_color = star_tint * star_brightness * 100.0; // Much brighter stars

    // Combine sky and stars
    let final_color = sky_color + star_color;

    return vec4<f32>(final_color, 1.0);
}
