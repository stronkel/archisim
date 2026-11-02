// ---- Textures (light) ------------------------------------------------------
//
// The same textures drawn more simply, for graphics drivers that hang on
// the full ones (AMD on Intel Macs): no `switch`, no loops, no remainders
// of real numbers; stones and gravel from smooth noise instead of cells.

fn tex_lod(world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> f32 {
    return 0.0;
}

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// How far into a joint a coordinate is (0 in the joint, 1 away from it).
fn joint(t: f32, width: f32) -> f32 {
    let f = fract(t);
    let e = min(f, 1.0 - f);
    return smoothstep(width * 0.5, width, e);
}

fn textured(base: vec3<f32>, world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec3<f32> {
    let kind = tex.x;
    if (kind < 0.5) {
        return base;
    }
    let p = tex_uv(world, n_in, i32(kind + 0.5)) / max(tex.y, 0.01);
    let mortar = mix(base, vec3<f32>(0.78, 0.76, 0.72), 0.75);
    var out = base;
    if (kind < 1.5) {
        // Brick.
        let row = floor(p.y * 4.0);
        let odd = fract(row * 0.5) * 2.0;
        let q = vec2<f32>(p.x + 0.5 * odd, p.y * 4.0);
        let id = hash2(vec2<f32>(floor(q.x), row));
        out = mix(mortar, base * (0.82 + 0.3 * id), joint(q.x, 0.06) * joint(q.y, 0.2));
    } else if (kind < 2.5) {
        // Stone.
        let v = noise(p * 1.6);
        let edge = smoothstep(0.05, 0.15, abs(fract(v * 3.0) - 0.5));
        out = mix(base * 0.5, base * (0.75 + 0.4 * noise(p * 2.3)), edge);
    } else if (kind < 3.5) {
        // Tiles.
        let id = hash2(floor(p));
        out = mix(mortar, base * (0.95 + 0.08 * id), joint(p.x, 0.03) * joint(p.y, 0.03));
    } else if (kind < 4.5) {
        // Wood.
        out = base * (0.88 + 0.1 * sin(p.y * 40.0 + noise(p * vec2<f32>(1.5, 8.0)) * 6.0));
    } else if (kind < 5.5) {
        // Boards.
        let row = floor(p.y);
        let along = p.x / 8.0 + hash2(vec2<f32>(row, 1.0));
        let id = hash2(vec2<f32>(floor(along), row));
        out = mix(base * 0.4, base * (0.82 + 0.25 * id), joint(p.y, 0.06) * joint(along, 0.01));
    } else if (kind < 6.5) {
        // Board-marked concrete.
        let q = vec2<f32>(p.x, p.y * 2.0);
        out = base * (0.94 + 0.1 * noise(p * 12.0)) * mix(0.82, 1.0, joint(q.x, 0.008) * joint(q.y, 0.012));
    } else if (kind < 7.5) {
        // Render.
        out = base * (0.95 + 0.07 * noise(p * 60.0));
    } else if (kind < 8.5) {
        // Grass.
        out = base * (0.7 + 0.55 * noise(p * vec2<f32>(18.0, 40.0)));
    } else if (kind < 9.5) {
        // Gravel.
        out = base * (0.7 + 0.5 * noise(p * 9.0));
    } else if (kind < 10.5) {
        // Water.
        out = base * (0.88 + 0.25 * smoothstep(0.55, 0.8, noise(p * vec2<f32>(3.0, 1.2))));
    } else if (kind < 11.5) {
        // Sheet metal.
        out = base * (0.8 + 0.3 * (0.5 + 0.5 * sin(p.x * 6.2832)));
    } else if (kind < 12.5) {
        // Linen: a soft mottle.
        out = base * (0.92 + 0.1 * noise(p * vec2<f32>(4.0, 60.0)));
    } else if (kind < 13.5) {
        // Bouclé: little loops.
        out = base * (0.86 + 0.2 * noise(p * 28.0));
    } else if (kind < 14.5) {
        // Knit: rows of stitches.
        out = base * (0.82 + 0.18 * joint(p.x * 10.0, 0.25) * joint(p.y * 7.0, 0.3));
    } else if (kind < 15.5) {
        // Velvet: a soft pile.
        out = base * (0.88 + 0.12 * noise(p * 12.0));
    } else if (kind < 16.5) {
        // Wicker: a basket weave of squares.
        let q = p * 7.0;
        out = base * (0.6 + 0.4 * joint(q.x, 0.15) * joint(q.y, 0.15));
    } else if (kind < 17.5) {
        // Terrazzo: chips in the ground.
        let chip = smoothstep(0.72, 0.78, noise(p * 18.0));
        out = mix(base, vec3<f32>(0.32, 0.3, 0.3), chip * 0.8);
    } else if (kind < 18.5) {
        // Marble: grey veins.
        let w = p.x * 1.3 + p.y * 0.7 + noise(p * 1.5) * 3.0;
        out = base * (1.0 - 0.3 * (1.0 - smoothstep(0.0, 0.08, abs(fract(w) - 0.5))));
    } else if (kind < 19.5) {
        // Soil: a mottle with light stones.
        let stone = smoothstep(0.8, 0.86, noise(p * 20.0));
        out = mix(base * (0.78 + 0.25 * noise(p * 3.0) + 0.12 * noise(p * 17.0)), base * 1.3 + vec3<f32>(0.08, 0.08, 0.07), stone);
    } else if (kind < 20.5) {
        // Fallen leaves.
        let leaf = smoothstep(0.55, 0.62, noise(p * 5.0));
        let tint = mix(vec3<f32>(0.4, 0.2, 0.1), vec3<f32>(0.66, 0.47, 0.18), noise(p * 1.3));
        out = mix(base * (0.72 + 0.25 * noise(p * 6.0)), tint, leaf * 0.85);
    } else if (kind < 21.5) {
        // Ashlar: blocks in courses half a block high.
        let row = floor(p.y * 2.0);
        let u = p.x + hash2(vec2<f32>(row, 3.0));
        let id = hash2(vec2<f32>(floor(u), row));
        out = mix(mix(base, vec3<f32>(0.9, 0.87, 0.8), 0.6), base * (0.88 + 0.16 * id), joint(u, 0.02) * joint(p.y * 2.0, 0.04));
    } else if (kind < 22.5) {
        // Clapboard: a shadow under each board's lip.
        let f = fract(p.y);
        out = base * (0.95 + 0.06 * hash2(vec2<f32>(floor(p.y), 2.0))) * (1.0 - 0.32 * smoothstep(0.82, 1.0, f));
    } else if (kind < 24.5) {
        // Shingles and scales: staggered rows, a shadow under each.
        let row = floor(p.y);
        let u = p.x / 1.2 + hash2(vec2<f32>(row, 9.0));
        let id = hash2(vec2<f32>(floor(u), row));
        out = base * (0.8 + 0.32 * id) * (1.0 - 0.38 * smoothstep(0.78, 1.0, fract(p.y))) * mix(0.55, 1.0, joint(u, 0.05));
    } else if (kind < 25.5) {
        // Barrel tiles: channels and caps.
        let r = sin(fract(p.x) * 3.14159);
        out = base * (0.84 + 0.2 * hash2(vec2<f32>(floor(p.x), floor(p.y / 1.8)))) * (0.72 + 0.36 * r) * (1.0 - 0.35 * smoothstep(0.85, 1.0, fract(p.y / 1.8)));
    } else if (kind < 26.5) {
        // Standing seam: light seams.
        out = base * (0.95 + 0.08 * hash2(vec2<f32>(floor(p.x), 1.0))) * (1.0 + 0.18 * (1.0 - joint(p.x, 0.06)));
    } else if (kind < 27.5) {
        // Setts.
        let row = floor(p.y * 2.0);
        let odd = fract(row * 0.5) * 2.0;
        let u = p.x + 0.5 * odd;
        out = mix(base * 0.6, base * (0.82 + 0.26 * hash2(vec2<f32>(floor(u), row))), joint(u, 0.05) * joint(p.y * 2.0, 0.1));
    } else if (kind < 28.5) {
        // Asphalt: worn patches.
        out = base * (0.96 + 0.12 * smoothstep(0.35, 0.8, noise(p * 0.25)));
    } else if (kind < 29.5) {
        // Rock in strata: long low slabs.
        let row = floor(p.y);
        let u = p.x * 0.306 + hash2(vec2<f32>(row, 5.0));
        out = mix(base * 0.6, base * (0.86 + 0.14 * hash2(vec2<f32>(floor(u), row))), joint(u, 0.02) * joint(p.y, 0.06));
    } else if (kind < 30.5) {
        // The sea: bands of light drifting.
        let t = g.lamp_count.y;
        let v = noise(p * vec2<f32>(0.22, 0.6) + vec2<f32>(t * 0.05, t * 0.02));
        out = mix(base, base * 1.3 + vec3<f32>(0.12, 0.22, 0.24), smoothstep(0.45, 0.8, v) * 0.75);
    } else if (kind < 32.5) {
        // Setts laid diagonally, or in fans (drawn as courses).
        var q = p;
        if (kind < 31.5) {
            q = vec2<f32>(p.x + p.y, p.y - p.x);
        }
        let row = floor(q.y * 2.0);
        let u = q.x + 0.5 * fract(row * 0.5) * 2.0;
        out = mix(base * 0.6, base * (0.82 + 0.26 * hash2(vec2<f32>(floor(u), row))), joint(u, 0.05) * joint(q.y * 2.0, 0.1));
    } else if (kind < 34.5 || (kind > 35.5 && kind < 37.5) || kind > 38.5) {
        // Cobbles, flagstones, pebbles: stones from smooth noise.
        let v = noise(p * 1.7);
        let edge = smoothstep(0.05, 0.15, abs(fract(v * 3.0) - 0.5));
        out = mix(base * 0.5, base * (0.8 + 0.3 * noise(p * 2.3)), edge);
    } else if (kind < 35.5) {
        // Opus romanum: drawn as squares of two units.
        let id = hash2(floor(p * 0.5));
        out = mix(mix(base, vec3<f32>(0.94, 0.91, 0.84), 0.7), base * (0.86 + 0.2 * id), joint(p.x * 0.5, 0.03) * joint(p.y * 0.5, 0.03));
    } else if (kind < 38.5) {
        // Marble slabs: veins and joints.
        let wv = p.x * 0.8 + p.y * 0.4 + noise(p) * 3.0;
        out = base * (1.0 - 0.3 * (1.0 - smoothstep(0.0, 0.08, abs(fract(wv) - 0.5)))) * mix(0.8, 1.0, joint(p.x, 0.01) * joint(p.y, 0.01));
    }
    return out;
}

