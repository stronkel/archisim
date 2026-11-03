// ---- Textures (pictures) ---------------------------------------------------
//
// The same textures, looked up in pictures made once on the processor
// (`archisim_render::textures`) instead of worked out at each pixel: for
// graphics cards that are slow with the full ones (Intel Macs). Each
// picture holds `period` pieces of its texture and repeats without a seam;
// `rgb` is the colour it adds and `a` the factor of the material's (over
// 1.6). The level of detail is worked out at the start of the fragment
// (`tex_level`), where every pixel still runs together.

@group(2) @binding(0) var tex_pictures: texture_2d_array<f32>;
@group(2) @binding(1) var tex_sampler: sampler;

// Where a point of a face is on its picture (0–1 over its period).
fn tex_point(world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec2<f32> {
    let kind = i32(tex.x + 0.5);
    let s = max(tex.y, 0.01);
    // As `archisim_render::textures::period`.
    var period = 4.0;
    if (kind == 5 || kind == 7 || kind == 10 || kind == 28 || kind == 29) {
        period = 8.0;
    }
    if (kind == 8 || kind == 30) {
        period = 16.0;
    }
    if (kind == 22) {
        period = 20.0;
    }
    if (kind == 32) {
        period = 48.0;
    }
    if (kind == 42 || kind == 43) {
        period = 8.0;
    }
    if (kind == 35) {
        period = 24.0;
    }
    return tex_uv(world, n_in, kind) / s / period;
}

fn tex_lod(world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> f32 {
    let size = f32(textureDimensions(tex_pictures).x);
    let q = tex_point(world, n_in, tex) * size;
    let d = max(length(dpdx(q)), length(dpdy(q)));
    return max(log2(max(d, 1e-6)), 0.0);
}

fn textured(base: vec3<f32>, world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec3<f32> {
    let kind = i32(tex.x + 0.5);
    if (kind == 0) {
        return base;
    }
    let t = textureSampleLevel(tex_pictures, tex_sampler, tex_point(world, n_in, tex), kind - 1, tex_level);
    var c = base * (t.a * 1.6) + t.rgb;
    if (kind == 15) {
        // Velvet: lighter where the face turns away from the eye.
        var d = g.forward.xyz;
        if (g.eye.w > 0.5) {
            d = normalize(world - g.eye.xyz);
        }
        let rim = 1.0 - abs(dot(normalize(n_in), d));
        c = c + base * 0.45 * rim * rim;
    }
    if (kind == 29 && abs(normalize(n_in).z) >= 0.7) {
        // Rock in strata, on top: the bare rock, not the layers.
        c = base * (0.95 + 0.1 * (t.a * 1.6 - 0.9));
    }
    return c;
}

