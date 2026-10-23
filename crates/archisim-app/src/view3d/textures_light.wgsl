// ---- Textures (light) ------------------------------------------------------
//
// The same textures drawn more simply, for graphics drivers that hang on
// the full ones (AMD on Intel Macs): no `switch`, no loops, no remainders
// of real numbers; stones and gravel from smooth noise instead of cells.

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
    let n = abs(normalize(n_in));
    var uv = world.xy;
    if (n.z < 0.7) {
        if (n.x > n.y) {
            uv = world.yz;
        } else {
            uv = world.xz;
        }
    }
    let p = uv / max(tex.y, 0.01);
    let mortar = mix(base, vec3<f32>(0.78, 0.76, 0.72), 0.75);
    var out = base;
    if (kind < 1.5) {
        // Brick.
        let row = floor(p.y * 3.8);
        let odd = fract(row * 0.5) * 2.0;
        let q = vec2<f32>(p.x + 0.5 * odd, p.y * 3.8);
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
    }
    return out;
}

