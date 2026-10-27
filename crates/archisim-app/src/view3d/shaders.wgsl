// ArchiSim 3D view.
//
// Pass 1 draws the opaque model, lit by a sun from the south-west and a
// sky light, and also writes each pixel's view-space normal and distance.
// Pass 2 draws the glass, see-through, over it.
// With a section box (Revit's), everything outside it is cut away: the
// faces seen through a cut are the inside of a closed solid, and they are
// painted as the flat cap of the cut, on the box face, in the material's
// colour.
// Pass 3 finds contour lines from those normals and distances (edges where
// the surface turns, silhouettes, and steps in depth) and draws them in ink.

struct Globals {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
    // xyz: direction towards the sun; w: unused.
    sun: vec4<f32>,
    // x: pixel step for contour detection; y: contour strength, negative
    // for thin lines (one pixel wide); z = 1
    // when something is selected (outlined); w = 1 while what is
    // selected is drawn stretched over the scene.
    edges: vec4<f32>,
    // Section box corners; clip_min.w = 1 when the box is on.
    clip_min: vec4<f32>,
    clip_max: vec4<f32>,
    // Eye position (w = 1 in perspective) and viewing direction.
    eye: vec4<f32>,
    forward: vec4<f32>,
    // The cut away walls (view3d/cutaway.rs): x the height everything is
    // cut at, y = 1 when on, z the height of the walls in front, w = 1
    // with walls in front.
    cut: vec4<f32>,
    // Walls in front: xy the horizontal direction towards the camera, z
    // where in front begins along it, w above which walls are upstairs.
    front: vec4<f32>,
    // The sun's view and projection, for the shadows (view3d/look.rs).
    sun_vp: mat4x4<f32>,
    // x = 1 with shadows; y: how far a point is moved off its face before
    // it is looked up in the shadow map; z: samples (1, 5 or 9); w = 1
    // with the shade of inner corners.
    look: vec4<f32>,
    // The line of sight through each pixel (the sky).
    inv_vp: mat4x4<f32>,
    // The light (view3d/look.rs, `Light::colours`): the sun's colour and
    // strength (w), the sky's above and at the horizon (w: strength), the
    // zenith, the haze and the far ground (w: its height).
    sun_color: vec4<f32>,
    sky_hi: vec4<f32>,
    sky_lo: vec4<f32>,
    zenith: vec4<f32>,
    haze: vec4<f32>,
    ground: vec4<f32>,
    // The grid on the ground: centre, reach and step (0: none).
    grid: vec4<f32>,
    // x = 1 with the sky; y: night (0…1); z, w: where the haze begins and
    // where it is thickest.
    atmos: vec4<f32>,
    // Lamps lit at night: position and reach; lamp_count.x of them.
    lamps: array<vec4<f32>, 16>,
    lamp_count: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

// The shadow map of the model's passes (the contour pass has its own
// picture at binding 0 of its group 1).
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var shadow_cmp: sampler_comparison;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    // xyz: the normal (w unused).
    @location(1) normal: vec4<f32>,
    // sRGB colour and opacity.
    @location(2) color: vec4<f32>,
    // x: the bias plus one; y: 1 if selected (2 stretched); z: the
    // texture (0 none, 1 brick…).
    @location(3) misc: vec4<u32>,
    // The size of the texture's piece, metres.
    @location(4) tex_size: f32,
    // The element for the cut away walls: kind (0 plain, 1 wall, 2
    // object), a point of it in plan and its base.
    @location(5) cut: vec4<f32>,
    // Where the object stands (scene.rs, `Instance`): cos, sin of its
    // turn and where it goes in plan; its height and scale; its cut (x
    // below zero: the corner's); flags: selected (below zero: the
    // corner's), greyed.
    @location(6) i_place: vec4<f32>,
    @location(7) i_lift: vec4<f32>,
    @location(8) i_cut: vec4<f32>,
    @location(9) i_flags: vec4<f32>,
};

// A corner where it is, with what the shader needs of it.
struct Corner {
    pos: vec3<f32>,
    normal: vec3<f32>,
    color: vec4<f32>,
    bias: f32,
    tex: vec2<f32>,
    cut: vec4<f32>,
    picked: f32,
};

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn corner(v: VertexIn) -> Corner {
    var c: Corner;
    let cs = v.i_place.x;
    let sn = v.i_place.y;
    let q = v.pos * v.i_lift.yzw;
    c.pos = vec3<f32>(
        cs * q.x - sn * q.y + v.i_place.z,
        sn * q.x + cs * q.y + v.i_place.w,
        q.z + v.i_lift.x,
    );
    let m = v.normal.xyz / v.i_lift.yzw;
    c.normal = normalize(vec3<f32>(cs * m.x - sn * m.y, sn * m.x + cs * m.y, m.z) + vec3<f32>(0.0, 0.0, 1e-9));
    var rgb = srgb_to_linear(v.color.rgb);
    if (v.i_flags.y > 0.5) {
        // Greyed by the disciplines, as a reference (as scene.rs `tone`).
        let lum = 0.3 * rgb.r + 0.59 * rgb.g + 0.11 * rgb.b;
        rgb = vec3<f32>(1.0 - (1.0 - lum) * 0.35);
    }
    c.color = vec4<f32>(rgb, v.color.a);
    c.bias = f32(v.misc.x) - 1.0;
    c.tex = vec2<f32>(f32(v.misc.z), v.tex_size);
    c.cut = select(v.cut, v.i_cut, v.i_cut.x > -0.5);
    c.picked = select(f32(v.misc.y), v.i_flags.x, v.i_flags.x > -0.5);
    return c;
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) view_normal: vec3<f32>,
    @location(3) view_dist: f32,
    @location(4) world: vec3<f32>,
    // Foliage (bias 2): no contour lines between its own facets.
    @location(5) @interpolate(flat) soft: f32,
    @location(6) @interpolate(flat) tex: vec2<f32>,
    @location(7) @interpolate(flat) cut: vec4<f32>,
    @location(8) @interpolate(flat) picked: f32,
};

@vertex
fn vs_model(input: VertexIn) -> VertexOut {
    let v = corner(input);
    var out: VertexOut;
    var vp = g.view * vec4<f32>(v.pos, 1.0);
    // A nudge along the line of sight, 0.02 % of the distance: invisible,
    // but it settles which element wins where two touch on an edge.
    vp.z = vp.z * (1.0 + min(v.bias, 1.0) * 0.0002);
    // 1: foliage; 2: what glows at night (lampshades, bulbs, flames); 3:
    // grass and flowers on the ground; 4: many small leaves (no contours).
    out.soft = select(select(select(select(0.0, 1.0, v.bias > 1.5), 2.0, v.bias > 2.5), 3.0, v.bias > 3.5), 4.0, v.bias > 4.5);
    out.clip = g.proj * vp;
    out.view_normal = (g.view * vec4<f32>(v.normal, 0.0)).xyz;
    out.view_dist = -vp.z;
    out.normal = v.normal;
    out.color = v.color;
    out.world = v.pos;
    out.tex = v.tex;
    out.cut = v.cut;
    out.picked = v.picked;
    return out;
}

fn boxed() -> bool {
    return g.clip_min.w > 0.5;
}

// The height above which an element is cut away by the cut away walls:
// 1e9 if it is not, -1e9 if all of it goes. As `CutRule::top_of`.
fn cut_top(c: vec4<f32>) -> f32 {
    if (g.cut.y < 0.5) {
        return 1e9;
    }
    if (c.x > 1.5) {
        if (c.w > g.cut.x + 1e-3) {
            return -1e9;
        }
        return 1e9;
    }
    if (c.x > 0.5 && g.cut.w > 0.5) {
        if (c.w > g.front.w) {
            return g.cut.x;
        }
        if (dot(g.front.xy, c.yz) > g.front.z) {
            return g.cut.z;
        }
        return 1e9;
    }
    return g.cut.x;
}

// The scene's own copy of what is being stretched (picked 1 while an
// overlay, picked 2, is drawn in its place: edges.w = 1).
fn stretched(picked: f32) -> bool {
    return g.edges.w > 0.5 && picked > 0.5 && picked < 1.5;
}

// What is kept of an element: the section box, lowered to its cut.
struct Keep {
    lo: vec3<f32>,
    hi: vec3<f32>,
    // Something of it is cut.
    on: bool,
};

fn keep(c: vec4<f32>) -> Keep {
    var k: Keep;
    k.lo = vec3<f32>(-1e9);
    k.hi = vec3<f32>(1e9);
    if (boxed()) {
        k.lo = g.clip_min.xyz;
        k.hi = g.clip_max.xyz;
    }
    let top = cut_top(c);
    k.hi.z = min(k.hi.z, top);
    k.on = boxed() || top < 1e8;
    return k;
}

// Outside what is kept (a hair of tolerance keeps faces on it).
fn outside(p: vec3<f32>, k: Keep) -> bool {
    let e = 1e-4;
    return any(p < k.lo - vec3<f32>(e)) || any(p > k.hi + vec3<f32>(e));
}

// Where the line of sight through `p` (inside the box) leaves it towards
// the eye: the cap of the cut. Returns the cap's normal (xyz) and its
// distance along the view (w).
fn cap(p: vec3<f32>, kept: Keep) -> vec4<f32> {
    var d = g.forward.xyz;
    if (g.eye.w > 0.5) {
        d = normalize(p - g.eye.xyz);
    }
    // Back towards the eye.
    let b = -d;
    var t = 1e9;
    var n = vec3<f32>(0.0, 0.0, 1.0);
    for (var k = 0; k < 3; k = k + 1) {
        if (abs(b[k]) > 1e-6) {
            var bound = kept.lo[k];
            var sgn = -1.0;
            if (b[k] > 0.0) {
                bound = kept.hi[k];
                sgn = 1.0;
            }
            let tk = (bound - p[k]) / b[k];
            if (tk < t) {
                t = tk;
                n = vec3<f32>(0.0);
                n[k] = sgn;
            }
        }
    }
    let q = p + b * max(t, 0.0);
    return vec4<f32>(n, dot(q - g.eye.xyz, g.forward.xyz));
}

// An illustration's light: a warm sun (its edge a little crisper than
// plain shading) and a cool sky, brighter from above; `lit` is how much
// of the sun reaches the point (0 in shadow).
fn shade(n_in: vec3<f32>, color: vec3<f32>, lit: f32) -> vec3<f32> {
    let n = normalize(n_in);
    let ndl = dot(n, normalize(g.sun.xyz));
    let direct = (0.65 * max(ndl, 0.0) + 0.35 * smoothstep(0.0, 0.3, ndl)) * lit;
    let up = 0.5 + 0.5 * n.z;
    let sky = mix(g.sky_lo.rgb * g.sky_lo.w, g.sky_hi.rgb * g.sky_hi.w, up) * (0.48 + 0.18 * up);
    let sun = g.sun_color.rgb * g.sun_color.w * direct;
    return color * (sky + sun);
}

// The warm light of the lamps lit at night on a point of a face (no
// shadows). A fixed number of turns, only sums: Windows' compiler unrolls
// it.
fn lamplight(world: vec3<f32>, n_in: vec3<f32>) -> vec3<f32> {
    let n = normalize(n_in);
    var sum = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let l = g.lamps[i];
        let to = l.xyz - world;
        let d = max(length(to), 0.05);
        let fall = clamp(1.0 - d / max(l.w, 0.1), 0.0, 1.0);
        // Only faces turned to the lamp: not the far side of its walls.
        let face = max(dot(n, to / d), 0.0);
        let on = select(0.0, 1.0, f32(i) < g.lamp_count.x);
        sum = sum + on * fall * fall * face;
    }
    return vec3<f32>(1.0, 0.72, 0.42) * 1.25 * sum * g.atmos.y;
}

// The haze between the eye and a point `dist` away.
fn hazed(color: vec3<f32>, dist: f32) -> vec3<f32> {
    if (g.atmos.x < 0.5) {
        return color;
    }
    let f = smoothstep(g.atmos.z, g.atmos.w, dist) * 0.75;
    return mix(color, g.haze.rgb, f);
}

// How much of the sun reaches a point of a face looking along `n`: 1 in
// the sun, 0 in shadow, softened over a few samples. No loops (Windows'
// shader compiler).
fn sunlit(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if (g.look.x < 0.5) {
        return 1.0;
    }
    let q = g.sun_vp * vec4<f32>(world + normalize(n) * g.look.y, 1.0);
    let ndc = q.xyz / q.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let d = ndc.z;
    let px = 1.0 / f32(textureDimensions(shadow_map).x);
    var sum = textureSampleCompareLevel(shadow_map, shadow_cmp, uv, d);
    var count = 1.0;
    if (g.look.z > 1.5) {
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(px, px), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-px, px), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(px, -px), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-px, -px), d);
        count = count + 4.0;
    }
    if (g.look.z > 5.5) {
        let r = 2.0 * px;
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(r, 0.0), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(-r, 0.0), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(0.0, r), d);
        sum = sum + textureSampleCompareLevel(shadow_map, shadow_cmp, uv + vec2<f32>(0.0, -r), d);
        count = count + 4.0;
    }
    // Outside the map (beyond the scene), in the sun.
    let inside = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 && d <= 1.0;
    return select(1.0, sum / count, inside);
}

// The level of detail of the textures' pictures at this pixel (set at the
// start of the fragment; only the pictures' textures use it).
var<private> tex_level: f32;

// ---- Textures -------------------------------------------------------------
//
// Drawn from each point's position on its face (no images, nothing to
// unwrap): on faces looking up, across the plan; on the others, along the
// face and up it. They change the colour a little, around the material's.

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

// Distance to the nearest and second nearest cell centres, and the
// nearest cell's id (stones, gravel).
fn cells(p: vec2<f32>) -> vec3<f32> {
    let i = floor(p);
    var d1 = 9.0;
    var d2 = 9.0;
    var id = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let c = i + vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(hash2(c), hash2(c + 17.0)) * 0.8 + 0.1;
            let d = length(c + o - p);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                id = hash2(c + 3.0);
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return vec3<f32>(d1, d2, id);
}

// How far into a joint a coordinate is (0 in the joint, 1 away from it).
fn joint(t: f32, width: f32) -> f32 {
    let f = fract(t);
    let e = min(f, 1.0 - f);
    return smoothstep(width * 0.5, width, e);
}

fn tex_lod(world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> f32 {
    return 0.0;
}

fn textured(base: vec3<f32>, world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec3<f32> {
    let kind = i32(tex.x + 0.5);
    if (kind == 0) {
        return base;
    }
    let n = abs(normalize(n_in));
    // Along the face (horizontal) and up it; on faces looking up, the plan.
    var uv = world.xy;
    if (n.z < 0.7) {
        if (n.x > n.y) {
            uv = world.yz;
        } else {
            uv = world.xz;
        }
    }
    let s = max(tex.y, 0.01);
    let p = uv / s;
    let mortar = mix(base, vec3<f32>(0.78, 0.76, 0.72), 0.75);
    switch kind {
        // Brick: courses a quarter of the brick's length high, every other
        // one moved half a brick; each brick a little different.
        case 1: {
            let row = floor(p.y * 3.8);
            let q = vec2<f32>(p.x + 0.5 * (row % 2.0), p.y * 3.8);
            let id = hash2(vec2<f32>(floor(q.x), row));
            let body = base * (0.82 + 0.3 * id) * (0.93 + 0.1 * noise(p * 9.0));
            let m = joint(q.x, 0.06) * joint(q.y, 0.2);
            return mix(mortar, body, m);
        }
        // Stone: irregular pieces with dark joints.
        case 2: {
            let c = cells(p * 1.6);
            let edge = smoothstep(0.02, 0.09, c.y - c.x);
            let body = base * (0.75 + 0.45 * c.z) * (0.9 + 0.15 * noise(p * 6.0));
            return mix(base * 0.45, body, edge);
        }
        // Tiles: squares with light joints.
        case 3: {
            let id = hash2(floor(p));
            let body = base * (0.95 + 0.08 * id);
            let m = joint(p.x, 0.03) * joint(p.y, 0.03);
            return mix(mortar, body, m);
        }
        // Wood: grain.
        case 4: {
            let g = sin(p.y * 40.0 + noise(p * vec2<f32>(1.5, 8.0)) * 6.0);
            return base * (0.86 + 0.1 * g + 0.08 * noise(p * vec2<f32>(0.5, 30.0)));
        }
        // Boards: strips a board wide, joined end to end in turns, with
        // their grain.
        case 5: {
            let row = floor(p.y);
            let along = p.x / 8.0 + hash2(vec2<f32>(row, 1.0));
            let id = hash2(vec2<f32>(floor(along), row));
            let g = sin(p.y * 25.0 + noise(vec2<f32>(p.x * 0.6, p.y * 5.0)) * 5.0);
            let body = base * (0.82 + 0.25 * id) * (0.93 + 0.06 * g);
            let m = joint(p.y, 0.06) * joint(along, 0.01);
            return mix(base * 0.4, body, m);
        }
        // Board-marked concrete: panels, their joints and tie holes.
        case 6: {
            let q = vec2<f32>(p.x, p.y * 2.0);
            let m = joint(q.x, 0.008) * joint(q.y, 0.012);
            let f = fract(q);
            let hole = smoothstep(0.012, 0.022, min(length(f - vec2<f32>(0.15, 0.25)), length(f - vec2<f32>(0.85, 0.25))));
            let body = base * (0.94 + 0.1 * noise(p * 12.0) + 0.04 * noise(p * 50.0));
            return body * mix(0.82, 1.0, m) * mix(0.55, 1.0, hole);
        }
        // Render: a fine grain.
        case 7: {
            return base * (0.95 + 0.07 * noise(p * 60.0) + 0.04 * noise(p * 7.0));
        }
        // Grass: blades of light and dark green.
        case 8: {
            let v = noise(p * vec2<f32>(18.0, 40.0)) * 0.6 + noise(p * 4.0) * 0.4;
            return base * (0.7 + 0.55 * v);
        }
        // Gravel: little stones.
        case 9: {
            let c = cells(p * 3.0);
            let body = base * (0.7 + 0.5 * c.z);
            return mix(base * 0.5, body, smoothstep(0.0, 0.25, c.y - c.x));
        }
        // Water: ripples.
        case 10: {
            let w = noise(p * vec2<f32>(3.0, 1.2) + vec2<f32>(noise(p * 2.0) * 2.0, 0.0));
            return base * (0.88 + 0.25 * smoothstep(0.55, 0.8, w));
        }
        // Sheet metal: ribs along the face.
        case 11: {
            let r = 0.5 + 0.5 * sin(p.x * 6.2832);
            return base * (0.8 + 0.3 * r);
        }
        // Linen: threads both ways and slubs along them, faint up close,
        // a soft mottle from afar.
        case 12: {
            let warp = 0.5 + 0.5 * sin(p.x * 150.0);
            let weft = 0.5 + 0.5 * sin(p.y * 150.0);
            let slub = noise(vec2<f32>(p.x * 4.0, p.y * 60.0)) * 0.6 + noise(vec2<f32>(p.x * 60.0, p.y * 4.0)) * 0.4;
            return base * (0.9 + 0.035 * (warp + weft) + 0.08 * slub + 0.04 * noise(p * 3.0));
        }
        // Bouclé: little loops, each catching the light.
        case 13: {
            let c = cells(p * 14.0);
            let loop_ = 1.0 - smoothstep(0.0, 0.45, c.x);
            return base * (0.84 + 0.2 * loop_ + 0.06 * c.z);
        }
        // Knit: rows of V-shaped stitches.
        case 14: {
            let q = vec2<f32>(p.x * 10.0, p.y * 7.0);
            let col = fract(q.x);
            let v = abs(col - 0.5) * 2.0;
            let row = fract(q.y + v * 0.5);
            let stitch = smoothstep(0.0, 0.35, row) * (1.0 - smoothstep(0.65, 1.0, row));
            let gap = smoothstep(0.0, 0.12, min(col, 1.0 - col));
            return base * (0.78 + 0.22 * stitch * gap + 0.05 * noise(p * 2.0));
        }
        // Velvet: lighter where the face turns away from the eye, a soft
        // pile.
        case 15: {
            var d = g.forward.xyz;
            if (g.eye.w > 0.5) {
                d = normalize(world - g.eye.xyz);
            }
            let rim = 1.0 - abs(dot(normalize(n_in), d));
            let pile = noise(p * 12.0) * 0.5 + noise(p * 3.0) * 0.5;
            return base * (0.82 + 0.45 * rim * rim + 0.07 * pile);
        }
        // Wicker: strands over and under in squares.
        case 16: {
            let q = p * 7.0;
            let c = floor(q);
            let f = fract(q);
            let across = (c.x + c.y) % 2.0;
            let t = select(f.x, f.y, across > 0.5);
            let strand = sin(t * 3.1416);
            let gap = smoothstep(0.0, 0.08, min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y)));
            return base * (0.55 + 0.5 * strand * gap + 0.05 * hash2(c));
        }
        // Terrazzo: chips of light, dark and warm stone in the ground.
        case 17: {
            let c = cells(p * 9.0);
            let chip = 1.0 - smoothstep(0.18, 0.24, c.x);
            var tint = vec3<f32>(0.32, 0.3, 0.3);
            if (c.z > 0.66) {
                tint = vec3<f32>(0.75, 0.45, 0.35);
            } else if (c.z > 0.33) {
                tint = vec3<f32>(0.95, 0.93, 0.9);
            }
            let ground = base * (0.96 + 0.05 * noise(p * 20.0));
            return mix(ground, tint, chip * 0.85);
        }
        // Marble: grey veins wandering across a white ground.
        case 18: {
            let w = p.x * 1.3 + p.y * 0.7 + noise(p * 1.5) * 3.0 + noise(p * 5.0) * 0.8;
            let vein = 1.0 - smoothstep(0.0, 0.06, abs(sin(w * 3.1416)));
            let fine = 1.0 - smoothstep(0.0, 0.03, abs(sin((w + noise(p * 9.0)) * 9.0)));
            return base * (0.97 + 0.03 * noise(p * 4.0)) * (1.0 - 0.32 * vein - 0.12 * fine);
        }
        // Soil: a mottle with clods and, here and there, a light stone.
        case 19: {
            let c = cells(p * 5.0);
            let clod = smoothstep(0.0, 0.3, c.y - c.x);
            let v = 0.78 + 0.25 * noise(p * 3.0) + 0.12 * noise(p * 17.0);
            let stone = (1.0 - smoothstep(0.1, 0.16, c.x)) * step(0.85, c.z);
            return mix(base * v * (0.82 + 0.18 * clod), base * 1.3 + vec3<f32>(0.08, 0.08, 0.07), stone);
        }
        // Fallen leaves: one in each cell, ochre, rust or brown, over the
        // darker ground.
        case 20: {
            let c = cells(p * 2.5);
            let leaf = 1.0 - smoothstep(0.26, 0.33, c.x);
            var tint = vec3<f32>(0.5, 0.31, 0.14);
            if (c.z > 0.66) {
                tint = vec3<f32>(0.66, 0.47, 0.18);
            } else if (c.z > 0.33) {
                tint = vec3<f32>(0.4, 0.2, 0.1);
            }
            let ground = base * (0.72 + 0.25 * noise(p * 6.0));
            return mix(ground, tint, leaf * 0.85);
        }
        default: {
            return base;
        }
    }
}

struct ModelOut {
    @location(0) color: vec4<f32>,
    @location(1) normal_dist: vec4<f32>,
};

@fragment
fn fs_model(i: VertexOut, @builtin(front_facing) front: bool) -> ModelOut {
    var out: ModelOut;
    tex_level = tex_lod(i.world, i.normal, i.tex);
    if (stretched(i.picked)) {
        discard;
    }
    let kept = keep(i.cut);
    if (kept.on && outside(i.world, kept)) {
        discard;
    }
    // Selected: the distance is written negative, for the contour pass
    // to outline it.
    let mark = select(1.0, -1.0, i.picked > 0.5);
    if (!front) {
        // The inside of a solid shows only through a cut.
        if (!kept.on) {
            discard;
        }
        let c = cap(i.world, kept);
        out.color = vec4<f32>(shade(c.xyz, i.color.rgb, 1.0) * 0.82, 1.0);
        out.normal_dist = vec4<f32>(normalize((g.view * vec4<f32>(c.xyz, 0.0)).xyz), c.w * mark);
        return out;
    }
    let lit = sunlit(i.world, i.normal);
    let base = textured(i.color.rgb, i.world, i.normal, i.tex);
    var color = shade(i.normal, base, lit) + base * lamplight(i.world, i.normal);
    if (i.soft > 1.5 && i.soft < 2.5) {
        // A lampshade or a bulb at night: it glows.
        let glow = (base * 0.6 + vec3<f32>(0.55, 0.42, 0.25)) * 1.6;
        color = mix(color, glow, g.atmos.y);
    }
    out.color = vec4<f32>(hazed(color, i.view_dist), 1.0);
    out.normal_dist = vec4<f32>(normalize(i.view_normal), i.view_dist * mark);
    if (i.soft > 0.5 && i.soft < 1.5) {
        // Leaves: the same normal everywhere, so the contours come only
        // from the outline of the masses.
        out.normal_dist = vec4<f32>(0.0, 0.0, 1.0, i.view_dist * mark);
    }
    if (i.soft > 3.5) {
        // Many small leaves: marked (a normal straight back), and the
        // contour pass draws no line next to them.
        out.normal_dist = vec4<f32>(0.0, 0.0, -1.0, i.view_dist * mark);
    }
    return out;
}

@fragment
fn fs_glass(i: VertexOut) -> @location(0) vec4<f32> {
    if (stretched(i.picked)) {
        discard;
    }
    let kept = keep(i.cut);
    if (kept.on && outside(i.world, kept)) {
        discard;
    }
    let c = shade(i.normal, i.color.rgb, 1.0) + i.color.rgb * lamplight(i.world, i.normal);
    return vec4<f32>(hazed(c, i.view_dist), i.color.a);
}

// ---- Shadow map -------------------------------------------------------------
//
// The scene's depth seen from the sun. What the cut away walls take away
// casts no shadow.

struct ShadowOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) @interpolate(flat) cut: vec4<f32>,
    @location(2) @interpolate(flat) picked: f32,
};

@vertex
fn vs_shadow(input: VertexIn) -> ShadowOut {
    let v = corner(input);
    var out: ShadowOut;
    out.clip = g.sun_vp * vec4<f32>(v.pos, 1.0);
    out.world = v.pos;
    out.cut = v.cut;
    out.picked = v.picked;
    return out;
}

@fragment
fn fs_shadow(i: ShadowOut) {
    if (stretched(i.picked)) {
        discard;
    }
    let kept = keep(i.cut);
    if (kept.on && outside(i.world, kept)) {
        discard;
    }
}

// ---- Contours -------------------------------------------------------------

@group(1) @binding(0) var normal_dist: texture_2d<f32>;

@vertex
fn vs_fullscreen(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // One triangle covering the whole view.
    let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(i & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn sample_nd(p: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(normal_dist));
    return textureLoad(normal_dist, clamp(p, vec2<i32>(0), size - 1), 0);
}

// Nothing drawn there (the distance of a selected element is negative).
fn empty(a: vec4<f32>) -> bool {
    return abs(a.w) < 1e-4;
}

// A sample of something selected (its distance written negative).
fn picked_at(a: vec4<f32>) -> bool {
    return a.w < 0.0 && !empty(a);
}

// A sample of many small leaves (no contour lines next to them).
fn fine(a: vec4<f32>) -> bool {
    return !empty(a) && a.z < -0.99;
}

// Whether two neighbouring samples belong to different surfaces.
fn crease(a: vec4<f32>, b: vec4<f32>) -> f32 {
    if (fine(a) || fine(b)) {
        return 0.0;
    }
    let a_empty = empty(a);
    let b_empty = empty(b);
    if (a_empty && b_empty) {
        return 0.0;
    }
    if (a_empty != b_empty) {
        return 1.0;
    }
    return select(0.0, 1.0, dot(a.xyz, b.xyz) < 0.94);
}

// Whether the distance steps between three samples in a row: inverse
// distance changes linearly across a flat surface, so a kink in it is a
// step from one surface to another behind it.
fn depth_step(a: vec4<f32>, c: vec4<f32>, b: vec4<f32>) -> f32 {
    if (empty(a) || empty(b) || empty(c)) {
        return 0.0;
    }
    if (fine(a) || fine(b) || fine(c)) {
        return 0.0;
    }
    let ia = 1.0 / abs(a.w);
    let ib = 1.0 / abs(b.w);
    let ic = 1.0 / abs(c.w);
    let kink = abs(ia + ib - 2.0 * ic) / ic;
    return select(0.0, 1.0, kink > 0.03);
}

// A step in depth seen from the nearer of its two surfaces only: the
// line of a thin-line step is drawn on one pixel, not on both sides.
fn near_step(a: vec4<f32>, c: vec4<f32>, b: vec4<f32>) -> f32 {
    if (depth_step(a, c, b) < 0.5) {
        return 0.0;
    }
    let ia = 1.0 / abs(a.w);
    let ib = 1.0 / abs(b.w);
    let ic = 1.0 / abs(c.w);
    return select(0.0, 1.0, 2.0 * ic > ia + ib);
}

// Where the line of sight through a pixel meets the ground (w = 1), or
// its direction (w = 0) when it looks above the horizon.
fn sight(pos: vec2<f32>) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(normal_dist));
    let ndc = vec2<f32>(pos.x / size.x * 2.0 - 1.0, 1.0 - pos.y / size.y * 2.0);
    let near = g.inv_vp * vec4<f32>(ndc, 0.0, 1.0);
    let far = g.inv_vp * vec4<f32>(ndc, 1.0, 1.0);
    let a = near.xyz / near.w;
    let b = far.xyz / far.w;
    let d = normalize(b - a);
    if (d.z < -1e-4) {
        let t = (g.ground.w - a.z) / d.z;
        if (t > 0.0) {
            return vec4<f32>(a + d * t, 1.0);
        }
    }
    return vec4<f32>(d, 0.0);
}

// The sky and the ground fading into the haze, with the grid on the
// ground near what is looked at; `fade` is how much the grid shows.
fn backdrop(at: vec4<f32>, line: f32) -> vec3<f32> {
    if (at.w < 0.5) {
        let up = clamp(at.z, 0.0, 1.0);
        return mix(g.haze.rgb, g.zenith.rgb, pow(up, 0.55));
    }
    let dist = length(at.xyz - g.eye.xyz);
    var c = mix(g.ground.rgb, g.haze.rgb, smoothstep(g.atmos.z * 0.5, g.atmos.w * 1.5, dist));
    c = mix(c, vec3<f32>(0.32, 0.36, 0.44) * (1.0 - 0.8 * g.atmos.y), line);
    return c;
}

@fragment
fn fs_edges(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let s = i32(g.edges.x);
    let c = sample_nd(p);
    // The grid's lines, measured before anything is decided (derivatives
    // only in uniform control flow).
    let at = sight(pos.xy);
    var line = 0.0;
    if (g.grid.w > 0.0) {
        let q = at.xy / g.grid.w;
        let w = max(fwidth(q), vec2<f32>(1e-4));
        let dl = abs(fract(q - 0.5) - 0.5) / w;
        let minor = 1.0 - clamp(min(dl.x, dl.y) - 0.5, 0.0, 1.0);
        let q2 = q * 0.5;
        let dl2 = abs(fract(q2 - 0.5) - 0.5) / max(fwidth(q2), vec2<f32>(1e-4));
        let major = 1.0 - clamp(min(dl2.x, dl2.y) - 0.5, 0.0, 1.0);
        let off = abs(at.xy - g.grid.xy);
        let inside = 1.0 - smoothstep(g.grid.z * 0.8, g.grid.z, max(off.x, off.y));
        line = max(minor * 0.22, major * 0.45) * inside * at.w;
    }
    if (g.atmos.x > 0.5 && empty(c)) {
        return vec4<f32>(backdrop(at, line), 1.0);
    }
    let l = sample_nd(p - vec2<i32>(s, 0));
    let r = sample_nd(p + vec2<i32>(s, 0));
    let u = sample_nd(p - vec2<i32>(0, s));
    let d = sample_nd(p + vec2<i32>(0, s));
    var e = 0.0;
    if (g.edges.y < 0.0) {
        // Thin lines: an edge is marked on one side only (towards the
        // right and down for a crease, on the nearer surface for a step in
        // depth), so it is one pixel wide instead of two.
        e = max(crease(c, r), crease(c, d));
        e = max(e, max(near_step(l, c, r), near_step(u, c, d)));
    } else {
        e = max(max(crease(c, r), crease(c, d)), max(crease(c, l), crease(c, u)));
        e = max(e, max(depth_step(l, c, r), depth_step(u, c, d)));
    }
    // The outline of what is selected, a little thicker than the
    // contours, in its own colour: its faces keep theirs.
    // No loops here: the shader compiler of Windows (FXC) refuses to
    // unroll them.
    if (g.edges.z > 0.5) {
        let inside = picked_at(c);
        let o = s;
        let t = 2 * s;
        var ring = false;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(o, 0))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(o, 0))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(0, o))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(0, o))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(t, 0))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(t, 0))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(0, t))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(0, t))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(o, o))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(o, o))) != inside;
        ring = ring || picked_at(sample_nd(p + vec2<i32>(o, -o))) != inside;
        ring = ring || picked_at(sample_nd(p - vec2<i32>(o, -o))) != inside;
        if (ring) {
            return vec4<f32>(0.98, 0.55, 0.05, 1.0);
        }
    }
    var shade_corner = 0.0;
    if (g.look.w > 0.5) {
        shade_corner = corner_shade(p, c, 3 * s);
    }
    // The ink: darker at night, or it would be lighter than the scene.
    let ink = vec3<f32>(0.05, 0.05, 0.06) * (1.0 - 0.9 * g.atmos.y);
    return vec4<f32>(ink, max(e * abs(g.edges.y), shade_corner));
}

// How far the surface through `c` bends inwards between two samples on
// either side: 0 on a flat face (1 / distance changes linearly across it)
// or a corner turned outwards, more in an inner corner. A big step (one
// thing in front of another) is not a corner.
fn bend(a: vec4<f32>, c: vec4<f32>, b: vec4<f32>) -> f32 {
    if (empty(a) || empty(b) || empty(c)) {
        return 0.0;
    }
    let ic = 1.0 / abs(c.w);
    let k = ((1.0 / abs(a.w) + 1.0 / abs(b.w)) * 0.5 - ic) / ic;
    return clamp(k * 25.0, 0.0, 1.0) * (1.0 - smoothstep(0.06, 0.15, abs(k)));
}

// The shade of inner corners (where a wall meets the floor, under a
// sofa…), from the distances around the point. No loops.
fn corner_shade(p: vec2<i32>, c: vec4<f32>, r: i32) -> f32 {
    var k = bend(sample_nd(p - vec2<i32>(r, 0)), c, sample_nd(p + vec2<i32>(r, 0)));
    k = k + bend(sample_nd(p - vec2<i32>(0, r)), c, sample_nd(p + vec2<i32>(0, r)));
    k = k + bend(sample_nd(p - vec2<i32>(r, r)), c, sample_nd(p + vec2<i32>(r, r)));
    k = k + bend(sample_nd(p - vec2<i32>(r, -r)), c, sample_nd(p + vec2<i32>(r, -r)));
    let t = 2 * r;
    k = k + bend(sample_nd(p - vec2<i32>(t, 0)), c, sample_nd(p + vec2<i32>(t, 0)));
    k = k + bend(sample_nd(p - vec2<i32>(0, t)), c, sample_nd(p + vec2<i32>(0, t)));
    return k / 6.0 * 0.4;
}
