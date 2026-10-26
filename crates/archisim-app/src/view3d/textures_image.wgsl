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
    let n = abs(normalize(n_in));
    var uv = world.xy;
    if (n.z < 0.7) {
        if (n.x > n.y) {
            uv = world.yz;
        } else {
            uv = world.xz;
        }
    }
    let s = max(tex.y, 0.01);
    var period = 4.0;
    if (kind == 5) {
        period = 8.0;
    }
    return uv / s / period;
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
    return c;
}

